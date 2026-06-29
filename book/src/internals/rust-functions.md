# Rust Function Index

This index covers production Rust functions and methods in `src/` and `crates/`. Test-only functions are not listed individually; the test suites are mapped in [Source Map](./source-map.md).

## Facade

### `src/lib.rs`

No local functions. The facade re-exports `html_extractor_core::*`, `extractor_program!`, and `extractor_program_file!`.

## `html-extractor-macros`

### `crates/html-extractor-macros/src/lib.rs`

| Function | Purpose |
|---|---|
| `extractor_program(input)` | Procedural macro entry point for inline source literals. |
| `extractor_program_file(input)` | Procedural macro entry point for manifest-relative extractor files. |
| `expand(source, name, tracked_path, span)` | Shared compile/diagnostic/embed implementation. |

## `html-extractor-core`

### `diagnostic.rs`

| Function | Purpose |
|---|---|
| `Diagnostic::new` | Build a compiler diagnostic. |
| `Diagnostic::message` | Borrow diagnostic text. |
| `Diagnostic::primary_span` | Return the primary source span. |
| `CompileError::new` | Build an error with one diagnostic. |
| `CompileError::from_diagnostics` | Build an error with multiple diagnostics. |
| `CompileError::diagnostics` | Borrow all diagnostics. |
| `CompileError::fmt` | Display the first diagnostic message. |

### `functions.rs`

| Function | Purpose |
|---|---|
| `BuiltinFunction::name` | Return the built-in name. |
| `BuiltinFunction::kind` | Return core/lambda/value kind. |
| `builtin` | Const helper for the built-in table. |
| `builtin_function` | Look up a built-in by name. |

### `program.rs`

| Function | Purpose |
|---|---|
| `CompiledProgram::requirements` | Borrow inferred requirements. |
| `CompiledProgram::source` | Borrow retained source. |
| `CompiledProgram::statements` | Borrow validated top-level statements. |
| `CompiledProgram::functions` | Borrow script function definitions. |
| `CompiledProgram::tests` | Borrow embedded test blocks. |
| `ProgramView::requirements/source/statements/functions/tests` | Read-only program trait surface. |
| `compile` | Parse, validate, and package source into `CompiledProgram`. |
| `__encode_compiled` | Hidden macro helper that serializes validated program payloads. |
| `__decode_compiled` | Hidden macro helper that reconstructs macro-embedded programs. |

### `requirements.rs`

| Function | Purpose |
|---|---|
| `Requirements::inputs` | Borrow input requirements. |
| `Requirements::functions` | Borrow external function requirements. |
| `Requirements::require_input` | Record or update an input requirement. |
| `Requirements::constrain_input` | Merge an inferred input constraint. |
| `Requirements::require_function` | Record or update an external function requirement and arity. |
| `InputRequirement::name` | Borrow input name. |
| `InputRequirement::constraint` | Return input constraint. |
| `InputRequirement::uses` | Borrow source-use spans. |
| `FunctionRequirement::identity` | Return qualified or unqualified identity. |
| `FunctionRequirement::arities` | Borrow observed arities. |
| `FunctionRequirement::uses` | Borrow source-use spans. |

### `source.rs`

| Function | Purpose |
|---|---|
| `Span::new` | Create a half-open byte span. |
| `Span::start` | Return start byte offset. |
| `Span::end` | Return end byte offset. |
| `SourceLocation::new` | Create one-based line/column. |
| `SourceLocation::line` | Return line. |
| `SourceLocation::column` | Return column. |
| `SourceText::new` | Retain source text. |
| `SourceText::as_str` | Borrow source. |
| `SourceText::slice` | Slice by span when valid. |
| `SourceText::location` | Convert byte offset to one-based location. |
| `SourceText::from` | Build from `&str`. |

### `syntax/token.rs`

| Function | Purpose |
|---|---|
| `Token::new` | Create a token with kind and span. |
| `Token::kind` | Borrow token kind. |
| `Token::span` | Return token span. |

### `syntax/ast.rs`

