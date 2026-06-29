use html_extractor_core::{CompileError, Diagnostic, SourceLocation, SourceText, Span, format};

#[test]
fn spans_slice_retained_source() {
    let source = SourceText::new("first\nsecond");
    let span = Span::new(6, 12);

    assert_eq!(source.slice(span), Some("second"));
}

#[test]
fn invalid_spans_do_not_slice_source() {
    let source = SourceText::new("short");

    assert_eq!(source.slice(Span::new(4, 2)), None);
    assert_eq!(source.slice(Span::new(0, 8)), None);
}

#[test]
fn byte_offsets_map_to_one_based_line_and_column() {
    let source = SourceText::new("alpha\nβeta\nlast");

    assert_eq!(source.location(0), Some(SourceLocation::new(1, 1)));
    assert_eq!(source.location(6), Some(SourceLocation::new(2, 1)));
    assert_eq!(source.location(8), Some(SourceLocation::new(2, 2)));
    assert_eq!(source.location(12), Some(SourceLocation::new(3, 1)));
}

#[test]
fn locations_reject_non_character_boundaries_and_out_of_bounds_offsets() {
    let source = SourceText::new("β");

    assert_eq!(source.location(1), None);
    assert_eq!(source.location(3), None);
}

#[test]
fn compile_errors_expose_stable_diagnostics() {
    let diagnostic = Diagnostic::new("unexpected token", Span::new(3, 4));
    let error = CompileError::new(diagnostic.clone());

    assert_eq!(diagnostic.message(), "unexpected token");
    assert_eq!(diagnostic.primary_span(), Span::new(3, 4));
    assert_eq!(error.diagnostics(), &[diagnostic]);
    assert_eq!(error.to_string(), "unexpected token");
}

#[test]
fn format_normalizes_valid_programs() {
    let source = r#"output=html.css("article").map{
title=it.css("h1").text()
if title {value=title}else{value="untitled"}
}
test{assertEq(1,1,"math")}"#;

    assert_eq!(
        format(source).unwrap(),
        r#"output = html.css("article").map {
    title = it.css("h1").text()
    if title {
        value = title
    } else {
        value = "untitled"
    }
}

test {
    assertEq(1, 1, "math")
}
"#
    );
}

#[test]
fn format_preserves_private_assignment_prefix_once() {
    assert_eq!(
        format("_info = catalog.extract_series_chapter(_url)").unwrap(),
        "_info = catalog.extract_series_chapter(_url)\n"
    );
}

#[test]
fn format_preserves_parentheses_and_required_postfix() {
    assert_eq!(
        format("value=(1+2)*3\nrequired_value=html.css(\"title\")!.text()").unwrap(),
        "value = (1 + 2) * 3\nrequired_value = html.css(\"title\")!.text()\n"
    );
}

#[test]
fn format_breaks_long_postfix_chains() {
    assert_eq!(
        format("count=[1,2,3].map(v=>v+1).filter(v=>v>1).sort_by_key(v=>v).len()").unwrap(),
        r#"count = [1, 2, 3]
    .map(v => v + 1)
    .filter(v => v > 1)
    .sort_by_key(v => v)
    .len()
"#
    );
}

#[test]
fn format_prints_core_calls_without_dollar_prefix() {
    assert_eq!(
        format(r#"response=fetch("https://example.test").json()"#).unwrap(),
        r#"response = fetch("https://example.test").json()
"#
    );
}

#[test]
fn format_prefers_dot_access_for_identifier_like_string_subscripts() {
    assert_eq!(
        format(r#"images=data["images"]["image_alt"]["images-alt"]"#).unwrap(),
        r#"images = data.images.image_alt["images-alt"]
"#
    );
}

#[test]
fn format_keeps_implicit_it_lambda_shorthand() {
    assert_eq!(
        format("urls = items.map(it.url)").unwrap(),
        "urls = items.map(it.url)\n"
    );
}
