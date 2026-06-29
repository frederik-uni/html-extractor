use html_extractor_core::compile;
use html_extractor_runtime::{Engine, Inputs, Value};

#[tokio::test]
async fn calls_unique_unqualified_and_qualified_exports() {
    let engine = Engine::builder()
        .lua_module(
            "maths",
            r#"
        local function double(value) return value * 2 end
        return { double = double, object = function() return { first = 1, second = 2 } end }
        "#,
        )
        .build()
        .unwrap();
    let program = compile("a = double(3)\nb = maths.double(4)\no = maths.object()").unwrap();

    let result = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(result.output().get("a"), Some(&Value::from(6)));
    assert_eq!(result.output().get("b"), Some(&Value::from(8)));
    assert_eq!(
        result
            .output()
            .get("o")
            .unwrap()
            .expect_object()
            .unwrap()
            .get("first"),
        Some(&Value::from(1))
    );
}

#[test]
fn rejects_ambient_os_access_during_module_loading() {
    let error = Engine::builder()
        .lua_module("unsafe", "return { home = os.getenv }")
        .build()
        .unwrap_err();
    assert!(error.message().contains("Lua module"));
}

#[tokio::test]
async fn ambiguous_unqualified_exports_do_not_link() {
    let engine = Engine::builder()
        .lua_module("one", "return { convert = function(v) return v end }")
        .lua_module("two", "return { convert = function(v) return v end }")
        .build()
        .unwrap();
    let program = compile("value = convert(1)").unwrap();
    let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
    assert!(error.message().contains("missing required function"));
}

#[tokio::test]
async fn converts_dense_arrays_and_string_keyed_objects() {
    let engine = Engine::builder()
        .lua_module(
            "values",
            "return { array = function() return {10, 20} end, object = function() return {name = 'x'} end }",
        )
        .build()
        .unwrap();
    let program = compile("array = values.array()\nobject = values.object()").unwrap();
    let result = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(
        result.output().get("array"),
        Some(&Value::Array(vec![Value::from(10), Value::from(20)]))
    );
    assert_eq!(
        result
            .output()
            .get("object")
            .unwrap()
            .expect_object()
            .unwrap()
            .get("name"),
        Some(&Value::from("x"))
    );
}

#[tokio::test]
async fn round_trips_array_and_object_arguments() {
    let engine = Engine::builder()
        .lua_module("identity", "return { value = function(v) return v end }")
        .build()
        .unwrap();
    let program =
        compile("array = identity.value([1, 2])\nobject = identity.value({name: \"x\"})").unwrap();
    let result = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(
        result.output().get("array"),
        Some(&Value::Array(vec![Value::from(1), Value::from(2)]))
    );
    assert_eq!(
        result
            .output()
            .get("object")
            .unwrap()
            .expect_object()
            .unwrap()
            .get("name"),
        Some(&Value::from("x"))
    );
}

#[tokio::test]
async fn rejects_sparse_mixed_cyclic_and_unsupported_return_values() {
    for (name, expression, expected) in [
        ("sparse", "local t = {}; t[2] = 1; return t", "sparse"),
        ("mixed", "return {1, name = 'x'}", "mixed"),
        ("cyclic", "local t = {}; t.self = t; return t", "cyclic"),
        ("function_value", "return function() end", "unsupported"),
    ] {
        let source = format!("return {{ run = function() {expression} end }}");
        let engine = Engine::builder().lua_module(name, source).build().unwrap();
        let program = compile(&format!("value = {name}.run()")).unwrap();
        let error = engine.execute(&program, Inputs::new()).await.unwrap_err();
        assert!(
            error.message().to_lowercase().contains(expected),
            "unexpected error for {name}: {error}"
        );
    }
}

#[test]
fn rejects_duplicate_invalid_and_non_function_exports() {
    let duplicate = Engine::builder()
        .lua_module("same", "return { a = function() end }")
        .lua_module("same", "return { b = function() end }")
        .build()
        .unwrap_err();
    assert!(duplicate.message().contains("duplicate Lua module"));

    let invalid_name = Engine::builder()
        .lua_module("not-valid", "return {}")
        .build()
        .unwrap_err();
    assert!(invalid_name.message().contains("invalid Lua module name"));

    let non_function = Engine::builder()
        .lua_module("values", "return { answer = 42 }")
        .build()
        .unwrap_err();
    assert!(non_function.message().contains("must be a named function"));
}

#[tokio::test]
async fn module_state_is_fresh_between_executions() {
    let engine = Engine::builder()
        .lua_module(
            "counter",
            "local n = 0; return { next = function() n = n + 1; return n end }",
        )
        .build()
        .unwrap();
    let program = compile("value = counter.next()").unwrap();
    let first = engine.execute(&program, Inputs::new()).await.unwrap();
    let second = engine.execute(&program, Inputs::new()).await.unwrap();

    assert_eq!(first.output().get("value"), Some(&Value::from(1)));
    assert_eq!(second.output().get("value"), Some(&Value::from(1)));
}

#[tokio::test]
async fn local_helpers_are_not_visible_to_extractor_programs() {
    let engine = Engine::builder()
        .lua_module(
            "module",
            "local function hidden() return 1 end; return { visible = function() return hidden() end }",
        )
        .build()
        .unwrap();
    let visible = compile("value = visible()").unwrap();
    let result = engine.execute(&visible, Inputs::new()).await.unwrap();
    assert_eq!(result.output().get("value"), Some(&Value::from(1)));

    let hidden = compile("value = hidden()").unwrap();
    let error = engine.execute(&hidden, Inputs::new()).await.unwrap_err();
    assert!(
        error
            .message()
            .contains("missing required function `hidden`")
    );
}

#[tokio::test]
async fn sandbox_excludes_all_ambient_system_libraries() {
    let engine = Engine::builder()
        .lua_module(
            "sandbox",
            "return { isolated = function() return os == nil and io == nil and package == nil and debug == nil end }",
        )
        .build()
        .unwrap();
    let program = compile("isolated = sandbox.isolated()").unwrap();
    let result = engine.execute(&program, Inputs::new()).await.unwrap();
    assert_eq!(result.output().get("isolated"), Some(&Value::from(true)));
}
