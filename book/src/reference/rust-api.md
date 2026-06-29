# Rust API

## Facade Crate: `html-extractor`

`src/lib.rs` re-exports the compiler crate and macros:

```rust
pub use html_extractor_core::*;
pub use html_extractor_macros::{extractor_program, extractor_program_file};
```

Use this crate when consuming HTML Extractor as a library.

## Core Compiler API

| Item | Purpose |
|---|---|
| `compile(source: &str)` | Lexes, parses, validates, and returns `CompiledProgram`. |
| `CompiledProgram` | Immutable source, validated AST, function/test blocks, and requirements. |
| `ProgramView` | Read-only trait over compiled programs. |
| `Requirements` | Inputs and external function requirements inferred during validation. |
| `InputRequirement` | Input name, constraint, and source-use spans. |
| `FunctionRequirement` | Function identity, observed arities, and source-use spans. |
| `Span` | Half-open byte range into source. |
| `SourceText` | Retained source with slicing and byte-offset to line/column mapping. |
| `CompileError` | One or more compiler diagnostics. |

Example:

```rust
let program = html_extractor_core::compile("title = html.css(\"h1\")!.text()")?;
assert_eq!(program.requirements().inputs()[0].name(), "html");
# Ok::<(), html_extractor_core::CompileError>(())
```

## AST API

The runtime consumes the public AST types.

| Item | Important variants or accessors |
|---|---|
| `Program` | `functions`, `tests`, `statements`, `span` |
| `FunctionDefinition` | `name`, `parameters`, `body`, `span` |
| `TestBlock` | `inputs`, `body`, `runs_script`, `span` |
| `StatementKind` | assignment, computed assignment, return, while, loop, break, expression |
| `ExpressionKind` | literals, variables, calls, method calls, lambdas, blocks, required, operators, if, switch, subscript |
| `AssignmentOperator` | `Assign`, `Add`, `Subtract` |
| `Visibility` | `Public`, `Private` |
| `UnaryOperator` | `Negate`, `Not` |
| `BinaryOperator` | logical, equality, comparison, arithmetic |

## Runtime API

| Item | Purpose |
|---|---|
| `Engine` | Executes programs and runs embedded tests. |
| `EngineBuilder` | Registers host functions, Lua modules, HTTP client, and HTTP policy. |
| `ExecutionResult` | Ordered output plus warnings. |
| `TestReport` | Total and passed test counts. |
| `Inputs` | Host-provided bindings with visibility. |
| `Value` | Runtime value enum. |
| `Object` | Ordered map used by runtime values and outputs. |
| `HtmlDocument`, `HtmlNode` | HTML runtime values. |
| `HttpClient`, `HttpRequest`, `HttpPolicy`, `Response` | HTTP boundary and response model. |
| `ExecutionError`, `Diagnostic`, `SerializationError`, `TypeError` | Runtime error and warning surfaces. |

## Runtime Values

```rust
pub enum Value {
    Null,
    Boolean(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(Object),
    HtmlDocument(HtmlDocument),
    HtmlNode(HtmlNode),
    Response(Response),
}
```

Strict accessors return typed errors:

```rust
let value = html_extractor_runtime::Value::from("hello");
assert_eq!(value.expect_string()?, "hello");
# Ok::<(), html_extractor_runtime::TypeError>(())
```

`Value::to_json` serializes only JSON-compatible runtime values. `Value::to_debug_json` is used by `log`.

## HTTP API

`HttpRequest::get` and `HttpRequest::post` construct request descriptions. `ReqwestHttpClient` is the default implementation. `Response` caches UTF-8 text, JSON, and parsed HTML conversions.

## CLI Internal API

The CLI keeps most items crate-private:

| Item | Purpose |
|---|---|
| `Args` and `Command` | Clap parser for run and test modes. |
| `run_with_io` | Testable CLI entry point. |
| `build_inputs` | Converts stdin and CLI variables into runtime inputs. |
| `render` | Converts spans into file:line:column diagnostics. |
| `CachedHttpClient` | Request cache for the `test` subcommand. |