| Function | Purpose |
|---|---|
| `Program::new` | Construct a parsed program. |
| `Program::functions` / `tests` / `statements` / `span` | Borrow program sections and span. |
| `Program::functions_mut` / `tests_mut` / `statements_mut` | Mutable validation accessors. |
| `TestBlock::new` | Construct a test block. |
| `TestBlock::inputs` / `body` / `runs_script` / `span` | Test accessors. |
| `TestBlock::body_mut` | Mutable test body accessor. |
| `TestInput::new` | Construct test input. |
| `TestInput::name` / `visibility` / `value` / `span` | Test input accessors. |
| `FunctionDefinition::new` | Construct script function definition. |
| `FunctionDefinition::name` / `parameters` / `body` / `span` | Function definition accessors. |
| `FunctionDefinition::body_mut` | Mutable body accessor. |
| `Statement::new` | Construct statement. |
| `Statement::kind` / `span` | Statement accessors. |
| `Statement::kind_mut` | Mutable statement kind accessor. |
| `Expression::new` | Construct expression. |
| `Expression::kind` / `span` | Expression accessors. |
| `Expression::kind_mut` | Mutable expression kind accessor. |
| `ObjectField::new` | Construct object field. |
| `ObjectField::name` / `value` / `span` | Object field accessors. |
| `ObjectField::value_mut` | Mutable field value accessor. |
| `SwitchArm::new` | Construct switch arm. |
| `SwitchArm::pattern` / `value` / `span` | Switch arm accessors. |
| `SwitchArm::pattern_mut` / `value_mut` | Mutable switch arm accessors. |

### `syntax/lexer.rs`

| Function | Purpose |
|---|---|
| `lex` | Public lexer entry point. |
| `Lexer::new` | Initialize lexer state. |
| `Lexer::lex` | Produce token stream. |
| `Lexer::current` | Inspect current character. |
| `Lexer::advance` | Consume next character. |
| `Lexer::next_is` | Check one-character lookahead. |
| `Lexer::single` | Push a single-character token. |
| `Lexer::one_or_two` | Push single or doubled operator token. |
| `Lexer::push` | Push token with span. |
| `Lexer::skip_line_comment` | Skip `//` comment. |
| `Lexer::skip_block_comment` | Skip `/* ... */` comment or error. |
| `Lexer::lex_string` | Lex plain string. |
| `Lexer::lex_interpolated_string` | Lex formatted string. |
| `Lexer::scan_interpolation_end` | Find interpolation expression boundary. |
| `Lexer::skip_quoted_expression_string` | Skip nested strings while scanning interpolation. |
| `Lexer::lex_number` | Lex integer/decimal number token. |
| `Lexer::lex_identifier` | Lex identifier token. |
| `push_literal` | Push buffered interpolation literal part. |
| `shifted_tokens` | Shift spans for tokens from nested interpolation expressions. |
| `shifted_kind` | Shift spans inside token kinds. |
| `shifted_error` | Shift nested lexer diagnostics. |
| `is_identifier_start` | Check identifier first character. |
| `is_identifier_continue` | Check identifier continuation. |
| `error` | Create a lexer compile error. |

### `syntax/parser.rs`

| Function | Purpose |
|---|---|
| `parse` | Public parser entry point. |
| `Parser::new` | Initialize parser state. |
| `Parser::parse_program` | Parse full source. |
| `Parser::statement` | Parse one statement. |
| `Parser::test_block` | Parse `test` block. |
| `Parser::function_definition` | Parse top-level `fun`. |
| `Parser::is_computed_assignment` | Detect computed assignment target. |
| `Parser::computed_assignment` | Parse `[expr] = value`. |
| `Parser::assignment` | Parse static assignment and augmented assignment. |
| `Parser::expression` | Parse expression. |
| `Parser::conditional` | Parse trailing conditional expression. |
| `Parser::next_if_starts_block` | Identify block `if`. |
| `Parser::is_parenthesized_lambda` | Detect `(a, b) => expr`. |
| `Parser::binary` | Parse binary operators by precedence. |
| `Parser::unary` | Parse unary operators. |
| `Parser::postfix` | Parse calls, methods, subscripts, and required operator. |
| `Parser::primary` | Parse primary expressions. |
| `Parser::block_if_expression` | Parse block `if`. |
| `Parser::switch_expression` | Parse `switch`. |
| `Parser::switch_arm` | Parse one switch arm. |
| `Parser::array` | Parse array literal. |
| `Parser::object` | Parse object literal. |
| `Parser::arguments` | Parse call arguments. |
| `Parser::block` | Parse statement block. |
| `Parser::binary_operator` | Return operator and precedence. |
| `Parser::take_identifier` | Consume identifier and span. |
| `Parser::consume_identifier` | Conditionally consume named identifier. |
| `Parser::expect_identifier` | Require a named identifier. |
| `Parser::expect` | Require token kind. |
| `Parser::consume` | Conditionally consume token kind. |
| `Parser::at` | Check current token kind. |
| `Parser::identifier_at` | Inspect identifier lookahead. |
| `Parser::kind_at` | Inspect token-kind lookahead. |
| `Parser::peek` | Current token. |
| `Parser::previous` | Previous token. |
| `Parser::advance` | Consume current token. |
| `Parser::error_at` | Create parser compile error. |
| `same` | Token-kind equality helper. |
| `assignment_operator` | Map token kind to assignment operator. |
| `visibility_for` | Derive public/private visibility from name. |

