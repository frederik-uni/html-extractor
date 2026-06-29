use html_extractor_core::{Expression, ExpressionKind, Span, Visibility};

use crate::{
    Diagnostic, ExecutionError, Value,
    evaluator::{EvalResult, Evaluator, Flow},
    scope::Scope,
};

pub(super) fn nullcheck(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let message = match arguments.as_slice() {
        [Value::String(message)] => message.as_str(),
        _ => {
            return Err(ExecutionError::new(
                "`nullcheck` expects one string argument",
                Some(span),
            ));
        }
    };
    if receiver == Value::Null {
        Err(ExecutionError::new(message, Some(span)))
    } else {
        Ok(receiver)
    }
}

pub(crate) async fn predicate(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
    warning: bool,
) -> EvalResult {
    if let [message] = arguments
        && let Value::Boolean(passed) = receiver
    {
        let message = match evaluator.expression(message, scope).await? {
            Flow::Value(value) => value,
            flow => return Ok(flow),
        }
        .expect_string()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
        .to_owned();
        if !passed {
            if warning {
                evaluator.push_warning(Diagnostic::new(message, Some(span)));
            } else {
                return Err(ExecutionError::new(message, Some(span)));
            }
        }
        return Ok(Flow::Value(Value::Boolean(passed)));
    }

    let [lambda, message] = arguments else {
        return Err(ExecutionError::new(
            "guard expects a lambda and message",
            Some(span),
        ));
    };
    let ExpressionKind::Lambda { parameters, body } = lambda.kind() else {
        return Err(ExecutionError::new("guard expects a lambda", Some(span)));
    };
    let [parameter] = parameters.as_slice() else {
        return Err(ExecutionError::new(
            "guard expects a one-parameter lambda",
            Some(span),
        ));
    };
    let message = match evaluator.expression(message, scope).await? {
        Flow::Value(value) => value,
        flow => return Ok(flow),
    }
    .expect_string()
    .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
    .to_owned();
    let mut child = scope.child();
    child.assign(parameter, receiver.clone(), Visibility::Private);
    let passed = match evaluator.expression(body, &mut child).await? {
        Flow::Value(value) => value,
        flow => return Ok(flow),
    }
    .expect_boolean()
    .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    if !passed {
        if warning {
            evaluator.push_warning(Diagnostic::new(message, Some(span)));
        } else {
            return Err(ExecutionError::new(message, Some(span)));
        }
    }
    Ok(Flow::Value(receiver))
}
