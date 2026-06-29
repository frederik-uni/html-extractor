use html_extractor_core::compile;
use html_extractor_runtime::{Engine, Inputs, Object, Value, Visibility};

#[tokio::test]
async fn collection_helpers_support_standalone_and_method_calls() {
    let program = compile(
        r#"
        ranged = range(1, 5)
        zipped = zip([1, 2], ["a", "b"], [true, false])
        zipped_method = [1, 2].zip(["a", "b"])
        first = [1, 2].first()
        empty_first = [].first()
        last = [1, 2].last()
        empty_last = [].last()
        taken = [1, 2, 3].take(2)
        unique_values = [1, 1, 2, 1].unique()
        extended = [1, 2].extend([3, 4])
        extended_function = extend([1, 2], [3, 4])
        object_values = { a: 1, b: 2 }.values()
        object_has = { a: 1 }.has("a")
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(
        result.output().get("ranged"),
        Some(&Value::Array(vec![1.into(), 2.into(), 3.into(), 4.into()]))
    );
    assert_eq!(
        result.output().get("zipped_method"),
        Some(&Value::Array(vec![
            Value::Array(vec![1.into(), Value::from("a")]),
            Value::Array(vec![2.into(), Value::from("b")])
        ]))
    );
    assert_eq!(result.output().get("first"), Some(&Value::from(1)));
    assert_eq!(result.output().get("empty_first"), Some(&Value::Null));
    assert_eq!(result.output().get("last"), Some(&Value::from(2)));
    assert_eq!(result.output().get("empty_last"), Some(&Value::Null));
    assert_eq!(
        result.output().get("taken"),
        Some(&Value::Array(vec![1.into(), 2.into()]))
    );
    assert_eq!(
        result.output().get("unique_values"),
        Some(&Value::Array(vec![1.into(), 2.into()]))
    );
    assert_eq!(
        result.output().get("extended"),
        Some(&Value::Array(vec![1.into(), 2.into(), 3.into(), 4.into()]))
    );
    assert_eq!(
        result.output().get("extended_function"),
        result.output().get("extended")
    );
    assert_eq!(
        result.output().get("object_values"),
        Some(&Value::Array(vec![1.into(), 2.into()]))
    );
    assert_eq!(result.output().get("object_has"), Some(&Value::from(true)));
    assert_eq!(
        result
            .output()
            .get("zipped")
            .unwrap()
            .expect_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn log_supports_standalone_and_method_calls() {
    let program = compile(
        r#"
        method = { value: 1 }.log()
        standalone = log({ value: 2 })
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    let mut method = Object::new();
    method.insert("value", Value::from(1));
    assert_eq!(result.output().get("method"), Some(&Value::Object(method)));

    let mut standalone = Object::new();
    standalone.insert("value", Value::from(2));
    assert_eq!(
        result.output().get("standalone"),
        Some(&Value::Object(standalone))
    );
}

#[tokio::test]
async fn collection_helpers_reject_invalid_arguments() {
    for (source, expected) in [
        ("value = zip([1], [2, 3])", "same length"),
        ("value = zip([1])", "at least two arrays"),
        ("value = range(1.5, 3)", "integer arguments"),
        ("value = [1].take(-1)", "non-negative integer"),
        ("value = { a: 1 }.has(1)", "string key"),
    ] {
        let program = compile(source).unwrap();
        let error = Engine::new()
            .execute(&program, Inputs::new())
            .await
            .unwrap_err();
        assert!(
            error.message().contains(expected),
            "expected `{expected}` in `{}`",
            error.message()
        );
    }
}

#[tokio::test]
async fn regex_supports_full_numbered_and_named_matches() {
    let program = compile(
        r#"
        full = text.regex("id=(\\d+)")
        numbered = text.regex("id=(\\d+)", 1)
        named = text.regex("id=(?<id>\\d+)", "id")
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert("text", Value::from("id=42"), Visibility::Private);
    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("full"), Some(&Value::from("id=42")));
    assert_eq!(result.output().get("numbered"), Some(&Value::from("42")));
    assert_eq!(result.output().get("named"), Some(&Value::from("42")));
}

#[tokio::test]
async fn regex_returns_null_for_missing_matches_and_captures() {
    let program = compile(
        r#"missing = "abc".regex("\\d+")
capture = "id=".regex("id=(\\d*)", 2)"#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(result.output().get("missing"), Some(&Value::Null));
    assert_eq!(result.output().get("capture"), Some(&Value::Null));
}

#[tokio::test]
async fn regex_preserves_nested_array_shape() {
    let program = compile(r#"matches = values.regex("\\d+")"#).unwrap();
    let inputs = Inputs::new().insert(
        "values",
        Value::Array(vec![
            Value::from("a1"),
            Value::Array(vec![Value::from("b2"), Value::from("none")]),
        ]),
        Visibility::Private,
    );
    let result = Engine::new().execute(&program, inputs).await.unwrap();
    assert_eq!(
        result.output().get("matches"),
        Some(&Value::Array(vec![
            Value::from("1"),
            Value::Array(vec![Value::from("2"), Value::Null]),
        ]))
    );
}

#[tokio::test]
async fn invalid_regex_patterns_and_capture_types_fail() {
    let invalid_pattern = compile(r#"value = "x".regex("[")"#).unwrap();
    let error = Engine::new()
        .execute(&invalid_pattern, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("invalid regex"));

    let invalid_capture = compile(r#"value = "x".regex("x", true)"#).unwrap();
    let error = Engine::new()
        .execute(&invalid_capture, Inputs::new())
        .await
        .unwrap_err();
    assert!(
        error
            .message()
            .contains("capture must be a number or string")
    );
}

#[tokio::test]
async fn flatten_removes_exactly_one_layer_and_keeps_scalar_members() {
    let program = compile("value = [[[1]], 2, [3]].flatten()").unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(
        result.output().get("value"),
        Some(&Value::Array(vec![
            Value::Array(vec![Value::from(1)]),
            Value::from(2),
            Value::from(3),
        ]))
    );
}

#[tokio::test]
async fn flatten_rejects_non_arrays_and_arguments() {
    let dynamic = compile("value = input.flatten()").unwrap();
    let inputs = Inputs::new().insert("input", Value::from(1), Visibility::Private);
    let error = Engine::new().execute(&dynamic, inputs).await.unwrap_err();
    assert!(error.message().contains("expected array"));

    let arguments = compile("value = [1].flatten(1)").unwrap();
    let error = Engine::new()
        .execute(&arguments, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("expects no arguments"));
}

#[tokio::test]
async fn required_regex_reports_the_first_missing_array_item() {
    let program = compile(r#"matches = values.regex("\\d+")!"#).unwrap();
    let inputs = Inputs::new().insert(
        "values",
        Value::Array(vec![
            Value::from("1"),
            Value::from("missing"),
            Value::from("3"),
        ]),
        Visibility::Private,
    );
    let error = Engine::new().execute(&program, inputs).await.unwrap_err();
    assert!(error.message().contains("regex"));
    assert!(error.message().contains("item index 1"));
    assert!(error.message().contains("\\d+"));
}

#[tokio::test]
async fn required_multi_selection_reports_nested_empty_item_index() {
    let program = compile(r#"matches = documents.css@("p")!"#).unwrap();
    let inputs = Inputs::new().insert(
        "documents",
        Value::Array(vec![
            Value::HtmlDocument(html_extractor_runtime::HtmlDocument::parse("<p>ok</p>")),
            Value::HtmlDocument(html_extractor_runtime::HtmlDocument::parse(
                "<div>missing</div>",
            )),
        ]),
        Visibility::Private,
    );
    let error = Engine::new().execute(&program, inputs).await.unwrap_err();
    assert!(error.message().contains("css@"));
    assert!(error.message().contains("item index 1"));
    assert!(error.message().contains("p"));
}

#[tokio::test]
async fn regex_at_returns_every_full_match_and_capture() {
    let program = compile(
        r#"
        full = "a b".regex@("[ab]")
        captures = "a b".regex@("(?:(a)|(b))", 1)
        "#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(
        result.output().get("full"),
        Some(&Value::Array(vec![Value::from("a"), Value::from("b")]))
    );
    assert_eq!(
        result.output().get("captures"),
        Some(&Value::Array(vec![Value::from("a"), Value::Null]))
    );
}

#[tokio::test]
async fn sorted_orders_homogeneous_numbers_and_strings() {
    let program = compile(
        r#"
        numbers = [3, 1, 2, 1].sorted()
        strings = ["beta", "alpha", "gamma"].sorted()
        "#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(
        result.output().get("numbers"),
        Some(&Value::Array(vec![1.into(), 1.into(), 2.into(), 3.into()]))
    );
    assert_eq!(
        result.output().get("strings"),
        Some(&Value::Array(vec![
            Value::from("alpha"),
            Value::from("beta"),
            Value::from("gamma"),
        ]))
    );
}

#[tokio::test]
async fn sorted_rejects_mixed_and_null_values() {
    for source in ["value = [1, \"2\"].sorted()", "value = [1, null].sorted()"] {
        let program = compile(source).unwrap();
        let error = Engine::new()
            .execute(&program, Inputs::new())
            .await
            .unwrap_err();
        assert!(
            error
                .message()
                .contains("homogeneous non-null strings or numbers")
        );
    }
}

#[tokio::test]
async fn starts_with_returns_whether_a_string_has_the_prefix() {
    let program = compile(
        r#"
        matches = "https://api.example.test".starts_with("https://api")
        misses = "https://www.example.test".starts_with("https://api")
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("matches"), Some(&Value::from(true)));
    assert_eq!(result.output().get("misses"), Some(&Value::from(false)));
}
