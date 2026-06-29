use html_extractor_core::Span;
use scraper::{Html, Selector};

use crate::{ExecutionError, HtmlDocument, Value};

pub(super) fn text(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("text", &arguments, span)?;
    map_array(receiver, |value| match value {
        Value::String(value) => Ok(Value::String(value)),
        Value::HtmlDocument(document) => Ok(Value::from(
            Html::parse_document(document.source())
                .root_element()
                .text()
                .collect::<String>(),
        )),
        Value::HtmlNode(node) => Ok(Value::from(node_text(
            node.fragment(),
            node.root_tag(),
            span,
        )?)),
        Value::Response(response) => Ok(Value::from(response.text()?.to_owned())),
        value => Err(ExecutionError::new(
            format!("`text` expected HTML or string, got {}", value.kind()),
            Some(span),
        )),
    })
}

pub(super) fn trim(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("trim", &arguments, span)?;

    map_array(receiver, |value| match value {
        Value::String(v) => Ok(Value::String(v.trim().to_owned())),

        v => {
            return Err(ExecutionError::new(
                format!("`contains` is not supported for {:?}", v),
                Some(span),
            ));
        }
    })
}

pub(super) fn contains(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(contains)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`contains` expects one string ",
            Some(span),
        ));
    };
    match receiver {
        Value::String(v) => Ok(Value::Boolean(v.contains(contains))),
        Value::Array(_values) => todo!(),
        Value::Object(_object) => todo!(),
        v => {
            return Err(ExecutionError::new(
                format!("`contains` is not supported for {:?}", v),
                Some(span),
            ));
        }
    }
}

pub(super) fn starts_with(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(prefix)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`starts_with` expects one string",
            Some(span),
        ));
    };
    match receiver {
        Value::String(value) => Ok(Value::Boolean(value.starts_with(prefix))),
        value => Err(ExecutionError::new(
            format!("`starts_with` expected a string, got {}", value.kind()),
            Some(span),
        )),
    }
}

pub(super) fn src(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("src", &arguments, span)?;
    attr(receiver, vec![Value::String("src".to_owned())], span)
}

pub(super) fn href(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("href", &arguments, span)?;
    attr(receiver, vec![Value::String("href".to_owned())], span)
}
pub(super) fn join(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [value] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`join` expects one string or number item",
            Some(span),
        ));
    };

    let join = match value {
        Value::Number(number) => number.to_string(),
        Value::String(s) => s.to_owned(),
        _ => {
            return Err(ExecutionError::new(
                "`join` expects one string or number item",
                Some(span),
            ));
        }
    };
    Ok(match receiver {
        Value::String(v) => Value::String(format!("{v}{join}")),
        v => {
            return Err(ExecutionError::new(
                format!("`join` only supports string types and not {:?}", v),
                Some(span),
            ));
        }
    })
}

pub(super) fn ensure_suffix(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(suffix)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`ensure_suffix` expects one string name",
            Some(span),
        ));
    };
    Ok(match receiver {
        Value::String(v) => match v.ends_with(suffix) {
            true => Value::String(v),
            false => Value::String(format!("{v}{suffix}")),
        },
        v => {
            return Err(ExecutionError::new(
                format!("`ensure_suffix` only supports string types and not {:?}", v),
                Some(span),
            ));
        }
    })
}

pub(super) fn replace(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(from), Value::String(to)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`replace` expects 2 strings",
            Some(span),
        ));
    };
    match receiver {
        Value::String(s) => Ok(Value::String(s.replace(from, to))),
        value => Err(ExecutionError::new(
            format!("`attr` expected an HTML node, got {}", value.kind()),
            Some(span),
        )),
    }
}

pub(super) fn urlencode(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("urlencode", &arguments, span)?;
    map_array(receiver, |value| match value {
        Value::String(value) => {
            let encoded = serde_urlencoded::to_string([("value", value)]).map_err(|error| {
                ExecutionError::new(format!("failed to encode URL value: {error}"), Some(span))
            })?;
            Ok(Value::String(
                encoded
                    .strip_prefix("value=")
                    .unwrap_or(encoded.as_str())
                    .to_owned(),
            ))
        }
        value => Err(ExecutionError::new(
            format!("`urlencode` expected a string, got {}", value.kind()),
            Some(span),
        )),
    })
}

