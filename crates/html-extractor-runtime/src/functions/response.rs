use html_extractor_core::Span;

use crate::{ExecutionError, Value, http::json_to_value};

pub(super) fn status(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("status", &arguments, span)?;
    let Value::Response(response) = receiver else {
        return Err(ExecutionError::new(
            "`status` expects a response",
            Some(span),
        ));
    };
    Ok(Value::from(response.status()))
}

pub(super) fn json(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("json", &arguments, span)?;
    match receiver {
        Value::Response(response) => Ok(json_to_value(response.json()?)),
        Value::String(str) => Ok(json_to_value(&serde_json::from_str(&str).map_err(
            |err| {
                ExecutionError::new(
                    format!("Failed to parse `{str}` to json: {:?}", err),
                    Some(span),
                )
            },
        )?)),
        _ => Err(ExecutionError::new("`json` expects a response", Some(span))),
    }
}

fn no_arguments(name: &str, arguments: &[Value], span: Span) -> Result<(), ExecutionError> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(ExecutionError::new(
            format!("`{name}` expects no arguments"),
            Some(span),
        ))
    }
}
