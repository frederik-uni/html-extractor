use html_extractor_core::{TokenKind, lex};

macro_rules! design_example {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            let tokens = lex($source).unwrap_or_else(|error| {
                panic!("documented example must lex successfully: {error:?}")
            });
            assert_eq!(
                tokens.last().map(|token| token.kind()),
                Some(&TokenKind::Eof)
            );
        }
    };
}

design_example!(public_assignment, "name = expression");
design_example!(private_assignment, "_name = expression");
design_example!(
    private_binding_read,
    "_raw = \"12.50\"\nprice = parse_price(_raw)"
);
design_example!(visibility_reassignment, "value = 1\n_value = 2\nvalue = 3");
design_example!(
    null_boolean_number_literals,
    "values = [null, true, false, 0, 12.50]"
);
design_example!(
    ordered_object_literal,
    "product = { name: \"A\", price: 12.50 }"
);
design_example!(
    outer_binding_in_map_scope,
    r#"
    currency = "EUR"
    products = html.css@(".product").map {
        _text = it.css(".price").text()
        price = parse_price(_text, currency)
    }
"#
);
design_example!(
    expression_lambda_filter,
    "positive = numbers.filter(x => x > 0)"
);
design_example!(
    expression_lambda_assert,
    "value.assert(x => x > 0, \"value must be positive\")"
);
design_example!(single_css_selection, "first = html.css(\".product\")");
design_example!(multiple_css_selection, "all = html.css@(\".product\")");
design_example!(
    nested_selection_and_flatten,
    "images = html.css@(\".gallery\").css@(\"img\").flatten()"
);
design_example!(
    required_single_selection,
    "title = html.css(\"h1\")!.text()"
);
design_example!(
    required_multi_selection,
    "products = html.css@(\".product\")!"
);
design_example!(required_regex_match, r#"id = text.regex("id=(\\d+)")!"#);
design_example!(regex_numbered_group, r#"id = text.regex("id=(\\d+)", 1)"#);
design_example!(
    nullcheck_pipeline,
    r#"
    price
        .nullcheck("price is required")
        .assert(x => x > 0, "price must be positive")
"#
);
design_example!(
    warning_pipeline,
    "stock.warn(x => x >= 0, \"negative stock reported\")"
);
design_example!(core_fetch, "_response = fetch(url)");
design_example!(
    response_operations,
    r#"
    status = response.status()
    data = response.json()
    _document = response.html()
    body = response.text()
"#
);
design_example!(
    implicit_response_html_conversion,
    "title = response.css(\"h1\")!.text()"
);
design_example!(
    implicit_response_regex_conversion,
    r#"token = response.regex("token=(\\w+)")!"#
);
design_example!(unqualified_external_function, "price = parse_price(raw)");
design_example!(
    qualified_external_function,
    "price = europarser.parse_price(raw)"
);
design_example!(
    line_and_block_comments,
    r#"
    // line comment
    /* block
       comment */
    name = "comments are ignored"
"#
);
design_example!(
    self_fetching_script,
    r#"
    _html = fetch("https://example.com").html()
    title = _html.css("h1")!.text()
"#
);
design_example!(
    compiled_program_inline_macro_source,
    r#"
    title = html.css("h1")!.text()
"#
);
design_example!(
    complete_product_extractor,
    r#"
    /*
     * This script works without piped input because it fetches its own page.
     */
    _response = fetch("https://example.com/products")
    _response.assert(r => r.status() == 200, "product request failed")

    products = _response.css@(".product")!.map {
        name = it.css(".name")!.text()
        link = it.css("a")!.attr("href").nullcheck("product link is missing")

        _raw_price = it.css(".price")!.text()
        price = parse_price(_raw_price)
            .nullcheck("price could not be parsed")
            .assert(x => x > 0, "price must be positive")

        stock = it.css(".stock").text()
            .warn(x => x != "", "stock text is empty")
    }
"#
);
