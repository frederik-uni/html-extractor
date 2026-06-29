use async_recursion::async_recursion;
use html_extractor_core::{
    AssignmentOperator, BinaryOperator, BuiltinFunctionKind, Expression, ExpressionKind,
    FunctionDefinition, InterpolatedStringPart, ObjectField, Span, Statement, StatementKind,
    SwitchPattern, UnaryOperator, Visibility, builtin_function,
};

use std::collections::HashMap;

use crate::{
    Diagnostic, ExecutionError, HttpClient, HttpPolicy, HttpRequest, Object, Value,
    engine::HostFunction, functions, scope::Scope,
};

pub(crate) struct Evaluator<'a> {
    warnings: Vec<Diagnostic>,
    host_functions: &'a HashMap<String, HostFunction>,
    http_client: &'a dyn HttpClient,
    http_policy: &'a HttpPolicy,
    script_functions: HashMap<String, FunctionDefinition>,
}

pub(crate) enum Flow<T = Value> {
    Value(T),
    Return(Value),
    Break,
}

impl<T> Flow<T> {
    fn cast<U>(self) -> Flow<U> {
        match self {
            Self::Return(value) => Flow::Return(value),
            Self::Break => Flow::Break,
            Self::Value(_) => unreachable!("value flow cannot change payload type"),
        }
    }
}

pub(crate) type EvalResult<T = Value> = Result<Flow<T>, ExecutionError>;

macro_rules! value_or_flow {
    ($result:expr) => {
        match $result? {
            Flow::Value(value) => value,
            flow => return Ok(flow.cast()),
        }
    };
}

impl<'a> Evaluator<'a> {
    pub(crate) fn new(
        host_functions: &'a HashMap<String, HostFunction>,
        http_client: &'a dyn HttpClient,
        http_policy: &'a HttpPolicy,
        script_functions: &[FunctionDefinition],
    ) -> Self {
        Self {
            warnings: Vec::new(),
            host_functions,
            http_client,
            http_policy,
            script_functions: script_functions
                .iter()
                .map(|function| (function.name().to_owned(), function.clone()))
                .collect(),
        }
    }
    pub(crate) fn into_warnings(self) -> Vec<Diagnostic> {
        self.warnings
    }

    pub(crate) fn push_warning(&mut self, warning: Diagnostic) {
        self.warnings.push(warning);
    }

    pub(crate) async fn statements(
        &mut self,
        statements: &[Statement],
        scope: &mut Scope,
    ) -> Result<(), ExecutionError> {
        match self.block(statements, scope).await? {
            Flow::Value(_) => Ok(()),
            Flow::Return(_) => Err(ExecutionError::new("`return` escaped its function", None)),
            Flow::Break => Err(ExecutionError::new("`break` escaped its loop", None)),
        }
    }

