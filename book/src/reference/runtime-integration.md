# Runtime Integration

The runtime crate executes `CompiledProgram` values from `html-extractor-core`.

## Engine

```rust
use html_extractor_runtime::{Engine, Inputs};

# async fn run(program: html_extractor_core::CompiledProgram) -> Result<(), html_extractor_runtime::ExecutionError> {
let engine = Engine::new();
let result = engine.execute(&program, Inputs::new()).await?;
# Ok(())
# }
```

`Engine::execute` checks inferred input and function requirements before evaluation. Missing inputs, incompatible inputs, or missing external functions fail before script execution.

## Inputs

```rust
use html_extractor_runtime::{HtmlDocument, Inputs, Value, Visibility};

let inputs = Inputs::new()
    .insert("html", Value::HtmlDocument(HtmlDocument::parse("<h1>Hello</h1>")), Visibility::Private)
    .insert("limit", Value::from(10_i32), Visibility::Private);
```

Input visibility participates in normal scope output rules. CLI inputs are private.

## Host Functions

```rust
use html_extractor_runtime::{Engine, ExecutionError, Value};

let engine = Engine::builder()
    .host_function("parse_price", |arguments| async move {
        let text = arguments[0].expect_string()?.replace(',', ".");
        let number = text.parse::<f64>().map_err(|error| {
            ExecutionError::new(format!("invalid price: {error}"), None)
        })?;
        Ok(Value::Number(serde_json::Number::from_f64(number).unwrap()))
    })
    .build()?;
```

Host functions are async and receive evaluated `Vec<Value>` arguments. `fetch` is reserved and cannot be replaced.

## HTTP

`HttpClient` is injectable:

```rust
use futures::future::BoxFuture;
use html_extractor_runtime::{ExecutionError, HttpClient, HttpPolicy, HttpRequest, Response};

#[derive(Debug)]
struct FixtureClient;

impl HttpClient for FixtureClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, ExecutionError>> {
        Box::pin(async move {
            assert_eq!(request.url(), "https://example.test/");
            Ok(Response::new(200, request.url(), b"<h1>Hello</h1>".to_vec()))
        })
    }
}
```

Policy controls allowed schemes and maximum response size:

```rust
let policy = html_extractor_runtime::HttpPolicy::default()
    .allow_schemes(["https"])
    .max_response_bytes(1024 * 1024);
```

## Lua

Lua modules are added at build time:

```rust
let engine = html_extractor_runtime::Engine::builder()
    .lua_module("prices", r#"
        return {
            parse_price = function(text)
                return tonumber(string.match(text, "%d+"))
            end
        }
    "#)
    .build()?;
```

The namespace is explicit. Unique exports are also registered unqualified. Duplicate module names are rejected.

Lua can exchange serializable extractor values: null, booleans, numbers, strings, arrays, and objects. Sparse, mixed, cyclic, and unsupported tables are rejected.

## Results

```rust
let output = result.output();
let warnings = result.warnings();
let json = result.to_json()?;
```

`to_json` fails if the public output still contains runtime-only values such as `HtmlDocument`, `HtmlNode`, or `Response`.