pub(super) fn attr(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    let [Value::String(name)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "`attr` expects one string name",
            Some(span),
        ));
    };
    map_array(receiver, |value| match value {
        Value::HtmlNode(node) => Ok(node_attr(node.fragment(), node.root_tag(), name, span)?
            .map_or(Value::Null, Value::from)),
        value => Err(ExecutionError::new(
            format!("`attr` expected an HTML node, got {}", value.kind()),
            Some(span),
        )),
    })
}

pub(super) fn inner_html(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("inner_html", &arguments, span)?;
    map_array(receiver, |value| match value {
        Value::HtmlNode(node) => Ok(Value::from(node_inner_html(
            node.fragment(),
            node.root_tag(),
            span,
        )?)),
        value => Err(ExecutionError::new(
            format!("`inner_html` expected an HTML node, got {}", value.kind()),
            Some(span),
        )),
    })
}

pub(super) fn html_text(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("html_text", &arguments, span)?;
    map_array(receiver, |value| match value {
        Value::HtmlNode(node) => Ok(Value::from(node_text(
            node.fragment(),
            node.root_tag(),
            span,
        )?)),
        value => Err(ExecutionError::new(
            format!("`html_text` expected an HTML node, got {}", value.kind()),
            Some(span),
        )),
    })
}

pub(super) fn html(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    no_arguments("html", &arguments, span)?;
    map_array(receiver, |value| match value {
        Value::String(source) => Ok(Value::HtmlDocument(HtmlDocument::parse(
            source
                .replace("<noscript>", "<notscript>")
                .replace("</noscript>", "</notscript>"),
        ))),
        Value::HtmlDocument(_) => Ok(value),
        Value::Response(response) => Ok(Value::HtmlDocument(response.html()?.clone())),
        value => Err(ExecutionError::new(
            format!("`html` expected a string, got {}", value.kind()),
            Some(span),
        )),
    })
}

fn map_array(
    receiver: Value,
    scalar: impl Fn(Value) -> Result<Value, ExecutionError> + Copy,
) -> Result<Value, ExecutionError> {
    match receiver {
        Value::Array(values) => values
            .into_iter()
            .map(|value| map_array(value, scalar))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        value => scalar(value),
    }
}

fn node_text(source: &str, tag: &str, span: Span) -> Result<String, ExecutionError> {
    let document = Html::parse_fragment(source);
    let selector = Selector::parse(tag)
        .map_err(|error| ExecutionError::new(format!("invalid HTML node: {error}"), Some(span)))?;
    document
        .select(&selector)
        .next()
        .map(|element| element.text().collect())
        .ok_or_else(|| ExecutionError::new("invalid HTML node", Some(span)))
}

fn node_inner_html(source: &str, tag: &str, span: Span) -> Result<String, ExecutionError> {
    let document = Html::parse_fragment(source);
    let selector = Selector::parse(tag)
        .map_err(|error| ExecutionError::new(format!("invalid HTML node: {error}"), Some(span)))?;
    document
        .select(&selector)
        .next()
        .map(|element| element.inner_html())
        .ok_or_else(|| ExecutionError::new("invalid HTML node", Some(span)))
}

fn node_attr(
    source: &str,
    tag: &str,
    name: &str,
    span: Span,
) -> Result<Option<String>, ExecutionError> {
    let document = Html::parse_fragment(source);
    let selector = Selector::parse(tag)
        .map_err(|error| ExecutionError::new(format!("invalid HTML node: {error}"), Some(span)))?;
    document
        .select(&selector)
        .next()
        .map(|element| element.value().attr(name).map(str::to_owned))
        .ok_or_else(|| ExecutionError::new("invalid HTML node", Some(span)))
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
