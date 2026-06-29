# Source Map

This chapter maps files to responsibilities.

## Facade

| File | Responsibility |
|---|---|
| `src/lib.rs` | Public `html-extractor` crate. Re-exports `html-extractor-core` and compile-time macros. |

## Core Compiler

| File | Responsibility |
|---|---|
| `crates/html-extractor-core/src/lib.rs` | Public exports for compiler, AST, diagnostics, source spans, requirements, and built-in function metadata. |
| `diagnostic.rs` | `Diagnostic` and `CompileError`. |
| `functions.rs` | Built-in function registry and kind metadata. |
| `program.rs` | `CompiledProgram`, `ProgramView`, `compile`, and hidden macro serialization helpers. |
| `requirements.rs` | Input/function requirement tracking and constraints. |
| `source.rs` | `Span`, `SourceLocation`, and retained `SourceText`. |
| `syntax/token.rs` | Token and interpolated-token types. |
| `syntax/lexer.rs` | Handwritten lexer, comments, strings, interpolation, numbers, identifiers. |
| `syntax/parser.rs` | Recursive-descent parser for statements and expressions. |
| `syntax/ast.rs` | Public AST node types. |
| `validation.rs` | Scope validation, requirement inference, control-flow checks, and qualified-call resolution. |

## Runtime

| File | Responsibility |
|---|---|
| `crates/html-extractor-runtime/src/lib.rs` | Runtime public exports and `Program` trait. |
| `engine.rs` | `Engine`, `EngineBuilder`, requirement linking, tests, host/Lua registration. |
| `evaluator.rs` | AST evaluation, flow control, function dispatch, fetch request construction, operators. |
| `functions/mod.rs` | Dispatch table for value functions. |
| `functions/arr.rs` | `len`, `range`, `zip`, `first`, `last`, `take`, `unique`, array `extend`. |
| `functions/collections.rs` | Lambda `map`, `filter`, `any`, `all`, `find`, and map blocks. |
| `functions/css.rs` | `css` and `css@`. |
| `functions/flatten.rs` | One-level array flattening. |
| `functions/guards.rs` | `nullcheck`, `assert`, and `warn`. |
| `functions/null.rs` | `unwrap_or`. |
| `functions/object.rs` | `keys`, `values`, `has`, `log`. |
| `functions/regex.rs` | `regex` and `regex@`. |
| `functions/response.rs` | `status` and `json`. |
| `functions/sorting.rs` | `sorted`, `sort_by_key`, `sort_by`. |
| `functions/text.rs` | Text, attribute, string, HTML conversion helpers. |
| `html.rs` | `HtmlDocument` and `HtmlNode`. |
| `http.rs` | `HttpRequest`, `HttpPolicy`, `HttpClient`, `ReqwestHttpClient`, `Response`. |
| `lua.rs` | Lua sandbox, module loading, and value conversion. |
| `scope.rs` | Inputs, root/child/function scopes, visibility, `extend` output. |
| `serialize.rs` | Runtime value to JSON conversion. |
| `value.rs` | `Object`, `Value`, `ValueKind`, strict accessors. |
| `error.rs` | Runtime error and diagnostic types. |

## CLI

| File | Responsibility |
|---|---|
| `crates/html-extractor-cli/src/main.rs` | Clap parsing, stdin detection, stdout/stderr wiring, exit codes. |
| `args.rs` | CLI arguments and `test` subcommand. |
| `input.rs` | Variable parsing and runtime input construction. |
| `diagnostic.rs` | File:line:column rendering. |
| `run.rs` | Extraction, test execution, Lua loading, cached HTTP client. |

## Macros

| File | Responsibility |
|---|---|
| `crates/html-extractor-macros/src/lib.rs` | `extractor_program!`, `extractor_program_file!`, compile-time diagnostics, encoded program expansion. |

## Editor Support

| Path | Responsibility |
|---|---|
| `zed/grammars/html_extractor/grammar.js` | Tree-sitter grammar source. |
| `zed/grammars/html_extractor/src/parser.c` | Generated parser. |
| `zed/languages/html_extractor/*.scm` | Highlight, bracket, indent, and outline queries. |
| `zed/extension.toml` | Local Zed extension manifest. |

## Tests and Fixtures

| Path | Responsibility |
|---|---|
| `crates/html-extractor-core/tests` | Lexer, parser, validation, source, fixture, and design example tests. |
| `crates/html-extractor-runtime/tests` | Runtime behavior, HTTP, Lua, HTML, values, functions, and end-to-end fixtures. |
| `crates/html-extractor-cli/tests` | CLI integration tests. |
| `tests/macros.rs` | Facade macro tests. |
| `test/corpus` | Tree-sitter corpus. |
