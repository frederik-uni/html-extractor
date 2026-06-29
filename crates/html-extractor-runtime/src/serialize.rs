use serde_json::json;

use crate::{Object, SerializationError, Value};

pub(crate) fn to_json(value: &Value, debug: bool) -> Result<serde_json::Value, SerializationError> {
    if debug {
        convert_debug(value, "$".to_owned())
    } else {
        convert(value, "$".to_owned())
    }
}

fn convert(value: &Value, path: String) -> Result<serde_json::Value, SerializationError> {
    Ok(match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(value) => serde_json::Value::Bool(*value),
        Value::Number(value) => serde_json::Value::Number(value.clone()),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| convert(value, format!("{path}[{index}]")))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(values) => object(values, path)?,
        Value::HtmlDocument(_) | Value::HtmlNode(_) | Value::Response(_) => {
            return Err(SerializationError::unsupported(path, value.kind()));
        }
    })
}

fn convert_debug(value: &Value, path: String) -> Result<serde_json::Value, SerializationError> {
    Ok(match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(value) => serde_json::Value::Bool(*value),
        Value::Number(value) => serde_json::Value::Number(value.clone()),
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| convert(value, format!("{path}[{index}]")))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(values) => object(values, path)?,
        Value::HtmlDocument(_) => serde_json::Value::String("HtmlDocument".to_owned()),
        Value::HtmlNode(_) => serde_json::Value::String("HtmlNode".to_owned()),
        Value::Response(r) => {
            json!({ "type": "Response", "status": r.status() , "url":r.url(), "body": r.text().unwrap_or_default()[..50].to_owned()})
        }
    })
}

fn object(values: &Object, path: String) -> Result<serde_json::Value, SerializationError> {
    let mut result = serde_json::Map::with_capacity(values.len());
    for (name, value) in values {
        result.insert(name.clone(), convert(value, format!("{path}.{name}"))?);
    }
    Ok(serde_json::Value::Object(result))
}
