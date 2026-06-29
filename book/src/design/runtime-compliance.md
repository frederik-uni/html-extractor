# Runtime Compliance

This is a condensed mdBook version of `docs/runtime-spec-compliance.md`.

## Implemented and Covered

| Requirement | Implementation |
|---|---|
| Sequential AST evaluation and pipeline forwarding | `crates/html-extractor-runtime/src/evaluator.rs` |
| Public/private bindings and ordered output | `scope.rs`, `value.rs` |
| Native scalar, array, object, HTML, node, and response values | `value.rs`, `html.rs`, `http.rs` |
| Strict typed accessors | `value.rs` |
| Explicit JSON conversion and ordered objects | `serialize.rs`, `value.rs` |
| CSS selection and nested shape preservation | `functions/css.rs`, `functions/text.rs` |
| Regex first/all matches and captures | `functions/regex.rs` |
| One-level flattening | `functions/flatten.rs` |
| Sorting helpers | `functions/sorting.rs` |
| Required result checks | `evaluator.rs` |
| `map`, `filter`, predicate helpers, map blocks | `functions/collections.rs` |
| Guards and warnings | `functions/guards.rs` |
| Per-execution inputs and requirement linking | `engine.rs`, `scope.rs` |
| Host functions | `engine.rs`, `evaluator.rs` |
| HTTP client, response conversions, policy checks | `http.rs`, `functions/response.rs`, `evaluator.rs` |
| Lua exports and sandboxing | `lua.rs`, `engine.rs` |
| CLI execution, diagnostics, Lua loading, JSON output | `crates/html-extractor-cli/src` |

## Partially Implemented

| Area | Current state |
|---|---|
| Required-match diagnostics | Messages include operation, selector/pattern, span, and first item index, but not structured payload fields. |
| Runtime diagnostics | Spans are retained, but richer filename/URL/status chains are not complete. |
| HTTP policy | Allowed schemes and response-size limits exist; redirect limits, timeout controls, hostname allowlists, and default-header policy are incomplete. |
| HTTP response model | Status, final URL, body, and cached conversions are retained; response headers are not exposed. |
| Lua execution isolation | Calls are isolated; shared module environment per execution is not implemented. |
| Function arity metadata | Observed arities are tracked; declared host arity validation is not present. |

## Not Implemented

- Controlled HTML/response userdata through Lua.
- `core.fetch` inside Lua functions.
- Lua instruction and memory limits.
- Full `ExecutionLimits` builder API.
- `Engine::compile` convenience API.
- A richer structured diagnostic payload model.

## Intentional Non-Goals

There is no browser execution, filesystem access from DSL or Lua, streaming HTTP, automatic dispatcher-level array mapping, partial output API, or parallel DSL execution.
