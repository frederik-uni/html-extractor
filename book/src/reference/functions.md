# Function Reference

Functions can be called as standalone functions or as methods. Method calls pass the value on the left as the first argument.

```extractor
title = html.css("h1").text()
same = text(css(html, "h1"))
```

## Core and Extension Functions

| Function | Arguments | Returns | Example |
|---|---:|---|---|
| `fetch` | URL string, optional options object | `Response` | `_response = fetch("https://example.com")` |
| `extend` | Object when used as core form | Object | `extend({source: "catalogue"})` |
| `function(...)` | Host or Lua-defined | Any supported value | `price = parse_price(_raw)` |
| `module.function(...)` | Host or Lua-defined | Any supported value | `price = prices.parse_price(_raw)` |

`fetch(url)` sends GET. `fetch(url, {method: "POST", json: value})` sends POST JSON. Current request objects also support headers internally through `HttpRequest`, but the DSL options shape is intentionally narrow.

## Built-In Functions

The compiler index in `crates/html-extractor-core/src/functions.rs` lists every reserved built-in. The runtime dispatch table in `crates/html-extractor-runtime/src/functions/mod.rs` implements the value functions.

| Name | Receiver | Written arguments | Returns | Example |
|---|---|---|---|---|
| `css` | HTML document, HTML node, response, or arrays of those | selector string | first `HtmlNode` or `null` | `title = html.css("h1")` |
| `css@` | HTML document, HTML node, response, or arrays of those | selector string | array of `HtmlNode` | `cards = html.css@(".card")` |
| `text` | string, HTML document, HTML node, response, or arrays | none | string or shape-preserving array | `title = html.css("h1")!.text()` |
| `inner_html` | HTML node or arrays of nodes | none | string or array | `markup = html.css("article")!.inner_html()` |
| `html_text` | HTML node or arrays of nodes | none | string or array | `text = html.css("article")!.html_text()` |
| `attr` | HTML node or arrays of nodes | attribute name | string, `null`, or array | `href = html.css("a")!.attr("href")` |
| `href` | HTML node or arrays of nodes | none | `href` attribute | `links = html.css@("a").href()` |
| `src` | HTML node or arrays of nodes | none | `src` attribute | `images = html.css@("img").src()` |
| `html` | string, HTML document, response, or arrays | none | `HtmlDocument` or array | `_doc = raw_html.html()` |
| `regex` | string, response, or arrays | pattern, optional capture | string or `null` | `id = url.regex("id=(\\d+)", 1)` |
| `regex@` | string, response, or arrays | pattern, optional capture | array | `ids = text.regex@("id=(\\d+)", 1)` |
| `flatten` | array | none | one-level flattened array | `images = galleries.css@("img").flatten()` |
| `map` | array | one lambda | array | `doubled = [1,2].map(x => x * 2)` |
| `map` block | array | assignment block | array of objects | `items = nodes.map { name = it.text() }` |
| `filter` | array | one boolean lambda | array | `positive = nums.filter(n => n > 0)` |
| `any` | array | one boolean lambda | boolean | `has_big = nums.any(n => n > 10)` |
| `all` | array | one boolean lambda | boolean | `all_big = nums.all(n => n > 10)` |
| `find` | array | one boolean lambda | value or `null` | `hit = nums.find(n => n == 3)` |
| `len` | string or array in current implementation | none | number | `count = items.len()` |
| `range` | start number | exclusive end number | array of numbers | `pages = 1.range(5)` |
| `zip` | first array | one or more arrays | array of rows | `pairs = [1,2].zip(["a","b"])` |
| `first` | array | none | first value or `null` | `first = items.first()` |
| `last` | array | none | last value or `null` | `last = items.last()` |
| `take` | array | non-negative count | array | `top = items.take(3)` |
| `unique` | array | none | array | `tags = tags.unique()` |
| `extend` | array | array | concatenated array | `both = first.extend(second)` |
| `keys` | object | none | array of strings | `names = object.keys()` |
| `values` | object | none | array | `vals = object.values()` |
| `has` | object | string key | boolean | `exists = object.has("title")` |
| `log` | any value | none | original value | `debugged = value.log()` |
| `sorted` | homogeneous string or number array | none | sorted array | `names = names.sorted()` |
| `sort_by_key` | array | one lambda returning string or number | sorted array | `items = items.sort_by_key(x => x.rank)` |
| `sort_by` | array | two-parameter comparator lambda | sorted array | `items = items.sort_by((a,b) => b.rank - a.rank)` |
| `nullcheck` | any value | message string | original value or error | `url = href.nullcheck("missing href")` |
| `assert` | value or boolean | lambda and message, or message for boolean receiver | original value or boolean | `price.assert(p => p > 0, "bad price")` |
| `warn` | value or boolean | lambda and message, or message for boolean receiver | original value or boolean | `names.warn(v => v != [], "no names")` |
| `status` | response | none | status number | `code = _response.status()` |
| `json` | response, or string in current implementation | none | extractor value | `data = _response.json()` |
| `trim` | string or arrays of strings | none | string or array | `name = raw.trim()` |
| `replace` | string | search, replacement | string | `clean = raw.replace(",", ".")` |
| `contains` | string in current implementation | search string | boolean | `ok = title.contains("Sale")` |
| `starts_with` | string | prefix string | boolean | `absolute = url.starts_with("https://")` |
| `unwrap_or` | any value | fallback value | original or fallback | `name = maybe.unwrap_or("Unknown")` |
| `ensure_suffix` | string | suffix string | string | `url = path.ensure_suffix("/")` |
| `join` | string in current implementation | string or number | appended string | `key = "page=".join(page)` |

## Shape-Preserving Array Behavior

`css`, `css@`, `text`, `attr`, `href`, `src`, `html`, `inner_html`, `html_text`, `regex`, `regex@`, and `trim` recurse over arrays where implemented. This keeps nested page structure intact until `flatten()` is called.

```extractor
_images_by_gallery = html.css@(".gallery").css@("img")
image_urls = _images_by_gallery.attr("src")
flat_urls = _images_by_gallery.flatten().attr("src")
```

## Required Operator

`!` is syntax, not a function.

```extractor
title = html.css("h1")!.text()
ids = text.regex@("id=(\\d+)", 1)!
```

It can follow `css`, `css@`, `regex`, and `regex@`.
