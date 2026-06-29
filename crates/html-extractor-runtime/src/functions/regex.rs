use html_extractor_core::Span;
use regex::Regex;

use crate::{ExecutionError, Value};

pub(super) fn call(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
    multiple: bool,
) -> Result<Value, ExecutionError> {
    match receiver {
        Value::Array(values) => values
            .into_iter()
            .map(|value| call(value, arguments.clone(), span, multiple))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::String(text) => scalar(&text, &arguments, span, multiple),
        Value::Response(response) => scalar(response.text()?, &arguments, span, multiple),
        value => Err(ExecutionError::new(
            format!("`regex` expected string or array, got {}", value.kind()),
            Some(span),
        )),
    }
}

fn scalar(
    text: &str,
    arguments: &[Value],
    span: Span,
    multiple: bool,
) -> Result<Value, ExecutionError> {
    let pattern = arguments
        .first()
        .ok_or_else(|| ExecutionError::new("`regex` expects a pattern", Some(span)))?
        .expect_string()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let regex = Regex::new(pattern)
        .map_err(|error| ExecutionError::new(format!("invalid regex: {error}"), Some(span)))?;
    if multiple {
        return regex
            .captures_iter(text)
            .map(|captures| capture(&captures, arguments.get(1), span))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array);
    }
    let Some(captures) = regex.captures(text) else {
        return Ok(Value::Null);
    };
    capture(&captures, arguments.get(1), span)
}

fn capture(
    captures: &regex::Captures<'_>,
    capture: Option<&Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let matched = match capture {
        None => captures.get(0),
        Some(Value::Number(number)) => number
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| captures.get(index)),
        Some(Value::String(name)) => captures.name(name),
        Some(value) => {
            return Err(ExecutionError::new(
                format!(
                    "regex capture must be a number or string, got {}",
                    value.kind()
                ),
                Some(span),
            ));
        }
    };
    Ok(matched.map_or(Value::Null, |value| Value::from(value.as_str())))
}