    #[async_recursion(?Send)]
    pub(crate) async fn block(
        &mut self,
        statements: &[Statement],
        scope: &mut Scope,
    ) -> EvalResult {
        let mut last = Value::Null;
        for statement in statements {
            match statement.kind() {
                StatementKind::Assignment {
                    name,
                    operator,
                    visibility,
                    value,
                } => {
                    let value = value_or_flow!(self.expression(value, scope).await);
                    let value = match operator {
                        AssignmentOperator::Assign => value,
                        AssignmentOperator::Add | AssignmentOperator::Subtract => {
                            let current = scope.get(name).ok_or_else(|| {
                                ExecutionError::new(
                                    format!("cannot update unresolved value `{name}`"),
                                    Some(statement.span()),
                                )
                            })?;
                            let operator = match operator {
                                AssignmentOperator::Add => BinaryOperator::Add,
                                AssignmentOperator::Subtract => BinaryOperator::Subtract,
                                AssignmentOperator::Assign => unreachable!(),
                            };
                            binary(current, operator, value, statement.span())?
                        }
                    };
                    scope.assign(name, value, *visibility);
                    last = Value::Null;
                }
                StatementKind::ComputedAssignment { name, value } => {
                    let name_span = name.span();
                    let name = value_or_flow!(self.expression(name, scope).await);
                    let Value::String(name) = name else {
                        return Err(ExecutionError::new(
                            format!(
                                "computed assignment name must be a string, got {}",
                                name.kind()
                            ),
                            Some(name_span),
                        ));
                    };
                    let value = value_or_flow!(self.expression(value, scope).await);
                    let visibility = if name.starts_with('_') {
                        html_extractor_core::Visibility::Private
                    } else {
                        html_extractor_core::Visibility::Public
                    };
                    scope.assign(&name, value, visibility);
                    last = Value::Null;
                }
                StatementKind::Return(value) => {
                    let value = value_or_flow!(self.expression(value, scope).await);
                    return Ok(Flow::Return(value));
                }
                StatementKind::While { condition, body } => {
                    loop {
                        let condition_value =
                            value_or_flow!(self.expression(condition, scope).await);
                        if !boolean(condition_value, condition.span())? {
                            break;
                        }
                        match self.block(body, scope).await? {
                            Flow::Value(_) => {}
                            Flow::Break => break,
                            flow @ Flow::Return(_) => return Ok(flow),
                        }
                    }
                    last = Value::Null;
                }
                StatementKind::Loop { body } => {
                    loop {
                        match self.block(body, scope).await? {
                            Flow::Value(_) => {}
                            Flow::Break => break,
                            flow @ Flow::Return(_) => return Ok(flow),
                        }
                    }
                    last = Value::Null;
                }
                StatementKind::Break => return Ok(Flow::Break),
                StatementKind::Expression(expression) => {
                    last = value_or_flow!(self.expression(expression, scope).await);
                    self.apply_expression_statement_effect(expression, &last, scope);
                }
            }
        }
        Ok(Flow::Value(last))
    }

