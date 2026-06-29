# Quick Start

Create an extractor:

```extractor
title = html.css("h1")!.text()
links = html.css@("a").href()
```

Run it with HTML from standard input:

```bash
printf '<main><h1>Hello</h1><a href="/a">A</a></main>' | \
  cargo run -p html-extractor-cli -- page.extractor
```

Expected output:

```json
{
  "title": "Hello",
  "links": ["/a"]
}
```

Pass private host inputs with JSON:

```bash
cargo run -p html-extractor-cli -- page.extractor \
  --variables '{"limit": 10, "category": "books"}' \
  --variable page=2
```

`--variables` must be a JSON object. Repeated `--variable NAME=VALUE` entries override fields from `--variables`; values are parsed as JSON first and fall back to strings.

Run extractor tests:

```bash
cargo run -p html-extractor-cli -- test examples --reset-cache
```

The `test` subcommand finds `.extractor` files in a file or directory, compiles them, runs their `test` blocks, and caches HTTP responses under `.cache/html-extractor/requests`.

Tests can provide inputs directly. A `test(...) { ... }` block runs the script first and exposes the output as `it`.

```extractor
title = html.css("h1")!.text()
count = 1
count += 2
count -= 1

test(html = "<h1>Hello</h1>") {
    assert(it.title == "Hello", "extracts title")
    assert(it.count == 2, "updates count")
}
```

Use the library:

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
