use html_extractor_core::Span;

use crate::{ExecutionError, Value};

pub(super) fn unwrap_or(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [value] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`unwrap_or` expects one value ",
            Some(span),
        ));
    };
    Ok(match receiver {
        Value::Null => value.clone(),
        v => v,
    })
}
