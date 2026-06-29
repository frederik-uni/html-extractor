use html_extractor_core::Span;

use crate::{ExecutionError, Value};

pub(super) fn keys(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("keys", &arguments, span)?;
    match receiver {
        Value::Object(object) => Ok(Value::Array(
            object
                .keys()
                .map(|v| Value::String(v.to_owned()))
                .collect::<Vec<_>>(),
        )),
        _ => return Err(ExecutionError::new("keys only work on objects", Some(span))),
    }
}

pub(super) fn log(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("log", &arguments, span)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receiver.to_debug_json().unwrap()).unwrap()
    );
    Ok(receiver)
}

pub(super) fn values(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("values", &arguments, span)?;
    let Value::Object(object) = receiver else {
        return Err(ExecutionError::new(
            "values only works on objects",
            Some(span),
        ));
    };
    Ok(Value::Array(
        object.iter().map(|(_, value)| value.clone()).collect(),
    ))
}

pub(super) fn has(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(key)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`has` expects exactly one string key",
            Some(span),
        ));
    };
    let Value::Object(object) = receiver else {
        return Err(ExecutionError::new("has only works on objects", Some(span)));
    };
    Ok(Value::Boolean(object.get(key).is_some()))
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