    #[async_recursion(?Send)]
    pub(crate) async fn expression(
        &mut self,
        expression: &Expression,
        scope: &mut Scope,
    ) -> EvalResult {
        let span = expression.span();
        match expression.kind() {
            ExpressionKind::Null => Ok(Flow::Value(Value::Null)),
            ExpressionKind::Boolean(value) => Ok(Flow::Value(Value::from(*value))),
            ExpressionKind::Number(value) => parse_number(value, span).map(Flow::Value),
            ExpressionKind::String(value) => Ok(Flow::Value(Value::from(value.clone()))),
            ExpressionKind::InterpolatedString(parts) => {
                let mut output = String::new();
                for part in parts {
                    match part {
                        InterpolatedStringPart::Literal(value) => output.push_str(value),
                        InterpolatedStringPart::Expression(expression) => {
                            let value = value_or_flow!(self.expression(expression, scope).await);
                            append_interpolated_value(&mut output, value, expression.span())?;
                        }
                    }
                }
                Ok(Flow::Value(Value::String(output)))
            }
            ExpressionKind::Array(values) => {
                let mut output = Vec::with_capacity(values.len());
                for value in values {
                    output.push(value_or_flow!(self.expression(value, scope).await));
                }
                Ok(Flow::Value(Value::Array(output)))
            }
            ExpressionKind::Object(fields) => self.object(fields, scope).await,
            ExpressionKind::Variable(name) | ExpressionKind::Identifier(name) => {
                scope.get(name).map(Flow::Value).ok_or_else(|| {
                    ExecutionError::new(format!("unresolved value `{name}`"), Some(span))
                })
            }
            ExpressionKind::MethodCall {
                receiver,
                name,
                multiple,
                arguments,
            } => {
                let exact_name = if *multiple {
                    format!("{name}@")
                } else {
                    name.clone()
                };
                let mut normalized = Vec::with_capacity(arguments.len() + 1);
                normalized.push(receiver.as_ref().clone());
                normalized.extend(arguments.iter().cloned());
                self.call_function(&exact_name, &normalized, scope, span)
                    .await
            }
            ExpressionKind::FunctionalBlock {
                receiver,
                method,
                body,
            } => {
                let receiver = value_or_flow!(self.expression(receiver, scope).await);
                if method != "map" {
                    return Err(ExecutionError::new(
                        format!("unsupported functional block `{method}`"),
                        Some(span),
                    ));
                }
                functions::collections::map_block(self, receiver, body, scope, span).await
            }
            ExpressionKind::Required(inner) => {
                let value = value_or_flow!(self.expression(inner, scope).await);
                if let Some(path) = first_missing_path(&value) {
                    let (operation, argument) = required_operation(inner);
                    let argument =
                        argument.map_or(String::new(), |argument| format!(" `{argument}`"));
                    let item = if path.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " at item index {}",
                            path.iter()
                                .map(usize::to_string)
                                .collect::<Vec<_>>()
                                .join(".")
                        )
                    };
                    return Err(ExecutionError::new(
                        format!(
                            "required `{operation}` operation{argument} produced no result{item}"
                        ),
                        Some(span),
                    ));
                }
                Ok(Flow::Value(value))
            }
            ExpressionKind::Unary { operator, operand } => {
                let operand = value_or_flow!(self.expression(operand, scope).await);
                unary(*operator, operand, span).map(Flow::Value)
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                if matches!(operator, BinaryOperator::And | BinaryOperator::Or) {
                    let left = value_or_flow!(self.expression(left, scope).await);
                    let left = boolean(left, span)?;
                    return match (operator, left) {
                        (BinaryOperator::And, false) => Ok(Flow::Value(Value::from(false))),
                        (BinaryOperator::Or, true) => Ok(Flow::Value(Value::from(true))),
                        (BinaryOperator::And, true) | (BinaryOperator::Or, false) => {
                            let right = value_or_flow!(self.expression(right, scope).await);
                            Ok(Flow::Value(Value::from(boolean(right, span)?)))
                        }
                        _ => unreachable!(),
                    };
                }
                let left = value_or_flow!(self.expression(left, scope).await);
                let right = value_or_flow!(self.expression(right, scope).await);
                binary(left, *operator, right, span).map(Flow::Value)
            }
            ExpressionKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = value_or_flow!(self.expression(condition, scope).await);
                match condition {
                    Value::Boolean(true) => self.expression(then_branch, scope).await,
                    Value::Boolean(false) => self.expression(else_branch, scope).await,
                    other => Err(ExecutionError::new(
                        format!("expected boolean condition, got {}", other.kind()),
                        Some(span),
                    )),
                }
            }
            ExpressionKind::BlockIf {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = value_or_flow!(self.expression(condition, scope).await);
                match condition {
                    Value::Boolean(true) => self.block(then_branch, scope).await,
                    Value::Boolean(false) => match else_branch {
                        Some(else_branch) => self.block(else_branch, scope).await,
                        None => Ok(Flow::Value(Value::Null)),
                    },
                    other => Err(ExecutionError::new(
                        format!("expected boolean condition, got {}", other.kind()),
                        Some(span),
                    )),
                }
            }
            ExpressionKind::Switch { target, arms } => {
                let target = value_or_flow!(self.expression(target, scope).await);
                for arm in arms {
                    match arm.pattern() {
                        SwitchPattern::Value(pattern) => {
                            let pattern = value_or_flow!(self.expression(pattern, scope).await);
                            if pattern == target {
                                return self.expression(arm.value(), scope).await;
                            }
                        }
                        SwitchPattern::Wildcard => {
                            return self.expression(arm.value(), scope).await;
                        }
                        SwitchPattern::Binding(name) => {
                            let mut child = scope.child();
                            child.assign(name, target, html_extractor_core::Visibility::Private);
                            return self.expression(arm.value(), &mut child).await;
                        }
                    }
                }
                Err(ExecutionError::new(
                    "switch expression did not match and has no fallback",
                    Some(span),
                ))
            }
            ExpressionKind::FunctionCall { name, arguments } => {
                self.call_function(name, arguments, scope, span).await
            }
            ExpressionKind::QualifiedFunctionCall {
                module,
                name,
                arguments,
            } => {
                let arguments = value_or_flow!(self.arguments(arguments, scope).await);
                self.call_host(&format!("{module}.{name}"), arguments, span)
                    .await
                    .map(Flow::Value)
            }
            ExpressionKind::CoreCall { name, arguments } => {
                let arguments = value_or_flow!(self.arguments(arguments, scope).await);
                self.call_core(name, arguments, scope, span)
                    .await
                    .map(Flow::Value)
            }
            ExpressionKind::Lambda { .. } => Err(ExecutionError::new(
                "lambda cannot be used as a value",
                Some(span),
            )),
            ExpressionKind::Subscript { receiver, index } => {
                let rec_val = value_or_flow!(self.expression(receiver, scope).await);
                let idx_val = value_or_flow!(self.expression(index, scope).await);
                match idx_val {
                    Value::Number(num) => {
                        let idx_usize = num.as_u64().and_then(|v| usize::try_from(v).ok());
                        if let Some(i) = idx_usize {
                            match rec_val {
                                Value::Array(arr) => {
                                    if i < arr.len() {
                                        Ok(Flow::Value(arr[i].clone()))
                                    } else {
                                        Ok(Flow::Value(Value::Null))
                                    }
                                }
                                other => Err(ExecutionError::new(
                                    format!(
                                        "cannot index {} with integer index {}",
                                        other.kind(),
                                        i
                                    ),
                                    Some(span),
                                )),
                            }
                        } else {
                            Err(ExecutionError::new(
                                format!(
                                    "subscript index must be a valid non-negative integer, got number {num}"
                                ),
                                Some(span),
                            ))
                        }
                    }
                    Value::String(key) => match rec_val {
                        Value::Object(obj) => {
                            Ok(Flow::Value(obj.get(&key).cloned().unwrap_or(Value::Null)))
                        }
                        other => Err(ExecutionError::new(
                            format!("cannot index {} with string key `{}`", other.kind(), key),
                            Some(span),
                        )),
                    },
                    other => Err(ExecutionError::new(
                        format!(
                            "subscript index must be a number or a string, got {}",
                            other.kind()
                        ),
                        Some(span),
                    )),
                }
            }
        }
    }

    async fn call_host(
        &self,
        name: &str,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, ExecutionError> {
        let function = self.host_functions.get(name).ok_or_else(|| {
            ExecutionError::new(format!("unlinked function `{name}`"), Some(span))
        })?;
        function(arguments).await.map_err(|error| {
            if error.span().is_some() {
                error
            } else {
                ExecutionError::new(error.message(), Some(span))
            }
        })
    }

    async fn call_core(
        &self,
        name: &str,
        arguments: Vec<Value>,
        scope: &mut Scope,
        span: Span,
    ) -> Result<Value, ExecutionError> {
        match name {
            "extend" => {
                if arguments.len() == 2 && matches!(arguments.first(), Some(Value::Array(_))) {
                    let mut arguments = arguments.into_iter();
                    let receiver = arguments.next().expect("length checked");
                    functions::call("extend", receiver, arguments.collect(), span)
                } else {
                    extend_context(arguments, scope, span)
                }
            }
            "fetch" => {
                let request = fetch_request(arguments, span)?;
                self.http_policy
                    .validate_url(request.url())
                    .map_err(|error| ExecutionError::new(error.message(), Some(span)))?;
                let response = self
                    .http_client
                    .fetch(&request, self.http_policy)
                    .await
                    .map_err(|error| ExecutionError::new(error.message(), Some(span)))?;
                self.http_policy
                    .validate_response(&response)
                    .map_err(|error| ExecutionError::new(error.message(), Some(span)))?;
                Ok(Value::Response(response))
            }
            _ => Err(ExecutionError::new(
                format!("unknown core function `{name}`"),
                Some(span),
            )),
        }
    }

    async fn call_function(
        &mut self,
        name: &str,
        arguments: &[Expression],
        scope: &mut Scope,
        span: Span,
    ) -> EvalResult {
        match builtin_function(name).map(|function| function.kind()) {
            Some(BuiltinFunctionKind::Lambda) => {
                let Some((receiver, arguments)) = arguments.split_first() else {
                    return Err(ExecutionError::new(
                        format!("`{name}` expects a target value as the first argument"),
                        Some(span),
                    ));
                };
                let receiver = value_or_flow!(self.expression(receiver, scope).await);
                match name {
                    "map" => {
                        functions::collections::map(self, receiver, arguments, scope, span).await
                    }
                    "filter" => {
                        functions::collections::filter(self, receiver, arguments, scope, span).await
                    }
                    "any" | "all" | "find" => {
                        functions::collections::predicate(
                            self, receiver, arguments, scope, span, name,
                        )
                        .await
                    }
                    "sort_by_key" => {
                        functions::sorting::sort_by_key(self, receiver, arguments, scope, span)
                            .await
                    }
                    "sort_by" => {
                        functions::sorting::sort_by(self, receiver, arguments, scope, span).await
                    }
                    "assert" => {
                        functions::guards::predicate(self, receiver, arguments, scope, span, false)
                            .await
                    }
                    "warn" => {
                        functions::guards::predicate(self, receiver, arguments, scope, span, true)
                            .await
                    }
                    _ => unreachable!(),
                }
            }
            Some(BuiltinFunctionKind::Value) => {
                let Some((receiver, arguments)) = arguments.split_first() else {
                    return Err(ExecutionError::new(
                        format!("`{name}` expects a target value as the first argument"),
                        Some(span),
                    ));
                };
                let receiver = value_or_flow!(self.expression(receiver, scope).await);
                let arguments = value_or_flow!(self.arguments(arguments, scope).await);
                functions::call(name, receiver, arguments, span).map(Flow::Value)
            }
            Some(BuiltinFunctionKind::Core) => {
                let arguments = value_or_flow!(self.arguments(arguments, scope).await);
                self.call_core(name, arguments, scope, span)
                    .await
                    .map(Flow::Value)
            }
            None => {
                let arguments = value_or_flow!(self.arguments(arguments, scope).await);
                if name == "assertEq" {
                    return assert_eq_call(arguments, span).map(Flow::Value);
                }
                if let Some(function) = self.script_functions.get(name).cloned() {
                    if arguments.len() != function.parameters().len() {
                        let expected = function.parameters().len();
                        let plural = if expected == 1 { "" } else { "s" };
                        return Err(ExecutionError::new(
                            format!(
                                "function `{name}` expects {expected} argument{plural}, got {}",
                                arguments.len()
                            ),
                            Some(span),
                        ));
                    }
                    let mut child = scope.function();
                    for (parameter, value) in function.parameters().iter().zip(arguments) {
                        child.assign(parameter, value, html_extractor_core::Visibility::Private);
                    }
                    return match self.block(function.body(), &mut child).await? {
                        Flow::Value(value) | Flow::Return(value) => Ok(Flow::Value(value)),
                        Flow::Break => {
                            Err(ExecutionError::new("`break` escaped its loop", Some(span)))
                        }
                    };
                }
                self.call_host(name, arguments, span).await.map(Flow::Value)
            }
        }
    }

    fn apply_expression_statement_effect(
        &self,
        expression: &Expression,
        value: &Value,
        scope: &mut Scope,
    ) {
        let ExpressionKind::MethodCall { receiver, name, .. } = expression.kind() else {
            return;
        };
        if name != "extend" {
            return;
        }
        let ExpressionKind::Variable(binding) = receiver.kind() else {
            return;
        };
        let visibility = if binding.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };
        scope.assign(binding, value.clone(), visibility);
    }

    async fn object(&mut self, fields: &[ObjectField], scope: &mut Scope) -> EvalResult {
        let mut object = Object::new();
        for field in fields {
            object.insert(
                field.name(),
                value_or_flow!(self.expression(field.value(), scope).await),
            );
        }
        Ok(Flow::Value(Value::Object(object)))
    }

    async fn arguments(
        &mut self,
        arguments: &[Expression],
        scope: &mut Scope,
    ) -> EvalResult<Vec<Value>> {
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(value_or_flow!(self.expression(argument, scope).await));
        }
        Ok(Flow::Value(values))
    }
}

