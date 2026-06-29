use html_extractor_runtime::{HtmlDocument, Object, Response, Value, ValueKind};

#[test]
fn objects_preserve_insertion_order_and_accessors_are_strict() {
    let mut object = Object::new();
    object.insert("second", Value::from(2));
    object.insert("first", Value::from("one"));

    assert_eq!(object.keys().collect::<Vec<_>>(), ["second", "first"]);
    assert_eq!(
        Value::Array(vec![Value::Null])
            .expect_array()
            .unwrap()
            .len(),
        1
    );
    assert!(Value::from("not an array").expect_array().is_err());
    assert_eq!(Value::from("text").expect_string().unwrap(), "text");
}

#[test]
fn json_conversion_reports_the_complete_native_value_path() {
    let mut nested = Object::new();
    nested.insert(
        "node",
        Value::HtmlDocument(HtmlDocument::parse("<h1>title</h1>")),
    );
    let mut output = Object::new();
    output.insert("items", Value::Array(vec![Value::Object(nested)]));

    let error = Value::Object(output).to_json().unwrap_err();
    assert_eq!(error.path(), "$.items[0].node");
}

#[test]
fn compatible_values_convert_without_mutating_the_source() {
    let value = Value::Array(vec![Value::Null, Value::from(true), Value::from(12)]);
    let json = value.to_json().unwrap();

    assert_eq!(json, serde_json::json!([null, true, 12]));
    assert_eq!(value.expect_array().unwrap().len(), 3);
}

#[test]
fn strict_accessors_report_the_actual_value_kind() {
    let error = Value::Boolean(true).expect_number().unwrap_err();
    assert_eq!(error.to_string(), "expected number, got boolean");

    let error = Value::Null.expect_object().unwrap_err();
    assert_eq!(error.to_string(), "expected object, got null");

    let error = Value::from(3).expect_boolean().unwrap_err();
    assert_eq!(error.to_string(), "expected boolean, got number");
}

#[test]
fn value_kinds_cover_native_and_serializable_variants() {
    assert_eq!(Value::Null.kind(), ValueKind::Null);
    assert_eq!(Value::Array(Vec::new()).kind(), ValueKind::Array);
    assert_eq!(Value::Object(Object::new()).kind(), ValueKind::Object);
    assert_eq!(
        Value::HtmlDocument(HtmlDocument::parse("")).kind(),
        ValueKind::HtmlDocument
    );
    assert_eq!(
        Value::Response(Response::new(200, "https://test", Vec::new())).kind(),
        ValueKind::Response
    );
}

#[test]
fn serialization_reports_nested_array_paths() {
    let value = Value::Array(vec![Value::Array(vec![Value::Response(Response::new(
        200,
        "https://test",
        Vec::new(),
    ))])]);

    let error = value.to_json().unwrap_err();
    assert_eq!(error.path(), "$[0][0]");
    assert_eq!(error.kind(), ValueKind::Response);
}

#[test]
fn object_replacement_keeps_original_position() {
    let mut object = Object::new();
    object.insert("a", Value::from(1));
    object.insert("b", Value::from(2));
    assert_eq!(object.insert("a", Value::from(3)), Some(Value::from(1)));

    assert_eq!(object.keys().collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(object.get("a"), Some(&Value::from(3)));
}

#[test]
fn serialized_json_objects_keep_extractor_insertion_order() {
    let mut object = Object::new();
    object.insert("z", Value::from(1));
    object.insert("a", Value::from(2));
    object.insert("middle", Value::from(3));

    let json = Value::Object(object).to_json().unwrap();
    assert_eq!(json.to_string(), r#"{"z":1,"a":2,"middle":3}"#);
}
