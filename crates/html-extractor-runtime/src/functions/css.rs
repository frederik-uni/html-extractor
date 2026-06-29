use html_extractor_core::Span;
use scraper::{Html, Selector};

use crate::{ExecutionError, HtmlNode, Value};

pub(super) fn call(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
    multiple: bool,
) -> Result<Value, ExecutionError> {
    if let Value::Array(values) = receiver {
        return values
            .into_iter()
            .map(|value| call(value, arguments.clone(), span, multiple))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array);
    }
    let [Value::String(selector)] = arguments.as_slice() else {
        return Err(ExecutionError::new(
            "CSS selection expects one string selector",
            Some(span),
        ));
    };
    let source = match receiver {
        Value::HtmlDocument(document) => document.source().to_owned(),
        Value::HtmlNode(node) => node.fragment().to_owned(),
        Value::Response(response) => response.html()?.source().to_owned(),
        value => {
            return Err(ExecutionError::new(
                format!("CSS selection expected HTML, got {}", value.kind()),
                Some(span),
            ));
        }
    };
    let selector = Selector::parse(selector).map_err(|error| {
        ExecutionError::new(format!("invalid CSS selector: {error}"), Some(span))
    })?;
    let document = Html::parse_fragment(&source);
    let mut matches = document.select(&selector).map(|element| {
        let name = element.value().name();
        let attributes = element
            .value()
            .attrs()
            .map(|(name, value)| {
                format!(
                    " {name}=\"{}\"",
                    value.replace('&', "&amp;").replace('"', "&quot;")
                )
            })
            .collect::<String>();
        Value::HtmlNode(HtmlNode::new(
            format!("<{name}{attributes}>{}</{name}>", element.inner_html()),
            name,
        ))
    });
    if multiple {
        Ok(Value::Array(matches.collect()))
    } else {
        Ok(matches.next().unwrap_or(Value::Null))
    }
}