fn append_interpolated_value(
    output: &mut String,
    value: Value,
    span: Span,
) -> Result<(), ExecutionError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Boolean(value) => output.push_str(if value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => output.push_str(&value),
        other => {
            return Err(ExecutionError::new(
                format!("cannot interpolate {}", other.kind()),
                Some(span),
            ));
        }
    }
    Ok(())
}

fn assert_eq_call(arguments: Vec<Value>, span: Span) -> Result<Value, ExecutionError> {
    let (actual, expected, message) = match arguments.as_slice() {
        [actual, expected] => (actual, expected, None),
        [actual, expected, Value::String(message)] => (actual, expected, Some(message.as_str())),
        [_, _, other] => {
            return Err(ExecutionError::new(
                format!("`assertEq` message must be a string, got {}", other.kind()),
                Some(span),
            ));
        }
        _ => {
            return Err(ExecutionError::new(
                "`assertEq` expects two values and an optional message",
                Some(span),
            ));
        }
    };
    if actual == expected {
        return Ok(Value::Null);
    }
    let prefix = message.map_or_else(String::new, |message| format!("{message}: "));
    Err(ExecutionError::new(
        format!("{prefix}assertEq expected {expected:?}, got {actual:?}"),
        Some(span),
    ))
}

fn extend_context(
    arguments: Vec<Value>,
    scope: &mut Scope,
    span: Span,
) -> Result<Value, ExecutionError> {
    let object = match arguments.as_slice() {
        [Value::Object(object)] => object.clone(),
        [_] => {
            return Err(ExecutionError::new(
                "`extend` expects an object",
                Some(span),
            ));
        }
        _ => {
            return Err(ExecutionError::new(
                "`extend` expects exactly one object argument",
                Some(span),
            ));
        }
    };
    scope.extend(object.clone());
    Ok(Value::Object(object))
}

