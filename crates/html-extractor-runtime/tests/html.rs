use html_extractor_core::compile;
use html_extractor_runtime::{Engine, HtmlDocument, Inputs, Value, Visibility};

#[tokio::test]
async fn selects_first_and_all_nodes_and_extracts_values() {
    let program = compile(
        r#"
        first = html.css(".item").text()
        all = html.css@(".item").text()
        href = html.css("a").attr("href")
        "#,
    )
    .unwrap();
    let html =
        r#"<main><div class="item">One</div><div class="item">Two</div><a href="/x">X</a></main>"#;
    let inputs = Inputs::new().insert(
        "html",
        Value::HtmlDocument(HtmlDocument::parse(html)),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("first"), Some(&Value::from("One")));
    assert_eq!(
        result.output().get("all"),
        Some(&Value::Array(vec![Value::from("One"), Value::from("Two")]))
    );
    assert_eq!(result.output().get("href"), Some(&Value::from("/x")));
}

#[tokio::test]
async fn nested_selection_preserves_shape_until_flattened() {
    let program = compile(
        r#"
        nested = html.css@("section").css@("span").text()
        flat = html.css@("section").css@("span").flatten().text()
        "#,
    )
    .unwrap();
    let html = "<section><span>A</span><span>B</span></section><section><span>C</span></section>";
    let inputs = Inputs::new().insert(
        "html",
        Value::HtmlDocument(HtmlDocument::parse(html)),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("nested"),
        Some(&Value::Array(vec![
            Value::Array(vec![Value::from("A"), Value::from("B")]),
            Value::Array(vec![Value::from("C")]),
        ]))
    );
    assert_eq!(
        result.output().get("flat"),
        Some(&Value::Array(vec![
            Value::from("A"),
            Value::from("B"),
            Value::from("C")
        ]))
    );
}

#[tokio::test]
async fn parses_html_explicitly_and_required_selection_fails() {
    let program = compile(
        r#"_doc = "<p>ok</p>".html()
value = _doc.css("missing")!"#,
    )
    .unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("required `css` operation"));
    assert!(error.message().contains("produced no result"));
}

#[tokio::test]
async fn missing_single_and_multiple_matches_have_distinct_values() {
    let program = compile(
        r#"single = html.css("missing")
multiple = html.css@("missing")"#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "html",
        Value::HtmlDocument(HtmlDocument::parse("<p>text</p>")),
        Visibility::Private,
    );
    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("single"), Some(&Value::Null));
    assert_eq!(
        result.output().get("multiple"),
        Some(&Value::Array(Vec::new()))
    );
}

#[tokio::test]
async fn attributes_preserve_nested_array_shape_and_missing_attributes_are_null() {
    let program = compile(r#"ids = html.css@("section").css@("a").attr("data-id")"#).unwrap();
    let html =
        r#"<section><a data-id="a">A</a><a>B</a></section><section><a data-id="c">C</a></section>"#;
    let inputs = Inputs::new().insert(
        "html",
        Value::HtmlDocument(HtmlDocument::parse(html)),
        Visibility::Private,
    );
    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("ids"),
        Some(&Value::Array(vec![
            Value::Array(vec![Value::from("a"), Value::Null]),
            Value::Array(vec![Value::from("c")]),
        ]))
    );
}

#[tokio::test]
async fn invalid_dynamic_selectors_are_source_located_errors() {
    let program = compile("value = html.css(selector)").unwrap();
    let inputs = Inputs::new()
        .insert(
            "html",
            Value::HtmlDocument(HtmlDocument::parse("<p/>")),
            Visibility::Private,
        )
        .insert("selector", Value::from("["), Visibility::Private);
    let error = Engine::new().execute(&program, inputs).await.unwrap_err();

    assert!(error.message().contains("invalid CSS selector"));
    assert!(error.span().is_some());
}

#[tokio::test]
async fn dynamic_non_html_receivers_are_rejected() {
    let program = compile("value = html.css(\"p\")").unwrap();
    let inputs = Inputs::new().insert("html", Value::from(12), Visibility::Private);
    let error = Engine::new().execute(&program, inputs).await.unwrap_err();
    assert!(error.message().contains("requires an HTML-like value"));
}

#[tokio::test]
async fn extracts_inner_markup_and_text_from_html_nodes() {
    let program = compile(
        r#"
        markup = html.css("article")!.inner_html()
        text = html.css("article")!.html_text()
        all_text = html.css@("p").html_text()
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "html",
        Value::HtmlDocument(HtmlDocument::parse(
            "<article><strong>Hello</strong> world</article><p>One</p><p>Two</p>",
        )),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("markup"),
        Some(&Value::from("<strong>Hello</strong> world"))
    );
    assert_eq!(
        result.output().get("text"),
        Some(&Value::from("Hello world"))
    );
    assert_eq!(
        result.output().get("all_text"),
        Some(&Value::Array(vec![Value::from("One"), Value::from("Two")]))
    );
}

#[tokio::test]
async fn node_only_html_functions_reject_documents() {
    for method in ["inner_html", "html_text"] {
        let program = compile(&format!("value = html.{method}()")).unwrap();
        let inputs = Inputs::new().insert(
            "html",
            Value::HtmlDocument(HtmlDocument::parse("<p>text</p>")),
            Visibility::Private,
        );
        let error = Engine::new().execute(&program, inputs).await.unwrap_err();
        assert!(error.message().contains("expected an HTML node"));
    }
}