### `validation.rs`

| Function | Purpose |
|---|---|
| `validate` | Validate AST and infer requirements. |
| `Scope::new` | Create root validation scope. |
| `Scope::function` | Create function validation scope. |
| `Scope::child` | Create child validation scope. |
| `Scope::contains_binding` | Check whether a binding is visible. |
| `Scope::bind_global` | Register global binding. |
| `Validator::statements` | Validate statement lists. |
| `Validator::expression` | Validate expressions. |
| `Validator::expressions` | Validate expression slices. |
| `Validator::validate_method` | Validate method-specific constraints. |
| `Validator::constrain` | Apply input constraints from expression usage. |
| `Validator::error` | Record a validation diagnostic. |
| `resolve_qualified_calls` | Rewrite unresolved qualified-looking calls after binding discovery. |
| `resolve_expression` | Recursive expression rewrite helper. |

## `html-extractor-runtime`

### `lib.rs`

| Function | Purpose |
|---|---|
| `Program::requirements/source/statements/functions/tests` | Runtime-facing read-only trait. |
| `CompiledProgram as Program` methods | Delegate trait calls to core accessors. |

### `engine.rs`

| Function | Purpose |
|---|---|
| `Engine::default` | Build default engine with reqwest client and default policy. |
| `Engine::fmt` | Debug-print engine configuration. |
| `Engine::new` | Create default engine. |
| `Engine::builder` | Start builder. |
| `Engine::execute` | Link and execute program. |
| `Engine::test` | Run program test blocks. |
| `Engine::run_test` | Execute one test block. |
| `TestReport::total` | Return total test count. |
| `TestReport::passed` | Return passed test count. |
| `EngineBuilder::host_function` | Register async host function. |
| `EngineBuilder::http_client` | Install custom HTTP client. |
| `EngineBuilder::http_policy` | Install HTTP policy. |
| `EngineBuilder::lua_module` | Queue Lua module source. |
| `EngineBuilder::build` | Load modules and create engine. |
| `is_reserved` | Check reserved host function name. |
| `function_key` | Format function requirement identity. |
| `is_html_like` | Validate HTML-like input constraints. |
| `ExecutionResult::output` | Borrow output object. |
| `ExecutionResult::warnings` | Borrow warnings. |
| `ExecutionResult::to_json` | Serialize output to JSON. |

### `evaluator.rs`

| Function | Purpose |
|---|---|
| `Flow::cast` | Convert flow payload type. |
| `Evaluator::new` | Initialize evaluator. |
| `Evaluator::into_warnings` | Consume evaluator warnings. |
| `Evaluator::push_warning` | Record warning. |
| `Evaluator::statements` | Evaluate statement list. |
| `Evaluator::block` | Evaluate block and final expression semantics. |
| `Evaluator::expression` | Evaluate expression AST. |
| `Evaluator::call_host` | Dispatch host/Lua functions. |
| `Evaluator::call_core` | Dispatch core functions such as `fetch` and context `extend`. |
| `Evaluator::call_function` | Dispatch script functions. |
| `Evaluator::apply_expression_statement_effect` | Apply side effects of expression statements such as `extend`. |
| `Evaluator::object` | Evaluate object literal. |
| `Evaluator::arguments` | Evaluate call arguments. |
| `append_interpolated_value` | Append scalar interpolation result. |
| `assert_eq_call` | Runtime helper for test equality assertions. |
| `extend_context` | Merge object fields into output context. |
| `fetch_request` | Convert DSL fetch arguments into `HttpRequest`. |
| `parse_number` | Convert AST number text to runtime value. |
| `unary` | Evaluate unary operators. |
| `binary` | Evaluate binary operators. |
| `boolean` | Require boolean value. |
| `arithmetic` | Evaluate arithmetic operations. |
| `number` | Require numeric value as `f64`. |
| `float_value` | Convert finite float to `Value::Number`. |
| `first_missing_path` | Find first missing nested required result. |
| `visit` | Recursive helper inside `first_missing_path`. |
| `required_operation` | Extract operation metadata for `!` diagnostics. |

