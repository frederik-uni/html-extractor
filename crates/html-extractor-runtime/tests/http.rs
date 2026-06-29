use std::sync::{Arc, Mutex};

use futures::future::BoxFuture;
use html_extractor_core::compile;
use html_extractor_runtime::{
    Engine, HttpClient, HttpMethod, HttpPolicy, HttpRequest, Inputs, Response, Value,
};

#[derive(Debug, Default)]
struct RecordingClient {
    request: Mutex<Option<HttpRequest>>,
}

impl HttpClient for RecordingClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, html_extractor_runtime::ExecutionError>> {
        *self.request.lock().unwrap() = Some(request.clone());
        Box::pin(async move { Ok(Response::new(200, request.url(), Vec::new())) })
    }
}

#[derive(Debug)]
struct FixtureClient;

impl HttpClient for FixtureClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, html_extractor_runtime::ExecutionError>> {
        Box::pin(async move {
            Ok(Response::new(
                201,
                request.url(),
                br#"<h1>Title</h1>{"ok":true}"#.to_vec(),
            ))
        })
    }
}

#[derive(Debug)]
struct StaticClient {
    body: Vec<u8>,
}

impl HttpClient for StaticClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, html_extractor_runtime::ExecutionError>> {
        let body = self.body.clone();
        Box::pin(async move { Ok(Response::new(200, request.url(), body)) })
    }
}

#[derive(Debug)]
struct FailingClient;

impl HttpClient for FailingClient {
    fn fetch<'a>(
        &'a self,
        _request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, html_extractor_runtime::ExecutionError>> {
        Box::pin(async { Err(html_extractor_runtime::ExecutionError::new("offline", None)) })
    }
}

#[tokio::test]
async fn post_with_json_body_builds_http_request() {
    let client = Arc::new(RecordingClient::default());
    let engine = Engine::builder()
        .http_client(client.clone())
        .build()
        .unwrap();
    let program = compile(
        r#"response = fetch("https://fixture.test/items", {
            method: "POST",
            json: { name: "item", quantity: 2 }
        })"#,
    )
    .unwrap();

    engine.execute(&program, Inputs::new()).await.unwrap();

    let request = client.request.lock().unwrap().clone().unwrap();
    assert_eq!(request.method(), HttpMethod::Post);
    assert_eq!(request.url(), "https://fixture.test/items");
    assert_eq!(
        request.body(),
        Some(br#"{"name":"item","quantity":2}"#.as_slice())
    );
    assert_eq!(request.content_type(), Some("application/json"));
}

