use std::{error::Error, fmt};

use html_extractor_runtime::{HtmlDocument, Inputs, Object, Value, Visibility};
use serde_json::{Map, Value as JsonValue};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum StdinInput {
    Terminal,
    Redirected(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputError(String);

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for InputError {}

pub(crate) fn parse_assignment(assignment: &str) -> Result<(String, JsonValue), InputError> {
    let Some((name, raw_value)) = assignment.split_once('=') else {
        return Err(InputError(format!(
            "invalid --variable `{assignment}`: expected NAME=VALUE"
        )));
    };
    if name.is_empty() {
        return Err(InputError(format!(
            "invalid --variable `{assignment}`: NAME must not be empty"
        )));
    }
    let value =
        serde_json::from_str(raw_value).unwrap_or_else(|_| JsonValue::String(raw_value.to_owned()));
    Ok((name.to_owned(), value))
}

pub(crate) fn merge_variables(
    variables: Option<&str>,
    assignments: &[String],
) -> Result<Map<String, JsonValue>, InputError> {
    let mut merged = match variables {
        Some(raw) => match serde_json::from_str(raw) {
            Ok(JsonValue::Object(values)) => values,
            Ok(_) => {
                return Err(InputError(
                    "invalid --variables: expected a JSON object".into(),
                ));
            }
            Err(error) => {
                return Err(InputError(format!("invalid --variables JSON: {error}")));
            }
        },
        None => Map::new(),
    };
    for assignment in assignments {
        let (name, value) = parse_assignment(assignment)?;
        merged.insert(name, value);
    }
    Ok(merged)
}

pub(crate) fn json_to_value(value: JsonValue) -> Value {
    match value {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(value) => Value::Boolean(value),
        JsonValue::Number(value) => Value::Number(value),
        JsonValue::String(value) => Value::String(value),
        JsonValue::Array(values) => Value::Array(values.into_iter().map(json_to_value).collect()),
        JsonValue::Object(values) => {
            let mut object = Object::new();
            for (name, value) in values {
                object.insert(name, json_to_value(value));
            }
            Value::Object(object)
        }
    }
}

pub(crate) fn build_inputs(
    stdin: StdinInput,
    variables: Option<&str>,
    assignments: &[String],
) -> Result<Inputs, InputError> {
    let mut inputs = Inputs::new();
    if let StdinInput::Redirected(source) = stdin {
        inputs = inputs.insert(
            "html",
            Value::HtmlDocument(HtmlDocument::parse(source)),
            Visibility::Private,
        );
    }
    for (name, value) in merge_variables(variables, assignments)? {
        inputs = inputs.insert(name, json_to_value(value), Visibility::Private);
    }
    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{json_to_value, merge_variables, parse_assignment};

    #[test]
    fn assignment_values_use_json_then_string_fallback() {
        assert_eq!(
            parse_assignment("limit=10").unwrap(),
            ("limit".into(), json!(10))
        );
        assert_eq!(
            parse_assignment("enabled=true").unwrap(),
            ("enabled".into(), json!(true))
        );
        assert_eq!(
            parse_assignment("category=books").unwrap(),
            ("category".into(), json!("books"))
        );
    }

    #[test]
    fn assignments_require_a_non_empty_name_and_equals_sign() {
        assert!(parse_assignment("missing").is_err());
        assert!(parse_assignment("=value").is_err());
    }

    #[test]
    fn repeated_variables_override_json_in_argument_order() {
        let variables = merge_variables(
            Some(r#"{"html":"json","limit":1}"#),
            &["html=explicit".into(), "limit=2".into(), "limit=3".into()],
        )
        .unwrap();

        assert_eq!(variables.get("html"), Some(&json!("explicit")));
        assert_eq!(variables.get("limit"), Some(&json!(3)));
    }

    #[test]
    fn variables_json_must_be_an_object() {
        assert!(merge_variables(Some("[]"), &[]).is_err());
        assert!(merge_variables(Some("null"), &[]).is_err());
    }

    #[test]
    fn json_values_convert_recursively_without_losing_object_order() {
        let value = json_to_value(json!({
            "name": "books",
            "flags": [true, null],
            "count": 2
        }));

        assert_eq!(
            value.to_json().unwrap(),
            json!({
                "name": "books",
                "flags": [true, null],
                "count": 2
            })
        );
    }
}