fn fetch_request(arguments: Vec<Value>, span: Span) -> Result<HttpRequest, ExecutionError> {
    match arguments.as_slice() {
        [Value::String(url)] => Ok(HttpRequest::get(url, None)),
        [Value::String(url), Value::Object(options)] => {
            if let Some(name) = options.keys().find(|name| {
                !matches!(
                    *name,
                    "method" | "json" | "headers" | "cloudflare" | "urlencoded"
                )
            }) {
                //TODO: cloudflare
                return Err(ExecutionError::new(
                    format!("unknown `fetch` option `{name}`"),
                    Some(span),
                ));
            }

            let method = match options.get("method") {
                Some(Value::String(method)) => method.as_str(),
                Some(_) => {
                    return Err(ExecutionError::new(
                        "`fetch` option `method` must be a string",
                        Some(span),
                    ));
                }
                None => "GET",
            };

            let headers = match options.get("headers") {
                Some(Value::Object(headers)) => Some(headers),
                Some(_) => {
                    return Err(ExecutionError::new(
                        "`fetch` option `headers` must be an object",
                        Some(span),
                    ));
                }
                None => None,
            };

            let headers = headers
                .map(|headers| {
                    headers
                        .iter()
                        .map(|(name, value)| {
                            let Value::String(value) = value else {
                                return Err(ExecutionError::new(
                                    format!("`fetch` header `{name}` must be a string"),
                                    Some(span),
                                ));
                            };

                            Ok((name.clone(), value.clone()))
                        })
                        .collect::<Result<Vec<_>, ExecutionError>>()
                })
                .transpose()?;

            let cloudflare = match options.get("cloudflare") {
                Some(Value::Boolean(cloudflare)) => *cloudflare,
                Some(_) => {
                    return Err(ExecutionError::new(
                        "`fetch` option `cloudflare` must be a boolean",
                        Some(span),
                    ));
                }
                None => false,
            };

            let (body, content_type) = match (options.get("json"), options.get("urlencoded")) {
                (Some(_), Some(_)) => {
                    return Err(ExecutionError::new(
                        "`fetch` options cannot contain both `json` and `urlencoded`",
                        Some(span),
                    ));
                }

                (Some(json), None) => {
                    let json = json.to_json().map_err(|error| {
                        ExecutionError::new(
                            format!("invalid `fetch` JSON body: {error}"),
                            Some(span),
                        )
                    })?;

                    let body = serde_json::to_vec(&json).map_err(|error| {
                        ExecutionError::new(
                            format!("failed to encode `fetch` JSON body: {error}"),
                            Some(span),
                        )
                    })?;

                    (Some(body), Some("application/json".to_string()))
                }

                (None, Some(urlencoded)) => {
                    let Value::Object(fields) = urlencoded else {
                        return Err(ExecutionError::new(
                            "`fetch` option `urlencoded` must be an object",
                            Some(span),
                        ));
                    };

                    let fields = fields
                        .iter()
                        .map(|(name, value)| {
                            let Value::String(value) = value else {
                                return Err(ExecutionError::new(
                                    format!("`fetch` urlencoded field `{name}` must be a string"),
                                    Some(span),
                                ));
                            };

                            Ok((name.clone(), value.clone()))
                        })
                        .collect::<Result<Vec<_>, ExecutionError>>()?;

                    let body = serde_urlencoded::to_string(fields).map_err(|error| {
                        ExecutionError::new(
                            format!("failed to encode `fetch` urlencoded body: {error}"),
                            Some(span),
                        )
                    })?;

                    (
                        Some(body.into_bytes()),
                        Some("application/x-www-form-urlencoded".to_string()),
                    )
                }

                (None, None) => (None, None),
            };

            let request = match method.to_ascii_uppercase().as_str() {
                "GET" => {
                    if body.is_some() {
                        return Err(ExecutionError::new(
                            "`fetch` GET options must not include `json`",
                            Some(span),
                        ));
                    }

                    HttpRequest::get(url, headers).with_cloudflare(cloudflare)
                }

                "POST" => {
                    HttpRequest::post(url, body, headers, content_type).with_cloudflare(cloudflare)
                }

                _ => {
                    return Err(ExecutionError::new(
                        format!("unsupported `fetch` method `{method}`"),
                        Some(span),
                    ));
                }
            };

            Ok(request)
        }
        [Value::String(_), _] => Err(ExecutionError::new(
            "`fetch` options must be an object",
            Some(span),
        )),
        _ => Err(ExecutionError::new(
            "`fetch` expects a URL string and optional options object",
            Some(span),
        )),
    }
}
fn parse_number(value: &str, span: Span) -> Result<Value, ExecutionError> {
    value
        .parse::<serde_json::Number>()
        .map(Value::Number)
        .map_err(|error| ExecutionError::new(format!("invalid number: {error}"), Some(span)))
}

