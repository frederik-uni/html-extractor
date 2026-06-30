# HTML Extractor

HTML Extractor is a Rust workspace for compiling and running small extractor
scripts that turn HTML, HTTP responses, and host-provided values into ordered
JSON output.

## Quick Start

Create an extractor file:

```extractor
// page.extractor
title = html.css("h1")!.text()
links = html.css@("a").href()
```

Run it with HTML from standard input:

```bash
printf '<main><h1>Hello</h1><a href="/docs">Docs</a></main>' | \
  cargo run -p html-extractor-cli -- page.extractor
```

Output:

```json
{
  "title": "Hello",
  "links": ["/docs"]
}
```

When installed as a binary, use the shorter command:

```bash
printf '<h1>Hello</h1>' | html-extractor page.extractor
```

## Passing Inputs

Extractor scripts can read host-provided inputs by name. Use `--variables` for
a JSON object and `--variable NAME=VALUE` for individual overrides.

```extractor
// search.extractor
offset = (page - 1) * limit
url = f"https://example.com/search?q={query}&offset={offset}"
```

```bash
cargo run -p html-extractor-cli -- search.extractor \
  --variables '{"query":"rust","limit":10,"page":1}' \
  --variable page=3
```

Repeated `--variable` flags override fields from `--variables`. Values are
parsed as JSON first and fall back to strings.

## Extracting Lists

Use `css@` to select many nodes and `map { ... }` to build one output object
per item.

```extractor
products = html.css@(".product")!.map {
    name = it.css(".name")!.text()
    _raw_price = it.css(".price")!.text()
    price = _raw_price.regex("[0-9]+(?:\\.[0-9]+)?")!
    href = it.css("a")!.href()
}
```

Private bindings start with `_`, so `_raw_price` can be used while building the
result without appearing in the final JSON.

## Tests

Extractor files can include `test` blocks. A test runs the script first and
exposes the output as `it`.

```extractor
title = html.css("h1")!.text()
count = 1
count += 2

test(html = "<h1>Hello</h1>") {
    assertEq("Hello", it.title, "extracts title")
    assertEq(3, it.count, "updates count")
}
```

Run tests for one file or every `.extractor` file in a directory:

```bash
cargo run -p html-extractor-cli -- test page.extractor
cargo run -p html-extractor-cli -- test examples --reset-cache
```

The test runner caches HTTP responses under `.cache/html-extractor/requests`.
Use `--reset-cache` to clear that cache before running.

## Formatting

Format an extractor to stdout:

```bash
cargo run -p html-extractor-cli -- fmt page.extractor
```

Rewrite it in place:

```bash
cargo run -p html-extractor-cli -- fmt --write page.extractor
```

## Rust API

Compile and run a program at runtime:

```rust
use html_extractor::compile;
use html_extractor_runtime::{Engine, HtmlDocument, Inputs, Value, Visibility};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let program = compile("title = html.css(\"h1\")!.text()")?;
let inputs = Inputs::new().insert(
    "html",
    Value::HtmlDocument(HtmlDocument::parse("<h1>Hello</h1>")),
    Visibility::Private,
);

let result = Engine::new().execute(&program, inputs).await?;
assert_eq!(result.to_json()?, serde_json::json!({"title": "Hello"}));
# Ok(())
# }
```

Embed a validated program at Rust compile time:

```rust
let program = html_extractor::extractor_program!(
    r#"title = html.css("h1")!.text()"#
);
```

Load an extractor file at compile time:

```rust
let program = html_extractor::extractor_program_file!("page.extractor");
```

