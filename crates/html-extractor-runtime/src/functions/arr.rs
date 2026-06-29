use html_extractor_core::Span;

use crate::{ExecutionError, Value};

pub(super) fn len(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("len", &arguments, span)?;
    Ok(match receiver {
        Value::String(str) => Value::Number(str.len().into()),
        Value::Array(values) => Value::Number(values.len().into()),
        v => {
            return Err(ExecutionError::new(
                format!("`{:?}` not supported by len", v),
                Some(span),
            ));
        }
    })
}

pub(super) fn range(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [end] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`range` expects exactly two arguments",
            Some(span),
        ));
    };
    let start = integer(&receiver, "range", span)?;
    let end = integer(end, "range", span)?;
    Ok(Value::Array((start..end).map(Value::from).collect()))
}

pub(super) fn zip(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    if arguments.is_empty() {
        return Err(ExecutionError::new(
            "`zip` expects at least two arrays",
            Some(span),
        ));
    }
    let first = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let expected_len = first.len();
    let mut arrays = Vec::with_capacity(arguments.len() + 1);
    arrays.push(first);
    for argument in arguments {
        let values = argument
            .into_array()
            .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
        if values.len() != expected_len {
            return Err(ExecutionError::new(
                "`zip` expects arrays with the same length",
                Some(span),
            ));
        }
        arrays.push(values);
    }
    let mut rows = Vec::with_capacity(expected_len);
    for index in 0..expected_len {
        rows.push(Value::Array(
            arrays.iter().map(|values| values[index].clone()).collect(),
        ));
    }
    Ok(Value::Array(rows))
}

pub(super) fn first(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("first", &arguments, span)?;
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    Ok(values.into_iter().next().unwrap_or(Value::Null))
}

pub(super) fn last(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("last", &arguments, span)?;
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    Ok(values.into_iter().next_back().unwrap_or(Value::Null))
}

pub(super) fn take(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [count] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`take` expects exactly one count",
            Some(span),
        ));
    };
    let count = non_negative_integer(count, "take", span)?;
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    Ok(Value::Array(values.into_iter().take(count).collect()))
}

pub(super) fn unique(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("unique", &arguments, span)?;
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let mut output = Vec::new();
    for value in values {
        if !output.contains(&value) {
            output.push(value);
        }
    }
    Ok(Value::Array(output))
}

pub(super) fn extend(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [other] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`extend` expects exactly two arrays",
            Some(span),
        ));
    };
    let mut output = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    output.extend(
        other
            .expect_array()
            .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?
            .iter()
            .cloned(),
    );
    Ok(Value::Array(output))
}

fn integer(value: &Value, name: &str, span: Span) -> Result<i64, ExecutionError> {
    value
        .expect_number()
        .ok()
        .and_then(serde_json::Number::as_i64)
        .ok_or_else(|| {
            ExecutionError::new(format!("`{name}` expects integer arguments"), Some(span))
        })
}

fn non_negative_integer(value: &Value, name: &str, span: Span) -> Result<usize, ExecutionError> {
    value
        .expect_number()
        .ok()
        .and_then(serde_json::Number::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| {
            ExecutionError::new(
                format!("`{name}` expects a non-negative integer"),
                Some(span),
            )
        })
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