fn unary(operator: UnaryOperator, operand: Value, span: Span) -> Result<Value, ExecutionError> {
    match operator {
        UnaryOperator::Not => {
            Ok(Value::Boolean(!operand.expect_boolean().map_err(
                |error| ExecutionError::new(error.to_string(), Some(span)),
            )?))
        }
        UnaryOperator::Negate => {
            let number = operand
                .expect_number()
                .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
            if let Some(value) = number.as_i64() {
                Ok(Value::from(-value))
            } else {
                float_value(-number.as_f64().unwrap(), span)
            }
        }
    }
}

fn binary(
    left: Value,
    operator: BinaryOperator,
    right: Value,
    span: Span,
) -> Result<Value, ExecutionError> {
    match operator {
        BinaryOperator::Or | BinaryOperator::And => {
            let left = boolean(left, span)?;
            let right = boolean(right, span)?;
            Ok(Value::Boolean(match operator {
                BinaryOperator::Or => left || right,
                BinaryOperator::And => left && right,
                _ => unreachable!(),
            }))
        }
        BinaryOperator::Equal => Ok(Value::Boolean(left == right)),
        BinaryOperator::NotEqual => Ok(Value::Boolean(left != right)),
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            let left = number(&left, span)?;
            let right = number(&right, span)?;
            Ok(Value::Boolean(match operator {
                BinaryOperator::Less => left < right,
                BinaryOperator::LessEqual => left <= right,
                BinaryOperator::Greater => left > right,
                BinaryOperator::GreaterEqual => left >= right,
                _ => unreachable!(),
            }))
        }
        _ => arithmetic(left, operator, right, span),
    }
}

