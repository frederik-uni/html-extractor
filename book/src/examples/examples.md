# Examples


## Minimal HTML

```extractor
title = html.css("h1")!.text()
links = html.css@("a").href()
```

## Products

```extractor
products = html.css@(".product")!.map {
    name = it.css(".name")!.text()
    link = it.css("a")!.href().nullcheck("product link is missing")
    _raw = it.css(".price")!.text()
    price = _raw.regex("[0-9]+(?:\\.[0-9]+)?")!
}
```

## API Fetch

```extractor
_response = fetch("https://example.com/api/products")
_response.assert(r => r.status() == 200, "request failed")
data = _response.json()
```

## POST JSON

```extractor
_response = fetch("https://api.example.com/search", {
    method: "POST",
    json: {
        query: query,
        page: page,
    }
})
results = _response.json()
```

## Lua Helper

`prices.module.lua`:

```lua
local function parse_price(text)
    local normalized = string.gsub(text, ",", ".")
    return tonumber(string.match(normalized, "%d+%.?%d*"))
end

return {
    parse_price = parse_price,
}
```

Extractor:

```extractor
_raw = html.css(".price")!.text()
price = prices.parse_price(_raw)
```

## Computed Keys

```extractor
fields = [{key: "title", value: "Example"}, {key: "kind", value: "book"}]

result = fields.map {
    _key = it.key
    [_key] = it.value
}
```

## Embedded Test

```extractor
fun normalize(text) {
    text.trim().replace(",", ".")
}

test {
    assert(normalize(" 12,50 ") == "12.50", "normalizes decimal comma")
}
```
