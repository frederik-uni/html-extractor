# Language Guide

## Scripts, Bindings, and Output

An extractor is a sequence of statements. Statements execute top to bottom. Public root bindings are exported; private bindings start with `_` and are omitted.

```extractor
name = "Keyboard"
_raw_price = "EUR 49"
price = _raw_price.regex("[0-9]+")!
```

Reassigning a name replaces the previous value. Host inputs can also be replaced by script assignments.

```extractor
value = 1
_value = 2
value = 3
```

The output contains `value` and omits `_value`.

Assignments also support `+=` and `-=` for updating an existing numeric value.

```extractor
count = 10
count += 5
count -= 3
```

## Values

Supported serializable values are null, booleans, numbers, strings, arrays, and ordered objects.

```extractor
nothing = null
available = true
quantity = 3
price = 12.50
tags = ["lighting", "office"]
product = {name: "Desk lamp", price: 12.50}
```

Runtime-only values are also supported internally:

- `HtmlDocument`
- `HtmlNode`
- `Response`

Keep runtime-only values private unless they are converted before output.

## Comments

```extractor
// line comment

/*
   block comment
*/
title = html.css("h1")!.text()
```

## Operators

The language supports arithmetic, equality, comparisons, boolean operators, unary negation, and logical not.

```extractor
same = 2 == 2
different = "a" != "b"
total = 10 + 5 * 2
remaining = 7 % 3
valid = total > 0 and not false
```

Operations are type-strict. Incompatible values produce runtime errors rather than implicit casts.

## Access

Arrays and objects support bracket access. Dot access is parser sugar for bracket access: identifiers become string keys and integer segments become indexes.

```extractor
name = item["name"]
same = item.name
first_tag = item.tags.0
```

Missing object fields and out-of-range array indexes return `null`.

## Conditional Expressions

Trailing condition:

```extractor
label = "adult" if age >= 18 else "minor"
```

Block form:

```extractor
label = if age >= 18 {
    "adult"
} else {
    "minor"
}
```

An `if` used as a statement may omit `else`. A value-producing `if` requires `else`.

## Switch Expressions

```extractor
status_text = switch(status) {
    200 => "ok",
    404 => "missing",
    code => f"unexpected {code}",
}
```

Arms are tested top to bottom. The final arm must be exhaustive, either `_` or a binding.

## Strings and Interpolation

Plain strings do not interpolate.

```extractor
plain = "query={query}"
```

Formatted strings use `f"..."` and Python-style braces.

```extractor
offset = (page - 1) * limit
url = f"https://example.com/search?q={query}&offset={offset}"
literal = f"braces={{value}}"
```

Interpolation supports scalar values: strings, numbers, booleans, and null. Arrays, objects, HTML values, and responses are rejected.

## Functions

Top-level script functions use `fun`.

```extractor
counter = 0

fun add(left, right) {
    counter = counter + 1
    left + right
}

total = add(2, 3)
```

The final expression is the implicit return. `return expression` exits immediately. A function body without a final expression returns `null`.

Function calls resolve in this order:

1. Reserved core or built-in function.
2. Script function.
3. Host or Lua function.

## Loops and Control Flow

```extractor
count = 0

while count < 10 {
    count = count + 1
}

loop {
    count = count + 1
    if count == 25 {
        break
    }
}
```

`break` exits the nearest loop. `return` exits the containing function.

## Pipelines and Method Calls

Method syntax inserts the receiver as the first argument.

```extractor
title = html.css("h1")!.text()
same = text(css(html, "h1")!)
```

This normalization also applies in lambdas:

```extractor
urls = html.css@("a").map(node => href(node))
```

## Map Blocks and Lambdas

Map blocks create one output object per input item. The current item is available as private `it`.

```extractor
products = html.css@(".product")!.map {
    name = it.css(".name")!.text()
    _raw_price = it.css(".price")!.text()
    price = _raw_price.regex("[0-9]+(?:\\.[0-9]+)?")!
}
```

Expression lambdas are used by collection helpers:

```extractor
doubled = [1, 2, 3].map(number => number * 2)
positive = [-2, 0, 3, 8].filter(number => number > 0)
```

## Computed Assignment Targets

A bracketed assignment target evaluates to the real binding name.

```extractor
_key = "title"
[_key] = "Example"
```

The resolved name controls visibility: `_secret` is private, while `title` is public.

## Required Results

Postfix `!` is available after `css`, `css@`, `regex`, and `regex@`.

```extractor
title = html.css("h1")!.text()
ids = html.css@("[data-id]")!.attr("data-id")
id = url.regex("id=(\\d+)", 1)!
```

It fails on missing single results, empty multi-results, and missing captures in required regex arrays.

## Test Blocks

Compiled programs can contain tests. The runtime exposes them through `Engine::test`, and the CLI runs them with `html-extractor test`.

```extractor
value = 1

test {
    assert(value == 1, "value should be one")
}
```

Tests may define inputs in parentheses. A test with inputs runs the script first; the script output is available as `it` inside the test body. Input visibility follows normal binding rules, so names starting with `_` are private.

```extractor
title = html.css("h1")!.text()

test(html = "<h1>Hello</h1>", _expected = "Hello") {
    assert(it.title == _expected, "extracts title")
}
```

A bare `test { ... }` block does not run the script before its assertions. Use it for testing helper functions or standalone expressions.
