# Introduction

HTML Extractor is a Rust workspace for compiling and running small extractor scripts that turn HTML, HTTP responses, and host-provided values into ordered JSON-like output.

The project contains:

- A small extractor language with assignments, expressions, functions, loops, HTML selection, regex, HTTP fetches, Lua module calls, and collection helpers.
- A compiler crate that lexes, parses, validates, records source spans, and infers external requirements.
- A runtime crate that executes compiled programs against explicit inputs, host functions, Lua modules, and an HTTP client.
- A CLI crate for running extractor files, passing variables, loading sibling Lua modules, and running embedded extractor tests.
- Compile-time macros for embedding validated extractor programs in Rust binaries.
- Tree-sitter and Zed syntax support under `zed/`.

There is no repository-level `design.md` in this checkout. The design source is the dated design set under `docs/superpowers/specs/`, plus the implementation and tests.

## Workspace Layout

```text
html-extractor
├── src/lib.rs                         # public facade crate
├── crates/html-extractor-core         # compiler, AST, requirements, diagnostics
├── crates/html-extractor-runtime                     # execution engine and runtime values
├── crates/html-extractor-cli          # CLI binary
├── crates/html-extractor-macros       # procedural macros
├── examples                           # real extractor and Lua examples
├── docs/superpowers/specs             # design documents
├── docs/learning.md                   # existing language guide
└── zed                                # Zed extension and generated Tree-sitter grammar
```

## Mental Model

An extractor file is a sequential script. Public root assignments become the output. Private names beginning with `_` are available during execution but omitted from the final output. Runtime-only values such as HTML documents, HTML nodes, and HTTP responses must be converted to serializable values before export.

```extractor
_response = fetch("https://example.com/products")
_response.assert(r => r.status() == 200, "request failed")

products = _response.css@(".product")!.map {
    name = it.css(".name")!.text()
    href = it.css("a")!.href().nullcheck("missing product link")
}
```

Execution returns an ordered object. The CLI serializes it as pretty JSON.