fn boolean(value: Value, span: Span) -> Result<bool, ExecutionError> {
    value
        .expect_boolean()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))
}

fn arithmetic(
    left: Value,
    operator: BinaryOperator,
    right: Value,
    span: Span,
) -> Result<Value, ExecutionError> {
    if let (Some(left), Some(right)) = (
        left.expect_number()
            .ok()
            .and_then(serde_json::Number::as_i64),
        right
            .expect_number()
            .ok()
            .and_then(serde_json::Number::as_i64),
    ) {
        let value = match operator {
            BinaryOperator::Add => left.checked_add(right),
            BinaryOperator::Subtract => left.checked_sub(right),
            BinaryOperator::Multiply => left.checked_mul(right),
            BinaryOperator::Divide => left.checked_div(right),
            BinaryOperator::Remainder => left.checked_rem(right),
            _ => None,
        };
        return value
            .map(Value::from)
            .ok_or_else(|| ExecutionError::new("invalid integer arithmetic", Some(span)));
    }
    let left = number(&left, span)?;
    let right = number(&right, span)?;
    let value = match operator {
        BinaryOperator::Add => left + right,
        BinaryOperator::Subtract => left - right,
        BinaryOperator::Multiply => left * right,
        BinaryOperator::Divide => left / right,
        BinaryOperator::Remainder => left % right,
        _ => unreachable!(),
    };
    float_value(value, span)
}

