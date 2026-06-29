# Implementation Notes

## Compiler Pipeline

`compile(source)` performs:

1. `parse(source)` in `syntax/parser.rs`.
2. `validate(&mut program)` in `validation.rs`.
3. Packaging into `CompiledProgram` with retained `SourceText`.

Lexing and parsing stop on unrecoverable syntax errors. Validation can return multiple diagnostics.

## Requirement Inference

Undefined reads become `InputRequirement`s. Calls to non-built-in, non-script functions become `FunctionRequirement`s. Requirements retain all source-use spans and function arities observed by the compiler.

Input constraints are inferred from use sites:

- HTML-like for CSS operations.
- Number-like for numeric contexts where implemented.
- Any when the compiler cannot narrow the value.

## Runtime Flow

`Engine::execute` links requirements, creates a root `Scope`, then asks `Evaluator` to run top-level statements. The final public root bindings plus `extend` output become `ExecutionResult::output`.

Evaluator flow is represented by `Flow<T>`:

- `Value(T)` for normal evaluation.
- `Return(Value)` for script function returns.
- `Break` for loop exits.

## Scope Rules

Root scope writes go into shared globals. Map-block child scopes can read outer/global values but their assignments are local. Function frames can mutate an existing global binding; first assignments to new names stay local to the function.

`extend(object)` writes public fields into an extension object that is merged into outputs.

## Function Dispatch

Built-in value functions go through `functions::call`. Lambda-sensitive functions (`map`, `filter`, `any`, `all`, `find`, `sort_by_key`, `sort_by`, `assert`, `warn`) are handled by the evaluator because their lambda expressions must remain unevaluated until the function applies them.

Host and Lua functions receive already evaluated `Vec<Value>` arguments.

## HTTP

`fetch` constructs an `HttpRequest`, validates URL policy, calls the injected `HttpClient`, then validates response size. `ReqwestHttpClient` sends GET or POST, applies request headers, installs a default user agent when none is supplied, and stores status, final URL, and body in `Response`.

`Response` lazily caches text, JSON, and HTML conversions.

## Lua

Lua modules are loaded into a sandbox without ambient `os`, `io`, `package`, or `debug`. A module must return a table of exported functions. Each export is registered as `module.function`; unique export names are also registered unqualified.

The Lua bridge supports serializable values. It rejects sparse arrays, mixed tables, cyclic tables, unsupported keys, unsupported values, duplicate invalid exports, and non-function exports.

## Serialization

JSON conversion is explicit. Serializable variants are null, boolean, number, string, array, and object. Runtime-only variants fail with `SerializationError` that includes the path and value kind.

## Known Implementation Caveats

Some prose in `docs/learning.md` describes intended behavior broader than the current implementation. The code currently shows these notable details:

- `len` supports strings and arrays in `arr.rs`.
- `contains` currently supports strings; array/object branches are `todo!()`.
- `join` currently appends a string/number to a string receiver; it is not an array-of-strings join in the implementation.
- `json` accepts responses and strings in the implementation.
- `HttpRequest` stores headers internally; the public getter set exposes URL, method, body, and content type.