### `functions/*.rs`

| File | Functions |
|---|---|
| `mod.rs` | `call` |
| `arr.rs` | `len`, `range`, `zip`, `first`, `last`, `take`, `unique`, `extend`, `integer`, `non_negative_integer`, `no_arguments` |
| `collections.rs` | `map`, `filter`, `predicate`, `map_block` |
| `css.rs` | `call` |
| `flatten.rs` | `call` |
| `guards.rs` | `nullcheck`, `predicate` |
| `null.rs` | `unwrap_or` |
| `object.rs` | `keys`, `log`, `values`, `has`, `no_arguments` |
| `regex.rs` | `call`, `scalar`, `capture` |
| `response.rs` | `status`, `json`, `no_arguments` |
| `sorting.rs` | `sorted`, `sort_by_key`, `sort_by`, `lambda`, `comparable_keys`, `comparable_error`, `Comparable::from_value`, `Comparable::kind`, `Comparable::compare` |
| `text.rs` | `text`, `trim`, `contains`, `starts_with`, `src`, `href`, `join`, `ensure_suffix`, `replace`, `attr`, `inner_html`, `html_text`, `html`, `map_array`, `node_text`, `node_inner_html`, `node_attr`, `no_arguments` |

### `html.rs`

| Function | Purpose |
|---|---|
| `HtmlDocument::parse` | Retain HTML source. |
| `HtmlDocument::source` | Borrow document source. |
| `HtmlNode::new` | Create node fragment and root tag. |
| `HtmlNode::fragment` | Borrow node fragment. |
| `HtmlNode::root_tag` | Borrow root tag. |

### `http.rs`

| Function | Purpose |
|---|---|
| `HttpRequest::get` | Build GET request. |
| `HttpRequest::post` | Build POST request. |
| `HttpRequest::url` | Borrow URL. |
| `HttpRequest::method` | Return method. |
| `HttpRequest::body` | Borrow request body. |
| `HttpRequest::content_type` | Borrow content type. |
| `HttpPolicy::default` | Default schemes and response-size limit. |
| `HttpPolicy::allow_schemes` | Replace allowed URL schemes. |
| `HttpPolicy::max_response_bytes` | Set response byte limit. |
| `HttpPolicy::validate_url` | Enforce scheme policy. |
| `HttpPolicy::validate_response` | Enforce response size. |
| `HttpClient::fetch` | Async HTTP trait method. |
| `ReqwestHttpClient::fetch` | Reqwest-backed fetch implementation. |
| `Response::new` | Create response. |
| `Response::status` | Return status code. |
| `Response::url` | Borrow final URL. |
| `Response::body` | Borrow body bytes. |
| `Response::text` | Cached UTF-8 conversion. |
| `Response::json` | Cached JSON parsing. |
| `Response::html` | Cached HTML parsing. |
| `Response::fmt` | Debug output. |
| `Response::eq` | Equality by status, URL, and body. |
| `json_to_value` | Convert serde JSON into runtime value. |

### `lua.rs`

| Function | Purpose |
|---|---|
| `LuaModule::load` | Sandbox and load a module. |
| `LuaModule::call` | Call a named export. |
| `sandbox` | Create restricted Lua state. |
| `to_lua` | Convert runtime value to Lua value. |
| `from_lua` | Convert Lua value to runtime value. |
| `table_to_value` | Convert Lua table to array/object. |
| `valid_identifier` | Validate module/export identifiers. |
| `module_error` | Wrap module load errors. |
| `lua_error` | Wrap Lua conversion/call errors. |

### `scope.rs`

| Function | Purpose |
|---|---|
| `Inputs::new` | Create empty inputs. |
| `Inputs::insert` | Add input binding with visibility. |
| `Inputs::get` | Borrow input value. |
| `Scope::root` | Create root runtime scope. |
| `Scope::child` | Create map-block child scope. |
| `Scope::function` | Create function frame. |
| `Scope::assign` | Assign with root/child/function rules. |
| `Scope::get` | Resolve binding. |
| `Scope::extend` | Merge object fields into extension output. |
| `Scope::output` | Build ordered public output object. |

