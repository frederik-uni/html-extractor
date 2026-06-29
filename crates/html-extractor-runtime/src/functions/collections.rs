use html_extractor_core::{Expression, ExpressionKind, Span, Statement, Visibility};

use crate::{
    ExecutionError, Value,
    evaluator::{EvalResult, Evaluator, Flow},
    scope::Scope,
};

pub(crate) async fn map(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
) -> EvalResult {
    let [lambda] = arguments else {
        return Err(ExecutionError::new("`map` expects one lambda", Some(span)));
    };
    let ExpressionKind::Lambda { parameters, body } = lambda.kind() else {
        return Err(ExecutionError::new("`map` expects a lambda", Some(span)));
    };
    let [parameter] = parameters.as_slice() else {
        return Err(ExecutionError::new(
            "`map` expects a one-parameter lambda",
            Some(span),
        ));
    };
    let mut output = Vec::new();
    for item in receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
    {
        let mut child = scope.child();
        child.assign(parameter, item, Visibility::Private);
        match evaluator.expression(body, &mut child).await? {
            Flow::Value(value) => output.push(value),
            flow => return Ok(flow),
        }
    }
    Ok(Flow::Value(Value::Array(output)))
}

pub(crate) async fn filter(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
) -> EvalResult {
    let [lambda] = arguments else {
        return Err(ExecutionError::new(
            "`filter` expects one lambda",
            Some(span),
        ));
    };
    let ExpressionKind::Lambda { parameters, body } = lambda.kind() else {
        return Err(ExecutionError::new("`filter` expects a lambda", Some(span)));
    };
    let [parameter] = parameters.as_slice() else {
        return Err(ExecutionError::new(
            "`filter` expects a one-parameter lambda",
            Some(span),
        ));
    };
    let mut output = Vec::new();
    for item in receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
    {
        let mut child = scope.child();
        child.assign(parameter, item.clone(), Visibility::Private);
        let keep = match evaluator.expression(body, &mut child).await? {
            Flow::Value(value) => value,
            flow => return Ok(flow),
        }
        .expect_boolean()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
        if keep {
            output.push(item);
        }
    }
    Ok(Flow::Value(Value::Array(output)))
}

pub(crate) async fn predicate(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
    name: &str,
) -> EvalResult {
    let [lambda] = arguments else {
        return Err(ExecutionError::new(
            format!("`{name}` expects one lambda"),
            Some(span),
        ));
    };
    let ExpressionKind::Lambda { parameters, body } = lambda.kind() else {
        return Err(ExecutionError::new(
            format!("`{name}` expects a lambda"),
            Some(span),
        ));
    };
    let [parameter] = parameters.as_slice() else {
        return Err(ExecutionError::new(
            format!("`{name}` expects a one-parameter lambda"),
            Some(span),
        ));
    };
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    for item in values {
        let mut child = scope.child();
        child.assign(parameter, item.clone(), Visibility::Private);
        let matches = match evaluator.expression(body, &mut child).await? {
            Flow::Value(value) => value,
            flow => return Ok(flow),
        }
        .expect_boolean()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
        match (name, matches) {
            ("any", true) => return Ok(Flow::Value(Value::Boolean(true))),
            ("all", false) => return Ok(Flow::Value(Value::Boolean(false))),
            ("find", true) => return Ok(Flow::Value(item)),
            _ => {}
        }
    }
    Ok(Flow::Value(match name {
        "any" => Value::Boolean(false),
        "all" => Value::Boolean(true),
        "find" => Value::Null,
        _ => unreachable!(),
    }))
}

pub(crate) async fn map_block(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    body: &[Statement],
    scope: &Scope,
    span: Span,
) -> EvalResult {
    let mut output = Vec::new();
    for item in receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
    {
        let mut child = scope.child();
        child.assign("it", item, Visibility::Private);
        match evaluator.block(body, &mut child).await? {
            Flow::Value(_) => output.push(Value::Object(child.output())),
            flow => return Ok(flow),
        }
    }
    Ok(Flow::Value(Value::Array(output)))
}
