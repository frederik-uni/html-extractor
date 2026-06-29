use std::{cell::RefCell, collections::HashSet, rc::Rc};

use crate::{
    BinaryOperator, BuiltinFunctionKind, CompileError, Diagnostic, Expression, ExpressionKind,
    FunctionIdentity, InputConstraint, InterpolatedStringPart, Program, Requirements, Span,
    Statement, StatementKind, SwitchPattern, builtin_function,
};

pub(crate) fn validate(program: &mut Program) -> Result<Requirements, CompileError> {
    let mut script_functions = HashSet::new();
    let mut declaration_diagnostics = Vec::new();
    for function in program.functions() {
        if builtin_function(function.name()).is_some() {
            declaration_diagnostics.push(Diagnostic::new(
                format!("function name `{}` is reserved", function.name()),
                function.span(),
            ));
        }
        if !script_functions.insert(function.name().to_owned()) {
            declaration_diagnostics.push(Diagnostic::new(
                format!("duplicate function `{}`", function.name()),
                function.span(),
            ));
        }
        let mut parameters = HashSet::new();
        for parameter in function.parameters() {
            if !parameters.insert(parameter) {
                declaration_diagnostics.push(Diagnostic::new(
                    format!("duplicate parameter `{parameter}`"),
                    function.span(),
                ));
            }
        }
    }

    let declared_globals: HashSet<_> = program
        .statements()
        .iter()
        .filter_map(|statement| match statement.kind() {
            StatementKind::Assignment { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    resolve_qualified_calls(program.statements_mut(), &mut HashSet::new());
    for test in program.tests_mut() {
        let mut bindings = declared_globals.clone();
        bindings.extend(test.inputs().iter().map(|input| input.name().to_owned()));
        if test.runs_script() {
            bindings.insert("it".to_owned());
        }
        resolve_qualified_calls(test.body_mut(), &mut bindings);
    }
    for function in program.functions_mut() {
        let mut bindings = declared_globals.clone();
        bindings.extend(function.parameters().iter().cloned());
        resolve_qualified_calls(function.body_mut(), &mut bindings);
    }

    let mut validator = Validator {
        script_functions,
        diagnostics: declaration_diagnostics,
        ..Validator::default()
    };
    for function in program.functions() {
        let mut scope = Scope::function(&declared_globals, function.parameters());
        validator.statements(
            function.body(),
            &mut scope,
            ValidationContext {
                in_function: true,
                ..ValidationContext::default()
            },
            true,
        );
    }
    validator.statements(
        program.statements(),
        &mut Scope::new(),
        ValidationContext::default(),
        false,
    );
    for test in program.tests() {
        let mut scope = Scope::new();
        for input in test.inputs() {
            validator.expression(
                input.value(),
                &mut scope,
                ValidationContext {
                    in_test: true,
                    ..ValidationContext::default()
                },
                ExpressionUse::Value,
            );
            scope.bindings.insert(input.name().to_owned());
        }
        if test.runs_script() {
            scope.locals.insert("it".to_owned());
        }
        validator.statements(
            test.body(),
            &mut scope,
            ValidationContext {
                in_test: true,
                ..ValidationContext::default()
            },
            false,
        );
    }
    if validator.diagnostics.is_empty() {
        Ok(validator.requirements)
    } else {
        Err(CompileError::from_diagnostics(validator.diagnostics))
    }
}

struct Scope {
    bindings: HashSet<String>,
    locals: HashSet<String>,
    globals: Rc<RefCell<HashSet<String>>>,
    it_available: bool,
}

impl Scope {
    fn new() -> Self {
        Self {
            bindings: HashSet::new(),
            locals: HashSet::new(),
            globals: Rc::new(RefCell::new(HashSet::new())),
            it_available: false,
        }
    }

    fn function(globals: &HashSet<String>, parameters: &[String]) -> Self {
        Self {
            bindings: globals.clone(),
            locals: parameters.iter().cloned().collect(),
            globals: Rc::new(RefCell::new(globals.clone())),
            it_available: false,
        }
    }

    fn child(&self, it_available: bool) -> Self {
        Self {
            bindings: self.bindings.clone(),
            locals: self.locals.clone(),
            globals: Rc::clone(&self.globals),
            it_available,
        }
    }

    fn contains_binding(&self, name: &str) -> bool {
        self.bindings.contains(name) || self.globals.borrow().contains(name)
    }

    fn bind_global(&mut self, name: String) {
        self.bindings.insert(name.clone());
        self.globals.borrow_mut().insert(name);
    }
}

#[derive(Default)]
struct Validator {
    requirements: Requirements,
    diagnostics: Vec<Diagnostic>,
    script_functions: HashSet<String>,
}

#[derive(Clone, Copy, Default)]
struct ValidationContext {
    in_function: bool,
    in_test: bool,
    loop_depth: usize,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ExpressionUse {
    Statement,
    Value,
}

impl Validator {
    fn statements(
        &mut self,
        statements: &[Statement],
        scope: &mut Scope,
        context: ValidationContext,
        value_required: bool,
    ) {
        for (index, statement) in statements.iter().enumerate() {
            let expression_use = if value_required && index + 1 == statements.len() {
                ExpressionUse::Value
            } else {
                ExpressionUse::Statement
            };
            match statement.kind() {
                StatementKind::Assignment { name, value, .. } => {
                    self.expression(value, scope, context, ExpressionUse::Value);
                    scope.bindings.insert(name.clone());
                }
                StatementKind::ComputedAssignment { name, value } => {
                    self.expression(name, scope, context, ExpressionUse::Value);
                    self.expression(value, scope, context, ExpressionUse::Value);
                }
                StatementKind::Return(value) => {
                    if !context.in_function {
                        self.error("`return` is only valid inside a function", statement.span());
                    }
                    self.expression(value, scope, context, ExpressionUse::Value);
                }
                StatementKind::While { condition, body } => {
                    self.expression(condition, scope, context, ExpressionUse::Value);
                    self.statements(
                        body,
                        scope,
                        ValidationContext {
                            loop_depth: context.loop_depth + 1,
                            ..context
                        },
                        false,
                    );
                }
                StatementKind::Loop { body } => self.statements(
                    body,
                    scope,
                    ValidationContext {
                        loop_depth: context.loop_depth + 1,
                        ..context
                    },
                    false,
                ),
                StatementKind::Break => {
                    if context.loop_depth == 0 {
                        self.error("`break` is only valid inside a loop", statement.span());
                    }
                }
                StatementKind::Expression(expression) => {
                    self.expression(expression, scope, context, expression_use);
                }
            }
        }
    }

    fn expression(
        &mut self,
        expression: &Expression,
        scope: &mut Scope,
        context: ValidationContext,
        usage: ExpressionUse,
    ) {
        match expression.kind() {
            ExpressionKind::Null
            | ExpressionKind::Boolean(_)
            | ExpressionKind::Number(_)
            | ExpressionKind::String(_) => {}
            ExpressionKind::InterpolatedString(parts) => {
                for part in parts {
                    if let InterpolatedStringPart::Expression(expression) = part {
                        self.expression(expression, scope, context, ExpressionUse::Value);
                    }
                }
            }
            ExpressionKind::Array(values) => {
                for value in values {
                    self.expression(value, scope, context, ExpressionUse::Value);
                }
            }
            ExpressionKind::Object(fields) => {
                for field in fields {
                    self.expression(field.value(), scope, context, ExpressionUse::Value);
                }
            }
            ExpressionKind::Variable(name) => {
                if scope.locals.contains(name) {
                    // Lambda parameters are local variable bindings.
                } else if name == "it" {
                    if !scope.it_available {
                        self.error(
                            "`it` is only available inside functional blocks",
                            expression.span(),
                        );
                    }
                } else if !scope.contains_binding(name) {
                    self.requirements.require_input(name, expression.span());
                }
            }
            ExpressionKind::Identifier(name) => {
                if !scope.locals.contains(name) {
                    self.error(
                        format!("unresolved local identifier `{name}`"),
                        expression.span(),
                    );
                }
            }
            ExpressionKind::FunctionCall { name, arguments } => {
                self.expressions(arguments, scope, context);
                if context.in_test && name == "assertEq" {
                    return;
                }
                if builtin_function(name).is_none() && !self.script_functions.contains(name) {
                    self.requirements.require_function(
                        FunctionIdentity::Unqualified(name.clone()),
                        arguments.len(),
                        expression.span(),
                    );
                }
            }
            ExpressionKind::QualifiedFunctionCall {
                module,
                name,
                arguments,
            } => {
                self.requirements.require_function(
                    FunctionIdentity::Qualified {
                        module: module.clone(),
                        name: name.clone(),
                    },
                    arguments.len(),
                    expression.span(),
                );
                self.expressions(arguments, scope, context);
            }
            ExpressionKind::CoreCall { name, arguments } => {
                self.expressions(arguments, scope, context);
                if name == "extend"
                    && let [argument] = arguments.as_slice()
                    && let ExpressionKind::Object(fields) = argument.kind()
                {
                    for field in fields {
                        scope.bind_global(field.name().to_owned());
                    }
                }
            }
            ExpressionKind::MethodCall {
                receiver,
                name,
                arguments,
                ..
            } => {
                self.expression(receiver, scope, context, ExpressionUse::Value);
                self.expressions(arguments, scope, context);
                self.validate_method(receiver, name, expression.span());
                if name == "css" {
                    self.constrain(receiver, InputConstraint::HtmlLike, expression.span());
                }
            }
            ExpressionKind::Lambda { parameters, body } => {
                let mut child = scope.child(scope.it_available);
                child.locals.extend(parameters.iter().cloned());
                self.expression(body, &mut child, context, ExpressionUse::Value);
            }
            ExpressionKind::FunctionalBlock { receiver, body, .. } => {
                self.expression(receiver, scope, context, ExpressionUse::Value);
                let mut child = scope.child(true);
                self.statements(body, &mut child, context, false);
            }
            ExpressionKind::Required(inner) => {
                self.expression(inner, scope, context, ExpressionUse::Value);
                if !matches!(
                    inner.kind(),
                    ExpressionKind::MethodCall { name, .. } if matches!(name.as_str(), "css" | "regex")
                ) && !matches!(
                    inner.kind(),
                    ExpressionKind::FunctionCall { name, .. } if matches!(name.as_str(), "css" | "css@" | "regex" | "regex@")
                ) {
                    self.error(
                        "`!` is only valid after `css`, `css@`, or `regex`",
                        expression.span(),
                    );
                }
            }
            ExpressionKind::Unary { operand, .. } => {
                self.expression(operand, scope, context, ExpressionUse::Value)
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                self.expression(left, scope, context, ExpressionUse::Value);
                self.expression(right, scope, context, ExpressionUse::Value);
                if matches!(
                    operator,
                    BinaryOperator::Add
                        | BinaryOperator::Subtract
                        | BinaryOperator::Multiply
                        | BinaryOperator::Divide
                        | BinaryOperator::Remainder
                ) {
                    self.constrain(left, InputConstraint::NumberLike, left.span());
                    self.constrain(right, InputConstraint::NumberLike, right.span());
                }
            }
            ExpressionKind::IfElse {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression(condition, scope, context, ExpressionUse::Value);
                self.expression(then_branch, scope, context, ExpressionUse::Value);
                self.expression(else_branch, scope, context, ExpressionUse::Value);
            }
            ExpressionKind::BlockIf {
                condition,
                then_branch,
                else_branch,
            } => {
                self.expression(condition, scope, context, ExpressionUse::Value);
                if usage == ExpressionUse::Value && else_branch.is_none() {
                    self.error(
                        "block `if` used as a value requires an `else` branch",
                        expression.span(),
                    );
                }
                self.statements(then_branch, scope, context, usage == ExpressionUse::Value);
                if let Some(else_branch) = else_branch {
                    self.statements(else_branch, scope, context, usage == ExpressionUse::Value);
                }
            }
            ExpressionKind::Switch { target, arms } => {
                self.expression(target, scope, context, ExpressionUse::Value);
                for arm in arms {
                    match arm.pattern() {
                        SwitchPattern::Value(pattern) => {
                            self.expression(pattern, scope, context, ExpressionUse::Value)
                        }
                        SwitchPattern::Wildcard => {}
                        SwitchPattern::Binding(name) => {
                            let mut child = scope.child(scope.it_available);
                            child.locals.insert(name.clone());
                            self.expression(arm.value(), &mut child, context, ExpressionUse::Value);
                            continue;
                        }
                    }
                    self.expression(arm.value(), scope, context, ExpressionUse::Value);
                }
            }
            ExpressionKind::Subscript { receiver, index } => {
                self.expression(receiver, scope, context, ExpressionUse::Value);
                self.expression(index, scope, context, ExpressionUse::Value);
            }
        }
    }

    fn expressions(
        &mut self,
        expressions: &[Expression],
        scope: &mut Scope,
        context: ValidationContext,
    ) {
        for expression in expressions {
            self.expression(expression, scope, context, ExpressionUse::Value);
        }
    }

    fn validate_method(&mut self, receiver: &Expression, name: &str, span: Span) {
        let receiver_type = match receiver.kind() {
            ExpressionKind::Number(_) => Some("number"),
            ExpressionKind::Boolean(_) => Some("boolean"),
            ExpressionKind::Null => Some("null"),
            _ => None,
        };
        if let Some(receiver_type) = receiver_type
            && matches!(name, "css" | "regex" | "text" | "attr" | "html")
        {
            self.error(
                format!("method `{name}` cannot be called on a {receiver_type}"),
                span,
            );
        }
    }

    fn constrain(&mut self, expression: &Expression, constraint: InputConstraint, span: Span) {
        let ExpressionKind::Variable(name) = expression.kind() else {
            return;
        };
        let Some(index) = self
            .requirements
            .inputs()
            .iter()
            .position(|input| input.name() == name)
        else {
            return;
        };
        if self
            .requirements
            .constrain_input(index, constraint)
            .is_err()
        {
            self.error(
                format!("input `{name}` has incompatible inferred constraints"),
                span,
            );
        }
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::new(message, span));
    }
}

fn resolve_qualified_calls(statements: &mut [Statement], bindings: &mut HashSet<String>) {
    for statement in statements {
        match statement.kind_mut() {
            StatementKind::Assignment { name, value, .. } => {
                resolve_expression(value, bindings);
                bindings.insert(name.clone());
            }
            StatementKind::ComputedAssignment { name, value } => {
                resolve_expression(name, bindings);
                resolve_expression(value, bindings);
            }
            StatementKind::Return(value) => resolve_expression(value, bindings),
            StatementKind::While { condition, body } => {
                resolve_expression(condition, bindings);
                resolve_qualified_calls(body, bindings);
            }
            StatementKind::Loop { body } => resolve_qualified_calls(body, bindings),
            StatementKind::Break => {}
            StatementKind::Expression(expression) => resolve_expression(expression, bindings),
        }
    }
}

fn resolve_expression(expression: &mut Expression, bindings: &HashSet<String>) {
    let core = match expression.kind() {
        ExpressionKind::FunctionCall { name, arguments }
            if builtin_function(name)
                .is_some_and(|function| function.kind() == BuiltinFunctionKind::Core) =>
        {
            Some((name.clone(), arguments.clone()))
        }
        _ => None,
    };
    if let Some((name, arguments)) = core {
        *expression.kind_mut() = ExpressionKind::CoreCall { name, arguments };
    }

    let qualified = match expression.kind() {
        ExpressionKind::MethodCall {
            receiver,
            name,
            arguments,
            ..
        } => match receiver.kind() {
            ExpressionKind::Variable(module)
                if !bindings.contains(module) && builtin_function(name).is_none() =>
            {
                Some((module.clone(), name.clone(), arguments.clone()))
            }
            _ => None,
        },
        _ => None,
    };
    if let Some((module, name, arguments)) = qualified {
        *expression.kind_mut() = ExpressionKind::QualifiedFunctionCall {
            module,
            name,
            arguments,
        };
    }

    match expression.kind_mut() {
        ExpressionKind::InterpolatedString(parts) => {
            for part in parts {
                if let InterpolatedStringPart::Expression(expression) = part {
                    resolve_expression(expression, bindings);
                }
            }
        }
        ExpressionKind::Array(values) => {
            for value in values {
                resolve_expression(value, bindings);
            }
        }
        ExpressionKind::Object(fields) => {
            for field in fields {
                resolve_expression(field.value_mut(), bindings);
            }
        }
        ExpressionKind::FunctionCall { name, arguments } => {
            wrap_lambda_shorthand(name, arguments, 1);
            for argument in arguments {
                resolve_expression(argument, bindings);
            }
        }
        ExpressionKind::QualifiedFunctionCall { arguments, .. }
        | ExpressionKind::CoreCall { arguments, .. } => {
            for argument in arguments {
                resolve_expression(argument, bindings);
            }
        }
        ExpressionKind::MethodCall {
            receiver,
            name,
            arguments,
            ..
        } => {
            resolve_expression(receiver, bindings);
            wrap_lambda_shorthand(name, arguments, 0);
            for argument in arguments {
                resolve_expression(argument, bindings);
            }
        }
        ExpressionKind::Lambda { body, .. }
        | ExpressionKind::Required(body)
        | ExpressionKind::Unary { operand: body, .. } => resolve_expression(body, bindings),
        ExpressionKind::FunctionalBlock { receiver, body, .. } => {
            resolve_expression(receiver, bindings);
            let mut child = bindings.clone();
            resolve_qualified_calls(body, &mut child);
        }
        ExpressionKind::Binary { left, right, .. } => {
            resolve_expression(left, bindings);
            resolve_expression(right, bindings);
        }
        ExpressionKind::IfElse {
            condition,
            then_branch,
            else_branch,
        } => {
            resolve_expression(condition, bindings);
            resolve_expression(then_branch, bindings);
            resolve_expression(else_branch, bindings);
        }
        ExpressionKind::BlockIf {
            condition,
            then_branch,
            else_branch,
        } => {
            resolve_expression(condition, bindings);
            let mut then_bindings = bindings.clone();
            resolve_qualified_calls(then_branch, &mut then_bindings);
            if let Some(else_branch) = else_branch {
                let mut else_bindings = bindings.clone();
                resolve_qualified_calls(else_branch, &mut else_bindings);
            }
        }
        ExpressionKind::Switch { target, arms } => {
            resolve_expression(target, bindings);
            for arm in arms {
                if let SwitchPattern::Value(pattern) = arm.pattern_mut() {
                    resolve_expression(pattern, bindings);
                }
                resolve_expression(arm.value_mut(), bindings);
            }
        }
        ExpressionKind::Subscript { receiver, index } => {
            resolve_expression(receiver, bindings);
            resolve_expression(index, bindings);
        }
        ExpressionKind::Null
        | ExpressionKind::Boolean(_)
        | ExpressionKind::Number(_)
        | ExpressionKind::String(_)
        | ExpressionKind::Variable(_)
        | ExpressionKind::Identifier(_) => {}
    }
}

fn wrap_lambda_shorthand(name: &str, arguments: &mut [Expression], lambda_index: usize) {
    if !uses_single_it_lambda(name) {
        return;
    }
    let Some(argument) = arguments.get_mut(lambda_index) else {
        return;
    };
    if matches!(argument.kind(), ExpressionKind::Lambda { .. }) {
        return;
    }
    let body = argument.clone();
    let span = body.span();
    *argument = Expression::new(
        ExpressionKind::Lambda {
            parameters: vec!["it".to_owned()],
            body: Box::new(body),
        },
        span,
    );
}

fn uses_single_it_lambda(name: &str) -> bool {
    matches!(
        name,
        "map" | "filter" | "any" | "all" | "find" | "sort_by_key"
    )
}