fn number(value: &Value, span: Span) -> Result<f64, ExecutionError> {
    value
        .expect_number()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
        .as_f64()
        .ok_or_else(|| ExecutionError::new("number cannot be represented", Some(span)))
}

fn float_value(value: f64, span: Span) -> Result<Value, ExecutionError> {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| ExecutionError::new("non-finite numeric result", Some(span)))
}

fn first_missing_path(value: &Value) -> Option<Vec<usize>> {
    fn visit(value: &Value, path: &mut Vec<usize>) -> bool {
        match value {
            Value::Null => true,
            Value::Array(values) if values.is_empty() => true,
            Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    path.push(index);
                    if visit(value, path) {
                        return true;
                    }
                    path.pop();
                }
                false
            }
            _ => false,
        }
    }

    let mut path = Vec::new();
    visit(value, &mut path).then_some(path)
}

fn required_operation(expression: &Expression) -> (String, Option<String>) {
    let ExpressionKind::MethodCall {
        name,
        multiple,
        arguments,
        ..
    } = expression.kind()
    else {
        return ("unknown".to_owned(), None);
    };
    let name = if *multiple {
        format!("{name}@")
    } else {
        name.clone()
    };
    let argument = arguments
        .first()
        .and_then(|argument| match argument.kind() {
            ExpressionKind::String(value) => Some(value.clone()),
            _ => None,
        });
    (name, argument)
}
