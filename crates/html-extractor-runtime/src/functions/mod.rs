mod arr;
pub(crate) mod collections;
mod css;
mod flatten;
pub(crate) mod guards;
mod null;
mod object;
mod regex;
mod response;
pub(crate) mod sorting;
mod text;

use html_extractor_core::Span;

use crate::{ExecutionError, Value};

pub(crate) fn call(
    name: &str,
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    match name {
        "css" => css::call(receiver, arguments, span, false),
        "css@" => css::call(receiver, arguments, span, true),
        "flatten" => flatten::call(receiver, arguments, span),
        "text" => text::text(receiver, arguments, span),
        "keys" => object::keys(receiver, arguments, span),
        "trim" => text::trim(receiver, arguments, span),
        "log" => object::log(receiver, arguments, span),
        "replace" => text::replace(receiver, arguments, span),
        "urlencode" => text::urlencode(receiver, arguments, span),
        "contains" => text::contains(receiver, arguments, span),
        "starts_with" => text::starts_with(receiver, arguments, span),
        "inner_html" => text::inner_html(receiver, arguments, span),
        "html_text" => text::html_text(receiver, arguments, span),
        "href" => text::href(receiver, arguments, span),
        "len" => arr::len(receiver, arguments, span),
        "range" => arr::range(receiver, arguments, span),
        "zip" => arr::zip(receiver, arguments, span),
        "first" => arr::first(receiver, arguments, span),
        "last" => arr::last(receiver, arguments, span),
        "take" => arr::take(receiver, arguments, span),
        "unique" => arr::unique(receiver, arguments, span),
        "extend" => arr::extend(receiver, arguments, span),
        "values" => object::values(receiver, arguments, span),
        "has" => object::has(receiver, arguments, span),
        "src" => text::src(receiver, arguments, span),
        "unwrap_or" => null::unwrap_or(receiver, arguments, span),
        "attr" => text::attr(receiver, arguments, span),
        "html" => text::html(receiver, arguments, span),
        "regex" => regex::call(receiver, arguments, span, false),
        "regex@" => regex::call(receiver, arguments, span, true),
        "sorted" => sorting::sorted(receiver, arguments, span),
        "status" => response::status(receiver, arguments, span),
        "json" => response::json(receiver, arguments, span),
        "ensure_suffix" => text::ensure_suffix(receiver, arguments, span),
        "join" => text::join(receiver, arguments, span),
        "nullcheck" => guards::nullcheck(receiver, arguments, span),
        _ => Err(ExecutionError::new(
            format!("unknown runtime function `{name}`"),
            Some(span),
        )),
    }
}
