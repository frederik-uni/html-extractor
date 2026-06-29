use html_extractor_core::compile;
use html_extractor_runtime::{Engine, Inputs, Value};

#[tokio::test]
async fn script_functions_shadow_host_functions() {
    let program = compile("fun custom(value) { value + 1 } result = custom(4)").unwrap();
    let engine = Engine::builder()
        .host_function("custom", |_| async { Ok(Value::from(99)) })
        .build()
        .unwrap();

    let result = engine.execute(&program, Inputs::new()).await.unwrap();
    assert_eq!(result.output().get("result"), Some(&Value::from(5)));
}

#[tokio::test]
async fn links_and_calls_async_host_functions() {
    let engine = Engine::builder()
        .host_function("double", |arguments| async move {
            let number = arguments[0].expect_number().unwrap().as_i64().unwrap();
            Ok(Value::from(number * 2))
        })
        .build()
        .unwrap();
    let program = compile("value = double(21)").unwrap();

    let result = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(result.output().get("value"), Some(&Value::from(42)));
}

#[tokio::test]
async fn missing_external_functions_fail_during_linking() {
    let program = compile("value = missing(1)").unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("missing required function"));
}

#[test]
fn host_functions_cannot_replace_reserved_core_functions() {
    let error = Engine::builder()
        .host_function("fetch", |_| async { Ok(Value::Null) })
        .build()
        .unwrap_err();
    assert!(error.message().contains("reserved"));
}

#[tokio::test]
async fn qualified_host_functions_link_by_exact_identity() {
    let engine = Engine::builder()
        .host_function("tools.answer", |_| async { Ok(Value::from(42)) })
        .build()
        .unwrap();
    let program = compile("value = tools.answer()").unwrap();
    let result = engine.execute(&program, Inputs::new()).await.unwrap();
    assert_eq!(result.output().get("value"), Some(&Value::from(42)));
}

#[tokio::test]
async fn host_errors_gain_the_extractor_call_span() {
    let engine = Engine::builder()
        .host_function("fail", |_| async {
            Err(html_extractor_runtime::ExecutionError::new(
                "host failed",
                None,
            ))
        })
        .build()
        .unwrap();
    let program = compile("value = fail()").unwrap();
    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
    assert_eq!(error.message(), "host failed");
    assert!(error.span().is_some());
}

#[tokio::test]
async fn executions_do_not_share_input_state() {
    let program = compile("value = input").unwrap();
    let engine = Engine::new();
    let first = engine
        .execute(
            &program,
            Inputs::new().insert(
                "input",
                Value::from(1),
                html_extractor_runtime::Visibility::Private,
            ),
        )
        .await
        .unwrap();
    let second = engine
        .execute(
            &program,
            Inputs::new().insert(
                "input",
                Value::from(2),
                html_extractor_runtime::Visibility::Private,
            ),
        )
        .await
        .unwrap();
    assert_eq!(first.output().get("value"), Some(&Value::from(1)));
    assert_eq!(second.output().get("value"), Some(&Value::from(2)));
}
