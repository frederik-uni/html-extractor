use html_extractor_core::compile;
use html_extractor_runtime::{Engine, Inputs, Object, Value, Visibility};

#[tokio::test]
async fn parses_a_product_page_and_equals_the_complete_expected_output() {
    let program = compile(
        r#"
        _html = source.html()
        page_title = _html.css("title")!.text()
        products = _html.css@(".product")!.map {
            id = it.attr("data-id").nullcheck("product id is missing")
            name = it.css("h2")!.text()
            price = it.css(".price")!.text().regex("[0-9]+\\.[0-9]+")!
            link = it.css("a")!.attr("href").nullcheck("product link is missing")
            tags = it.css@(".tag").text()
        }
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "source",
        Value::from(include_str!("fixtures/products.html")),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    let mut first_product = Object::new();
    first_product.insert("id", Value::from("sku-101"));
    first_product.insert("name", Value::from("Canvas Bag"));
    first_product.insert("price", Value::from("24.95"));
    first_product.insert("link", Value::from("/products/canvas-bag"));
    first_product.insert(
        "tags",
        Value::Array(vec![Value::from("travel"), Value::from("canvas")]),
    );

    let mut second_product = Object::new();
    second_product.insert("id", Value::from("sku-102"));
    second_product.insert("name", Value::from("Steel Bottle"));
    second_product.insert("price", Value::from("18.50"));
    second_product.insert("link", Value::from("/products/steel-bottle"));
    second_product.insert("tags", Value::Array(vec![Value::from("travel")]));

    let mut expected = Object::new();
    expected.insert("page_title", Value::from("Summer catalogue"));
    expected.insert(
        "products",
        Value::Array(vec![
            Value::Object(first_product),
            Value::Object(second_product),
        ]),
    );

    assert_eq!(result.output(), &expected);
    let expected_json: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/products.expected.json")).unwrap();
    assert_eq!(result.to_json().unwrap(), expected_json);
}

#[tokio::test]
async fn parses_nested_html_flattens_it_and_equals_the_expected_object() {
    let program = compile(
        r#"
        _html = source.html()
        _nested_articles = _html.css@(".group").css@("article")
        article_count = 3
        articles = _nested_articles.flatten().map {
            key = it.attr("data-key").nullcheck("article key is missing")
            title = it.css("h2")!.text()
            body = it.css("p")!.text()
        }
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "source",
        Value::from(include_str!("fixtures/articles.html")),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    let expected_articles = [
        ("post-7", "First post", "Alpha text"),
        ("post-8", "Second post", "Beta text"),
        ("post-9", "Third post", "Gamma text"),
    ]
    .into_iter()
    .map(|(key, title, body)| {
        let mut article = Object::new();
        article.insert("key", Value::from(key));
        article.insert("title", Value::from(title));
        article.insert("body", Value::from(body));
        Value::Object(article)
    })
    .collect();

    let mut expected = Object::new();
    expected.insert("article_count", Value::from(3));
    expected.insert("articles", Value::Array(expected_articles));

    assert_eq!(result.output(), &expected);

    let expected_json: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/articles.expected.json")).unwrap();
    assert_eq!(result.to_json().unwrap(), expected_json);
}