### `serialize.rs`

| Function | Purpose |
|---|---|
| `to_json` | Runtime value to JSON entry point. |
| `convert` | Strict serializable conversion. |
| `convert_debug` | Debug conversion for runtime-only values. |
| `object` | Convert ordered object fields with paths. |

### `value.rs`

| Function | Purpose |
|---|---|
| `Object::new` | Create ordered object. |
| `Object::insert` | Insert or replace field. |
| `Object::get` | Borrow field. |
| `Object::keys` | Iterate keys. |
| `Object::iter` | Iterate fields. |
| `Object::len` | Field count. |
| `Object::is_empty` | Empty check. |
| `Object::into_iter` | Borrowed object iteration. |
| `ValueKind::fmt` | Display value kind. |
| `Value::kind` | Return value kind. |
| `Value::expect_array` | Borrow array or return type error. |
| `Value::into_array` | Consume array or return type error. |
| `Value::expect_string` | Borrow string or return type error. |
| `Value::expect_boolean` | Return bool or type error. |
| `Value::expect_number` | Borrow number or type error. |
| `Value::expect_object` | Borrow object or type error. |
| `Value::to_json` | Strict JSON conversion. |
| `Value::to_debug_json` | Debug JSON conversion. |
| `Value::from(bool/&str/String/integers)` | Primitive conversions. |

### `error.rs`

| Function | Purpose |
|---|---|
| `TypeError::new` | Build type error. |
| `TypeError::fmt` | Display type error. |
| `SerializationError::unsupported` | Build unsupported serialization error. |
| `SerializationError::path` | Borrow failing JSON path. |
| `SerializationError::kind` | Return unsupported kind. |
| `SerializationError::fmt` | Display serialization error. |
| `Diagnostic::new` | Build warning diagnostic. |
| `Diagnostic::message` | Borrow message. |
| `Diagnostic::span` | Return optional span. |
| `ExecutionError::new` | Build execution error. |
| `ExecutionError::message` | Borrow message. |
| `ExecutionError::span` | Return optional span. |
| `ExecutionError::fmt` | Display message. |
| `ExecutionError::from(TypeError)` | Convert type error. |

## `html-extractor-cli`

### `main.rs`

| Function | Purpose |
|---|---|
| `main` | CLI entry point. |
| `read_stdin` | Distinguish terminal stdin from redirected HTML. |
| `report_error` | Print startup error and return exit code. |

### `diagnostic.rs`

| Function | Purpose |
|---|---|
| `render` | Format file-level or source-located diagnostic. |

### `input.rs`

| Function | Purpose |
|---|---|
| `InputError::fmt` | Display input error. |
| `parse_assignment` | Parse `NAME=VALUE`. |
| `merge_variables` | Merge `--variables` and repeated `--variable`. |
| `json_to_value` | Convert CLI JSON into runtime value. |
| `build_inputs` | Build runtime inputs from stdin and variables. |

### `run.rs`

| Function | Purpose |
|---|---|
| `CliError::new` | Build CLI error. |
| `CliError::stdin` | Wrap stdin read error. |
| `CliError::exit_code` | Return process exit code. |
| `CliError::fmt` | Display CLI error. |
| `run_with_io` | Testable top-level runner. |
| `run_extract` | Compile and execute one extractor file. |
| `run_tests` | Compile and run test blocks for files. |
| `compile_source` | Compile with rendered diagnostics. |
| `engine_for` | Build engine and load sibling Lua modules. |
| `extractor_files` | Resolve test target to extractor files. |
| `collect_extractor_files` | Recursive extractor discovery. |
| `test_label` | Format test labels relative to current dir. |
| `CachedHttpClient::new` | Create test-cache client. |
| `CachedHttpClient::cache_path` | Derive cache path for request. |
| `CachedHttpClient::fetch` | Read/write cached HTTP responses. |
| `request_hash` | Stable FNV-style cache key. |
| `update_hash` | Hash byte slice into cache key. |
| `read_cached_response` | Decode cached response JSON. |
| `write_cached_response` | Encode cached response JSON. |
| `variable_error` | Map variable parse error to CLI status 2. |
| `stream_error` | Map stdout/stderr write error. |
| `file_error` | Format path-prefixed error message. |