#[tokio::test]
async fn fetch_without_options_builds_get_request() {
    let client = Arc::new(RecordingClient::default());
    let engine = Engine::builder()
        .http_client(client.clone())
        .build()
        .unwrap();
    let program = compile(r#"response = fetch("https://fixture.test/items")"#).unwrap();

    engine.execute(&program, Inputs::new()).await.unwrap();

    let request = client.request.lock().unwrap().clone().unwrap();
    assert_eq!(request.method(), HttpMethod::Get);
    assert_eq!(request.url(), "https://fixture.test/items");
    assert_eq!(request.body(), None);
    assert_eq!(request.content_type(), None);
    assert!(!request.cloudflare());
}

#[tokio::test]
async fn cloudflare_option_marks_request_for_managed_cookies() {
    let client = Arc::new(RecordingClient::default());
    let engine = Engine::builder()
        .http_client(client.clone())
        .build()
        .unwrap();
    let program =
        compile(r#"response = fetch("https://fixture.test/items", { cloudflare: true })"#).unwrap();

    engine.execute(&program, Inputs::new()).await.unwrap();

    let request = client.request.lock().unwrap().clone().unwrap();
    assert!(request.cloudflare());
}

#[tokio::test]
async fn cloudflare_option_must_be_boolean() {
    let engine = Engine::builder()
        .http_client(Arc::new(FixtureClient))
        .build()
        .unwrap();
    let program =
        compile(r#"response = fetch("https://fixture.test", { cloudflare: "yes" })"#).unwrap();

    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();

    assert!(
        error
            .message()
            .contains("`fetch` option `cloudflare` must be a boolean")
    );
}

#[tokio::test]
async fn conditional_fetch_can_leave_api_urls_unfetched() {
    let client = Arc::new(RecordingClient::default());
    let engine = Engine::builder()
        .http_client(client.clone())
        .build()
        .unwrap();
    let program =
        compile(r#"url = url if url.starts_with("https://api") else fetch(url).text()"#).unwrap();
    let inputs = Inputs::new().insert(
        "url",
        Value::from("https://api.fixture.test/items"),
        html_extractor_runtime::Visibility::Private,
    );

    let result = engine.execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("url"),
        Some(&Value::from("https://api.fixture.test/items"))
    );
    assert!(client.request.lock().unwrap().is_none());
}

#[tokio::test]
async fn fetch_rejects_invalid_post_options_at_the_call_site() {
    let engine = Engine::builder()
        .http_client(Arc::new(FixtureClient))
        .build()
        .unwrap();
    let cases = [
        (
            r#"response = fetch("https://fixture.test", 1)"#,
            "options must be an object",
        ),
        (
            r#"response = fetch("https://fixture.test", { json: {} })"#,
            "GET options must not include `json`",
        ),
        (
            r#"response = fetch("https://fixture.test", { method: 1, json: {} })"#,
            "option `method` must be a string",
        ),
        (
            r#"response = fetch("https://fixture.test", { method: "PUT", json: {} })"#,
            "unsupported `fetch` method `PUT`",
        ),
    ];

    for (source, expected) in cases {
        let program = compile(source).unwrap();
        let error = engine
            .execute(&program, Inputs::new())
            .await
            .err()
            .unwrap_or_else(|| panic!("expected `{source}` to fail"));
        assert!(
            error.message().contains(expected),
            "expected `{expected}` in `{}`",
            error.message()
        );
        assert!(error.span().is_some());
    }
}

#[tokio::test]
async fn fetch_rejects_non_serializable_json_before_sending_post() {
    let engine = Engine::builder()
        .http_client(Arc::new(FixtureClient))
        .build()
        .unwrap();
    let program = compile(
        r#"_body = fetch("https://fixture.test/body")
        response = fetch("https://fixture.test/items", {
            method: "POST",
            json: { nested: _body }
        })"#,
    )
    .unwrap();

    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();

    assert!(error.message().contains("invalid `fetch` JSON body"));
    assert!(error.message().contains("$.nested"));
    assert!(error.span().is_some());
}

#[tokio::test]
async fn fetches_and_converts_owned_responses() {
    let engine = Engine::builder()
        .http_client(Arc::new(FixtureClient))
        .build()
        .unwrap();
    let program = compile(
        r#"
        _response = fetch("https://fixture.test")
        status = _response.status()
        body = _response.text()
        title = _response.css("h1")!.text()
        matched = _response.regex("Title")
        "#,
    )
    .unwrap();

    let result = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(result.output().get("status"), Some(&Value::from(201)));
    assert_eq!(result.output().get("title"), Some(&Value::from("Title")));
    assert_eq!(result.output().get("matched"), Some(&Value::from("Title")));
    assert!(
        result
            .output()
            .get("body")
            .unwrap()
            .expect_string()
            .unwrap()
            .contains("Title")
    );
}

#[tokio::test]
async fn policy_rejects_disallowed_schemes_before_calling_client() {
    let policy = HttpPolicy::default().allow_schemes(["https"]);
    let engine = Engine::builder()
        .http_client(Arc::new(FixtureClient))
        .http_policy(policy)
        .build()
        .unwrap();
    let program = compile(r#"value = fetch("http://fixture.test")"#).unwrap();

    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
    assert!(error.message().contains("scheme"));
}

#[tokio::test]
async fn converts_json_responses_into_native_ordered_values() {
    let engine = Engine::builder()
        .http_client(Arc::new(StaticClient {
            body: br#"{"name":"item","values":[1,true,null]}"#.to_vec(),
        }))
        .build()
        .unwrap();
    let program = compile(
        r#"_response = fetch("https://fixture.test")
data = _response.json()"#,
    )
    .unwrap();
    let result = engine.execute(&program, Inputs::new()).await.unwrap();
    let data = result
        .output()
        .get("data")
        .unwrap()
        .expect_object()
        .unwrap();

    assert_eq!(data.get("name"), Some(&Value::from("item")));
    assert_eq!(
        data.get("values"),
        Some(&Value::Array(vec![
            Value::from(1),
            Value::from(true),
            Value::Null
        ]))
    );
}

#[tokio::test]
async fn invalid_json_and_utf8_are_conversion_errors() {
    let json_engine = Engine::builder()
        .http_client(Arc::new(StaticClient {
            body: b"not-json".to_vec(),
        }))
        .build()
        .unwrap();
    let json_program = compile(
        r#"_response = fetch("https://fixture.test")
data = _response.json()"#,
    )
    .unwrap();
    let error = json_engine
        .execute(&json_program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("invalid response JSON"));

    let text_engine = Engine::builder()
        .http_client(Arc::new(StaticClient {
            body: vec![0xff, 0xfe],
        }))
        .build()
        .unwrap();
    let text_program = compile(
        r#"_response = fetch("https://fixture.test")
data = _response.text()"#,
    )
    .unwrap();
    let error = text_engine
        .execute(&text_program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("not UTF-8"));
}

#[tokio::test]
async fn response_body_limits_are_enforced_after_fetch() {
    let engine = Engine::builder()
        .http_client(Arc::new(StaticClient { body: vec![0; 5] }))
        .http_policy(HttpPolicy::default().max_response_bytes(4))
        .build()
        .unwrap();
    let program = compile(r#"response = fetch("https://fixture.test")"#).unwrap();
    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
    assert!(error.message().contains("exceeds 4 byte limit"));
}

#[tokio::test]
async fn client_failures_are_attached_to_the_fetch_call_site() {
    let engine = Engine::builder()
        .http_client(Arc::new(FailingClient))
        .build()
        .unwrap();
    let program = compile(r#"response = fetch("https://fixture.test")"#).unwrap();
    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
    assert_eq!(error.message(), "offline");
    assert!(error.span().is_some());
}

#[test]
fn response_text_html_and_json_conversions_are_cached() {
    let text = Response::new(200, "https://test", b"text".to_vec());
    assert!(std::ptr::eq(text.text().unwrap(), text.text().unwrap()));

    let html = Response::new(200, "https://test", b"<p>text</p>".to_vec());
    assert!(std::ptr::eq(html.html().unwrap(), html.html().unwrap()));

    let json = Response::new(200, "https://test", br#"{"ok":true}"#.to_vec());
    assert!(std::ptr::eq(json.json().unwrap(), json.json().unwrap()));
}

#[test]
fn response_can_carry_headers() {
    let response = Response::new_with_headers(
        200,
        "https://test",
        b"body".to_vec(),
        vec![("set-cookie".to_string(), "a=b; Path=/".to_string())],
    );

    assert_eq!(
        response.headers(),
        &[("set-cookie".to_string(), "a=b; Path=/".to_string())]
    );
}

#[tokio::test]
async fn response_methods_reject_unexpected_arguments() {
    let engine = Engine::builder()
        .http_client(Arc::new(StaticClient {
            body: b"{}".to_vec(),
        }))
        .build()
        .unwrap();
    for source in [
        r#"_response = fetch("https://test")
value = _response.status(1)"#,
        r#"_response = fetch("https://test")
value = _response.json(1)"#,
        r#"_response = fetch("https://test")
value = _response.text(1)"#,
        r#"_response = fetch("https://test")
value = _response.html(1)"#,
    ] {
        let program = compile(source).unwrap();
        let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
        assert!(error.message().contains("expects no arguments"));
    }
}
