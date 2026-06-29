# Macros

The facade crate re-exports two procedural macros from `html-extractor-macros`.

## `extractor_program!`

Compiles an inline extractor source string while compiling the Rust crate.

```rust
let program = html_extractor::extractor_program!(
    r#"title = html.css("h1")!.text()"#
);
```

The macro accepts exactly one string literal. Invalid extractor source becomes Rust `compile_error!` output with extractor line and column information.

## `extractor_program_file!`

Reads a file relative to the consuming crate's `CARGO_MANIFEST_DIR`, compiles it, and embeds the validated representation.

```rust
static PROGRAM: std::sync::LazyLock<html_extractor::CompiledProgram> =
    std::sync::LazyLock::new(|| {
        html_extractor::extractor_program_file!("extractors/products.extractor")
    });
```

The expansion includes an `include_str!` dependency on the resolved file so Cargo recompiles when the extractor changes.

## Runtime Cost

Successful macro expansion serializes the validated AST and requirements using the core crate's hidden encode/decode helpers. A macro-generated `CompiledProgram` does not lex, parse, or validate when the application starts.

## Macro Functions in Source

| Rust function | File | Purpose |
|---|---|---|
| `extractor_program` | `crates/html-extractor-macros/src/lib.rs` | Parses one inline `LitStr` and delegates to `expand`. |
| `extractor_program_file` | `crates/html-extractor-macros/src/lib.rs` | Resolves a manifest-relative path, reads it, and delegates to `expand`. |
| `expand` | `crates/html-extractor-macros/src/lib.rs` | Runs `compile`, converts diagnostics, embeds encoded bytes and tracked source. |
