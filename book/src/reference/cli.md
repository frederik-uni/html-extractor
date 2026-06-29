# CLI

The CLI package is `crates/html-extractor-cli`. The binary name is `html-extractor`.

## Run an Extractor

```bash
html-extractor FILE [--variables JSON] [--variable NAME=VALUE ...]
```

If stdin is redirected, the CLI inserts a private `html` input as an `HtmlDocument`.

```bash
cat page.html | html-extractor page.extractor
```

If stdin is a terminal, `html` is not defined unless supplied through variables.

## Variables

```bash
html-extractor page.extractor \
  --variables '{"limit": 10, "category": "books"}' \
  --variable page=2 \
  --variable query=lamps
```

Rules:

- `--variables` must be a JSON object.
- `--variable NAME=VALUE` parses `VALUE` as JSON first, then falls back to a string.
- Later `--variable` entries override earlier JSON fields.
- All CLI-provided variables are private inputs.

## Lua Modules

When running a file, the CLI scans the extractor's parent directory for sibling `*.module.lua` files.

```text
examples/
├── prices.module.lua
└── products.extractor
```

`prices.module.lua` is loaded with namespace `prices`. Unique exports are callable unqualified; qualified calls use `prices.function_name(...)`.

## Test Command

```bash
html-extractor test PATH [--reset-cache]
```

`PATH` may be one extractor file or a directory. Directory mode recursively finds `.extractor` files, skipping `.cache`.

The command prints Rust-test-like output:

```text
running 3 tests

test examples/products.extractor ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 filtered out
```

HTTP responses are cached by request method, URL, content type, and body under:

```text
.cache/html-extractor/requests
```

`--reset-cache` removes that cache before running tests.

## Exit Codes

| Code | Meaning |
|---:|---|
| `0` | Success |
| `1` | Compilation, runtime, file, I/O, serialization, or test failure |
| `2` | Invalid CLI input such as malformed variables |

## Diagnostics

Compile and runtime errors are rendered with file, line, and column when a source span is available.

```text
page.extractor:2:10: error: missing required input `html`
```

Warnings are written to stderr. Successful JSON output is written to stdout.
