use html_extractor_core::Span;

use crate::{ExecutionError, Value};

pub(super) fn call(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    if !arguments.is_empty() {
        return Err(ExecutionError::new(
            "`flatten` expects no arguments",
            Some(span),
        ));
    }
    let arrays = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let mut output = Vec::new();
    for value in arrays {
        match value {
            Value::Array(values) => output.extend(values),
            value => output.push(value),
        }
    }
    Ok(Value::Array(output))
}
