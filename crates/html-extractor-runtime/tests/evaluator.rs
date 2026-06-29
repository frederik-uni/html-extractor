use html_extractor_core::compile;
use html_extractor_runtime::{Engine, Inputs, Object, Value, Visibility};

fn object(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    let mut object = Object::new();
    for (name, value) in fields {
        object.insert(name, value);
    }
    Value::Object(object)
}

#[tokio::test]
async fn script_functions_return_values_and_mutate_existing_globals() {
    let program = compile(
        r#"
        counter = 0
        fun add(left, right) {
            counter = counter + 1
            temporary = left + right
            temporary
        }
        fun describe(value) {
            if value < 0 { return "negative" }
            "non-negative"
        }
        fun empty() {}
        fun non_negative(value) {
            return null if value < 0 else value
        }
        sum = add(1, 2)
        label = describe(-1)
        nothing = empty()
        rejected = non_negative(-1)
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("counter"), Some(&Value::from(1)));
    assert_eq!(result.output().get("sum"), Some(&Value::from(3)));
    assert_eq!(result.output().get("label"), Some(&Value::from("negative")));
    assert_eq!(result.output().get("nothing"), Some(&Value::Null));
    assert_eq!(result.output().get("rejected"), Some(&Value::Null));
    assert!(result.output().get("temporary").is_none());
}

#[tokio::test]
async fn script_functions_support_recursion_and_exact_arity() {
    let recursive = compile(
        r#"
        result = factorial(5)
        fun factorial(value) {
            if value == 0 { 1 } else { value * factorial(value - 1) }
        }
        "#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&recursive, Inputs::new())
        .await
        .unwrap();
    assert_eq!(result.output().get("result"), Some(&Value::from(120)));

    let wrong_arity = compile("fun identity(value) { value } result = identity() ").unwrap();
    let error = Engine::new()
        .execute(&wrong_arity, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("expects 1 argument"));
}

#[tokio::test]
async fn block_if_while_loop_break_and_return_compose() {
    let program = compile(
        r#"
        count = 0
        while count < 2 { count = count + 1 }
        while (count < 4) { count = count + 1 }
        loop {
            count = count + 1
            if count == 6 { break }
        }
        fun locate() {
            loop {
                while true { return "found" }
            }
        }
        label = if count == 6 { locate() } else { "missing" }
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(result.output().get("count"), Some(&Value::from(6)));
    assert_eq!(result.output().get("label"), Some(&Value::from("found")));
}

#[tokio::test]
async fn loop_expression_statement_extend_mutates_receiver_binding() {
    let program = compile(
        r#"
        items = []
        page = 1
        loop {
            items.extend([page])
            page += 1
            if page > 3 { break }
        }
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(
        result.output().get("items"),
        Some(&Value::Array(vec![1.into(), 2.into(), 3.into()]))
    );
    assert_eq!(result.output().get("page"), Some(&Value::from(4)));
}

#[tokio::test]
async fn augmented_assignment_adds_and_subtracts_existing_values() {
    let program = compile(
        r#"
        count = 10
        count += 5
        count -= 3
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("count"), Some(&Value::from(12)));
}

#[tokio::test]
async fn block_if_and_while_require_boolean_conditions() {
    for source in ["if 1 { value = true }", "while 1 { break }"] {
        let program = compile(source).unwrap();
        let error = Engine::new()
            .execute(&program, Inputs::new())
            .await
            .unwrap_err();
        assert!(error.message().contains("expected boolean"));
    }
}

#[tokio::test]
async fn evaluates_dollar_free_private_it_and_python_interpolation() {
    let program = compile(
        r#"
        _hidden = 4
        objects = [1, 2].map {
            value = it
            _copy = it
        }
        message = f"count={objects[1]["value"]} dollar=$ braces={{ok}}"
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert!(result.output().get("_hidden").is_none());
    assert_eq!(
        result.output().get("message"),
        Some(&Value::from("count=2 dollar=$ braces={ok}"))
    );
    let objects = result
        .output()
        .get("objects")
        .unwrap()
        .expect_array()
        .unwrap();
    let second = objects[1].expect_object().unwrap();
    assert_eq!(second.get("value"), Some(&Value::from(2)));
    assert!(second.get("_copy").is_none());
    assert!(second.get("it").is_none());
}

#[tokio::test]
async fn computed_assignment_uses_generated_name_and_visibility() {
    let program = compile(
        r#"
        objects = items.map {
            _key = it.key
            _value = it.value
            [_key] = _value
        }
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "items",
        Value::Array(vec![
            object([
                ("key", Value::from("title")),
                ("value", Value::from("Example")),
            ]),
            object([("key", Value::from("_hidden")), ("value", Value::from(1))]),
        ]),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();
    let objects = result
        .output()
        .get("objects")
        .unwrap()
        .expect_array()
        .unwrap();
    assert_eq!(
        objects[0].expect_object().unwrap().get("title"),
        Some(&Value::from("Example"))
    );
    assert!(objects[1].expect_object().unwrap().get("_hidden").is_none());
}

#[tokio::test]
async fn executes_assignments_pipelines_and_visibility_replacement() {
    let program = compile(
        r#"
        _hidden = 1
        values = [[1], [2, 3]].flatten()
        hidden = 4
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(
        result.output().get("values"),
        Some(&Value::Array(vec![1.into(), 2.into(), 3.into()]))
    );
    assert_eq!(result.output().get("hidden"), Some(&Value::from(4)));
}

#[tokio::test]
async fn maps_filters_and_builds_objects_from_functional_blocks() {
    let program = compile(
        r#"
        doubled = numbers.map(x => x * 2)
        positive = doubled.filter(x => x > 0)
        objects = positive.map { value = it }
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "numbers",
        Value::Array(vec![(-1_i64).into(), 2.into()]),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("positive"),
        Some(&Value::Array(vec![Value::from(4)]))
    );
    let objects = result
        .output()
        .get("objects")
        .unwrap()
        .expect_array()
        .unwrap();
    assert_eq!(
        objects[0].expect_object().unwrap().get("value"),
        Some(&Value::from(4))
    );
}

#[tokio::test]
async fn lambda_shorthand_uses_it_as_the_implicit_parameter() {
    let program = compile(
        r#"
        urls = items.map(it.url)
        filtered = items.filter(it.score > 1).map(it.url)
        ordered = items.sort_by_key(it.order).map(it.url)
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "items",
        Value::Array(vec![
            object([
                ("url", Value::from("second")),
                ("score", Value::from(2)),
                ("order", Value::from(2)),
            ]),
            object([
                ("url", Value::from("first")),
                ("score", Value::from(1)),
                ("order", Value::from(1)),
            ]),
        ]),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("urls"),
        Some(&Value::Array(vec!["second".into(), "first".into()]))
    );
    assert_eq!(
        result.output().get("filtered"),
        Some(&Value::Array(vec!["second".into()]))
    );
    assert_eq!(
        result.output().get("ordered"),
        Some(&Value::Array(vec!["first".into(), "second".into()]))
    );
}

#[tokio::test]
async fn collection_predicates_support_both_call_forms_and_short_circuit() {
    let program = compile(
        r#"
        any_method = [1, 2, 3].any(x => x > 2)
        any_function = any([1, 2, 3], x => x > 2)
        all_method = [1, 2, 3].all(x => x > 0)
        all_function = all([1, 2, 3], x => x > 0)
        found_method = [1, 2, 3].find(x => x > 1)
        found_function = find([1, 2, 3], x => x > 1)
        not_found = [1].find(x => x > 2)
        empty_any = [].any(x => true)
        empty_all = [].all(x => false)
        short_any = [1, 0].any(x => x == 1 || 1 / x > 0)
        short_all = [0].all(x => false && 1 / x > 0)
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(result.output().get("any_method"), Some(&Value::from(true)));
    assert_eq!(
        result.output().get("any_function"),
        Some(&Value::from(true))
    );
    assert_eq!(result.output().get("all_method"), Some(&Value::from(true)));
    assert_eq!(
        result.output().get("all_function"),
        Some(&Value::from(true))
    );
    assert_eq!(result.output().get("found_method"), Some(&Value::from(2)));
    assert_eq!(result.output().get("found_function"), Some(&Value::from(2)));
    assert_eq!(result.output().get("not_found"), Some(&Value::Null));
    assert_eq!(result.output().get("empty_any"), Some(&Value::from(false)));
    assert_eq!(result.output().get("empty_all"), Some(&Value::from(true)));
    assert_eq!(result.output().get("short_any"), Some(&Value::from(true)));
    assert_eq!(result.output().get("short_all"), Some(&Value::from(false)));
}

#[tokio::test]
async fn collection_predicates_and_computed_assignments_validate_results() {
    let predicate = compile("value = [1].any(x => x)").unwrap();
    let error = Engine::new()
        .execute(&predicate, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("expected boolean"));

    let computed = compile("_key = 1 [_key] = true").unwrap();
    let error = Engine::new()
        .execute(&computed, Inputs::new())
        .await
        .unwrap_err();
    assert!(
        error
            .message()
            .contains("computed assignment name must be a string")
    );
}

#[tokio::test]
async fn runtime_functions_work_as_standalone_and_method_calls() {
    let program = compile(
        r#"
        method = "  hello world  ".trim().contains("hello")
        standalone = contains(trim("  hello world  "), "hello")
        encoded_method = "a b+c&d/e?".urlencode()
        encoded_function = urlencode("a b+c&d/e?")
        method_len = [1, 2, 3].len()
        standalone_len = len([1, 2, 3])
        direct_map = map([1, 2, 3], n => n * 2)
        direct_filter = filter([1, 2, 3], n => n > 1)
        direct_css_multi = len(css@(html("<a>A</a><a>B</a>"), "a"))
        direct_required = text(css(html("<h1>Title</h1>"), "h1")!)
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("method"), Some(&Value::from(true)));
    assert_eq!(result.output().get("standalone"), Some(&Value::from(true)));
    assert_eq!(
        result.output().get("encoded_method"),
        Some(&Value::from("a+b%2Bc%26d%2Fe%3F"))
    );
    assert_eq!(
        result.output().get("encoded_function"),
        Some(&Value::from("a+b%2Bc%26d%2Fe%3F"))
    );
    assert_eq!(result.output().get("method_len"), Some(&Value::from(3)));
    assert_eq!(result.output().get("standalone_len"), Some(&Value::from(3)));
    assert_eq!(
        result.output().get("direct_map"),
        Some(&Value::Array(vec![2.into(), 4.into(), 6.into()]))
    );
    assert_eq!(
        result.output().get("direct_filter"),
        Some(&Value::Array(vec![2.into(), 3.into()]))
    );
    assert_eq!(
        result.output().get("direct_css_multi"),
        Some(&Value::from(2))
    );
    assert_eq!(
        result.output().get("direct_required"),
        Some(&Value::from("Title"))
    );
}

#[tokio::test]
async fn global_runtime_functions_work_inside_collection_lambdas() {
    let program = compile(
        r#"
        _nodes = html("<a href='https://example.com/a'>A</a><a href='/b'>B</a>").css@("a")
        mapped_method = _nodes.map(node => node.href())
        mapped_function = map(_nodes, node => href(node))
        filtered_method = _nodes.filter(node => node.href().contains("https://"))
        filtered_function = filter(_nodes, node => contains(href(node), "https://"))
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    let expected = Value::Array(vec![
        Value::from("https://example.com/a"),
        Value::from("/b"),
    ]);
    assert_eq!(result.output().get("mapped_method"), Some(&expected));
    assert_eq!(result.output().get("mapped_function"), Some(&expected));
    assert_eq!(
        result.output().get("filtered_method"),
        result.output().get("filtered_function")
    );
    assert_eq!(
        result
            .output()
            .get("filtered_method")
            .unwrap()
            .expect_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn switch_matches_first_equal_arm_and_requires_exhaustive_fallback() {
    let program = compile(
        r#"
        selected = switch(status) {
            200 => "ok",
            404 => "missing",
            code => f"unexpected {code}",
        }
        "#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(
            &program,
            Inputs::new().insert("status", Value::from(500), Visibility::Private),
        )
        .await
        .unwrap();

    assert_eq!(
        result.output().get("selected"),
        Some(&Value::from("unexpected 500"))
    );

    let missing_fallback = compile(r#"value = switch(status) { 200 => "ok" }"#).unwrap_err();
    assert!(
        missing_fallback.diagnostics()[0]
            .message()
            .contains("final switch arm")
    );
}

#[tokio::test]
async fn sorts_by_key_and_two_parameter_comparator() {
    let program = compile(
        r#"
        by_key = [3, 1, 2].sort_by_key(value => -value)
        by_comparator = [3, 1, 2].sort_by((a, b) => b - a)
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    let descending = Value::Array(vec![3.into(), 2.into(), 1.into()]);
    assert_eq!(result.output().get("by_key"), Some(&descending));
    assert_eq!(result.output().get("by_comparator"), Some(&descending));
}

#[tokio::test]
async fn sorting_lambdas_enforce_keys_and_comparator_results() {
    let null_key = compile("value = [1].sort_by_key(item => null)").unwrap();
    let error = Engine::new()
        .execute(&null_key, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("non-null string or number key"));

    let boolean_comparator = compile("value = [1, 2].sort_by((a, b) => a < b)").unwrap();
    let error = Engine::new()
        .execute(&boolean_comparator, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("comparator must return a number"));
}

#[tokio::test]
async fn guards_preserve_payloads_and_collect_warnings() {
    let program = compile(
        r#"
        value = input.nullcheck("missing")
            .assert(x => x > 0, "must be positive")
            .warn(x => x > 10, "small value")
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert("input", Value::from(2), Visibility::Private);

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("value"), Some(&Value::from(2)));
    assert_eq!(result.warnings()[0].message(), "small value");
}

#[tokio::test]
async fn assert_accepts_direct_boolean_conditions() {
    let program = compile(r#"value = assert([1, 2, 3, 4].len() > 3, "too few images")"#).unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("value"), Some(&Value::from(true)));

    let program = compile(r#"value = assert([1].len() > 3, "too few images")"#).unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert_eq!(error.message(), "too few images");
}

#[tokio::test]
async fn evaluates_arithmetic_comparisons_equality_and_unary_operators() {
    let program = compile(
        r#"
        precedence = 2 + 3 * 4
        remainder = 17 % 5
        negative = -3
        not = !false
        less = 2 < 3
        equal = [1, 2] == [1, 2]
        different = "a" != "b"
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(result.output().get("precedence"), Some(&Value::from(14)));
    assert_eq!(result.output().get("remainder"), Some(&Value::from(2)));
    assert_eq!(result.output().get("negative"), Some(&Value::from(-3_i64)));
    assert_eq!(result.output().get("not"), Some(&Value::from(true)));
    assert_eq!(result.output().get("less"), Some(&Value::from(true)));
    assert_eq!(result.output().get("equal"), Some(&Value::from(true)));
    assert_eq!(result.output().get("different"), Some(&Value::from(true)));
}

#[tokio::test]
async fn interpolates_scalar_values_in_source_order() {
    let program = compile(
        r#"value = f"text={text} integer={40 + 2} decimal={decimal} boolean={enabled} null={null} dollar=$""#,
    )
    .unwrap();
    let inputs = Inputs::new()
        .insert("text", Value::from("ok"), Visibility::Private)
        .insert(
            "decimal",
            Value::Number(serde_json::Number::from_f64(12.5).unwrap()),
            Visibility::Private,
        )
        .insert("enabled", Value::from(true), Visibility::Private);

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("value"),
        Some(&Value::from(
            "text=ok integer=42 decimal=12.5 boolean=true null=null dollar=$"
        ))
    );
}

#[tokio::test]
async fn interpolates_asurascans_search_url() {
    let program = compile(
        r#"
        limit = 50
        order = "desc"
        offset = (page - 1) * limit
        url = f"https://api.asurascans.com/api/series?search={query.urlencode()}&sort=latest&order={order}&limit={limit}&offset={offset}"
        "#,
    )
    .unwrap();
    let inputs = Inputs::new()
        .insert("query", Value::from("hero & mage"), Visibility::Private)
        .insert("page", Value::from(2), Visibility::Private);

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("url"),
        Some(&Value::from(
            "https://api.asurascans.com/api/series?search=hero+%26+mage&sort=latest&order=desc&limit=50&offset=50"
        ))
    );
}

#[tokio::test]
async fn rejects_structured_values_in_interpolated_strings() {
    let program = compile(r#"value = f"items={[1, 2]}""#).unwrap();

    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();

    assert!(error.message().contains("cannot interpolate array"));
    assert!(error.span().is_some());
}

#[tokio::test]
async fn evaluates_logical_operators_in_filter_predicates() {
    let program = compile(
        r#"
        visible = items.filter(item => !item["is_premium"] && !item["is_locked"] || item["override"])
        "#,
    )
    .unwrap();
    let inputs = Inputs::new().insert(
        "items",
        Value::Array(vec![
            object([
                ("is_premium", Value::from(false)),
                ("is_locked", Value::from(false)),
                ("override", Value::from(false)),
                ("id", Value::from(1)),
            ]),
            object([
                ("is_premium", Value::from(true)),
                ("is_locked", Value::from(false)),
                ("override", Value::from(false)),
                ("id", Value::from(2)),
            ]),
            object([
                ("is_premium", Value::from(true)),
                ("is_locked", Value::from(true)),
                ("override", Value::from(true)),
                ("id", Value::from(3)),
            ]),
        ]),
        Visibility::Private,
    );

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(
        result.output().get("visible"),
        Some(&Value::Array(vec![
            object([
                ("is_premium", Value::from(false)),
                ("is_locked", Value::from(false)),
                ("override", Value::from(false)),
                ("id", Value::from(1)),
            ]),
            object([
                ("is_premium", Value::from(true)),
                ("is_locked", Value::from(true)),
                ("override", Value::from(true)),
                ("id", Value::from(3)),
            ]),
        ]))
    );
}

#[tokio::test]
async fn logical_operators_require_boolean_operands() {
    let program = compile("value = true && 1").unwrap();

    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();

    assert!(error.message().contains("expected boolean"));
}

#[tokio::test]
async fn logical_operators_short_circuit_unselected_operands() {
    let program = compile(
        r#"
        and = false && 1 / 0 == 0
        or = true || 1 / 0 == 0
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("and"), Some(&Value::from(false)));
    assert_eq!(result.output().get("or"), Some(&Value::from(true)));
}

#[tokio::test]
async fn conditional_expressions_evaluate_only_the_selected_branch() {
    let program = compile(
        r#"
        truthy = "yes" if true else 1 / 0
        falsey = "no" if false else "fallback"
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("truthy"), Some(&Value::from("yes")));
    assert_eq!(
        result.output().get("falsey"),
        Some(&Value::from("fallback"))
    );
}

#[tokio::test]
async fn conditional_expressions_require_boolean_conditions() {
    let program = compile(r#"value = "yes" if "truthy" else "no""#).unwrap();

    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();

    assert!(error.message().contains("expected boolean condition"));
}

#[tokio::test]
async fn rejects_invalid_dynamic_operand_types_with_a_source_span() {
    let program = compile("value = dynamic + 1").unwrap();
    let inputs = Inputs::new().insert("dynamic", Value::from("x"), Visibility::Private);

    let error = Engine::new().execute(&program, inputs).await.unwrap_err();
    assert!(error.message().contains("requires a number-like value"));
    assert!(error.span().is_some());
}

#[tokio::test]
async fn rejects_integer_division_by_zero() {
    let program = compile("value = 10 / 0").unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("invalid integer arithmetic"));
}

#[tokio::test]
async fn public_inputs_are_exported_and_private_inputs_are_hidden() {
    let program = compile("copy = private").unwrap();
    let inputs = Inputs::new()
        .insert("public", Value::from(1), Visibility::Public)
        .insert("private", Value::from(2), Visibility::Private);
    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("public"), Some(&Value::from(1)));
    assert_eq!(result.output().get("private"), None);
    assert_eq!(result.output().get("copy"), Some(&Value::from(2)));
}

#[tokio::test]
async fn extend_merges_object_fields_into_exported_context() {
    let program = compile(
        r#"
        before = 1
        extend({added: 2, before: 3})
        after = added + before
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("before"), Some(&Value::from(3)));
    assert_eq!(result.output().get("added"), Some(&Value::from(2)));
    assert_eq!(result.output().get("after"), Some(&Value::from(5)));
}

#[tokio::test]
async fn extend_exports_fields_from_child_scopes() {
    let program = compile(
        r#"
        _items = [1, 2].map { extend({last: it}) }
        after = last + 1
        "#,
    )
    .unwrap();

    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("items"), None);
    assert_eq!(result.output().get("last"), Some(&Value::from(2)));
    assert_eq!(result.output().get("after"), Some(&Value::from(3)));
}

#[tokio::test]
async fn extend_requires_exactly_one_object_argument() {
    let not_object = compile("extend(1)").unwrap();
    let error = Engine::new()
        .execute(&not_object, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("expects an object"));

    let wrong_arity = compile("extend({}, {})").unwrap();
    let error = Engine::new()
        .execute(&wrong_arity, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("exactly one object argument"));
}

#[tokio::test]
async fn missing_inputs_fail_before_execution() {
    let program = compile("value = missing").unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("missing required input `missing`"));
}

#[tokio::test]
async fn filter_requires_a_boolean_predicate() {
    let program = compile("values = [1].filter(x => x)").unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert!(error.message().contains("expected boolean"));
}

#[tokio::test]
async fn nullcheck_only_rejects_null() {
    let accepted = compile(
        r#"false = false.nullcheck("bad")
empty = "".nullcheck("bad")
array = [].nullcheck("bad")"#,
    )
    .unwrap();
    Engine::new()
        .execute(&accepted, Inputs::new())
        .await
        .unwrap();

    let rejected = compile("value = null.nullcheck(\"missing\")").unwrap();
    let error = Engine::new()
        .execute(&rejected, Inputs::new())
        .await
        .unwrap_err();
    assert_eq!(error.message(), "missing");
}

#[tokio::test]
async fn failed_assertions_stop_execution_without_output() {
    let program = compile("value = 1.assert(x => x > 2, \"too small\")").unwrap();
    let error = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap_err();
    assert_eq!(error.message(), "too small");
}

#[tokio::test]
async fn linking_rejects_inputs_that_violate_inferred_constraints() {
    let html_program = compile("value = html.css(\"p\")").unwrap();
    let error = Engine::new()
        .execute(
            &html_program,
            Inputs::new().insert("html", Value::from("raw"), Visibility::Private),
        )
        .await
        .unwrap_err();
    assert!(error.message().contains("requires an HTML-like value"));

    let number_program = compile("value = number + 1").unwrap();
    let error = Engine::new()
        .execute(
            &number_program,
            Inputs::new().insert("number", Value::from(false), Visibility::Private),
        )
        .await
        .unwrap_err();
    assert!(error.message().contains("requires a number-like value"));
}

#[tokio::test]
async fn child_scope_assignments_do_not_mutate_outer_bindings() {
    let program = compile(
        r#"value = 10
items = [1].map { value = it }
after = value"#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();

    assert_eq!(result.output().get("value"), Some(&Value::from(10)));
    assert_eq!(result.output().get("after"), Some(&Value::from(10)));
    assert_eq!(
        result
            .output()
            .get("items")
            .unwrap()
            .expect_array()
            .unwrap()[0]
            .expect_object()
            .unwrap()
            .get("value"),
        Some(&Value::from(1))
    );
}

#[tokio::test]
async fn warnings_accumulate_in_execution_order() {
    let program = compile(
        r#"1.warn(x => false, "first")
2.warn(x => false, "second")"#,
    )
    .unwrap();
    let result = Engine::new()
        .execute(&program, Inputs::new())
        .await
        .unwrap();
    assert_eq!(
        result
            .warnings()
            .iter()
            .map(|warning| warning.message())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
}

#[tokio::test]
async fn evaluates_subscript_indexing_on_arrays_and_objects() {
    let program = compile(
        r#"
        array_val = arr[1]
        array_out_of_bounds = arr[5]
        object_val = obj["name"]
        object_missing = obj["missing"]
        chained_val = nested["items"][0]
        "#,
    )
    .unwrap();

    let mut obj_map = html_extractor_runtime::Object::new();
    obj_map.insert("name", Value::from("Antigravity"));

    let mut nested_map = html_extractor_runtime::Object::new();
    nested_map.insert(
        "items",
        Value::Array(vec![Value::from("first"), Value::from("second")]),
    );

    let inputs = Inputs::new()
        .insert(
            "arr",
            Value::Array(vec![Value::from(10), Value::from(20), Value::from(30)]),
            Visibility::Private,
        )
        .insert("obj", Value::Object(obj_map), Visibility::Private)
        .insert("nested", Value::Object(nested_map), Visibility::Private);

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("array_val"), Some(&Value::from(20)));
    assert_eq!(
        result.output().get("array_out_of_bounds"),
        Some(&Value::Null)
    );
    assert_eq!(
        result.output().get("object_val"),
        Some(&Value::from("Antigravity"))
    );
    assert_eq!(result.output().get("object_missing"), Some(&Value::Null));
    assert_eq!(
        result.output().get("chained_val"),
        Some(&Value::from("first"))
    );
}

#[tokio::test]
async fn evaluates_dot_access_like_subscript() {
    let program = compile(
        r#"
        array_val = arr.1
        array_out_of_bounds = arr.5
        object_val = obj.name
        object_missing = obj.missing
        chained_val = nested.items.0
        "#,
    )
    .unwrap();

    let mut obj_map = html_extractor_runtime::Object::new();
    obj_map.insert("name", Value::from("Antigravity"));

    let mut nested_map = html_extractor_runtime::Object::new();
    nested_map.insert(
        "items",
        Value::Array(vec![Value::from("first"), Value::from("second")]),
    );

    let inputs = Inputs::new()
        .insert(
            "arr",
            Value::Array(vec![Value::from(10), Value::from(20), Value::from(30)]),
            Visibility::Private,
        )
        .insert("obj", Value::Object(obj_map), Visibility::Private)
        .insert("nested", Value::Object(nested_map), Visibility::Private);

    let result = Engine::new().execute(&program, inputs).await.unwrap();

    assert_eq!(result.output().get("array_val"), Some(&Value::from(20)));
    assert_eq!(
        result.output().get("array_out_of_bounds"),
        Some(&Value::Null)
    );
    assert_eq!(
        result.output().get("object_val"),
        Some(&Value::from("Antigravity"))
    );
    assert_eq!(result.output().get("object_missing"), Some(&Value::Null));
    assert_eq!(
        result.output().get("chained_val"),
        Some(&Value::from("first"))
    );

    let string_on_array = compile("value = arr.key").unwrap();
    let inputs = Inputs::new().insert(
        "arr",
        Value::Array(vec![Value::from(10)]),
        Visibility::Private,
    );
    let error = Engine::new()
        .execute(&string_on_array, inputs)
        .await
        .unwrap_err();
    assert!(
        error
            .message()
            .contains("cannot index array with string key")
    );

    let integer_on_object = compile("value = obj.0").unwrap();
    let inputs = Inputs::new().insert(
        "obj",
        Value::Object(html_extractor_runtime::Object::new()),
        Visibility::Private,
    );
    let error = Engine::new()
        .execute(&integer_on_object, inputs)
        .await
        .unwrap_err();
    assert!(
        error
            .message()
            .contains("cannot index object with integer index")
    );
}

#[tokio::test]
async fn rejects_invalid_subscript_indexing_types() {
    // 1. String key on array should fail.
    let program1 = compile(r#"val = arr["key"]"#).unwrap();
    let inputs1 = Inputs::new().insert(
        "arr",
        Value::Array(vec![Value::from(10)]),
        Visibility::Private,
    );
    let error1 = Engine::new().execute(&program1, inputs1).await.unwrap_err();
    assert!(
        error1
            .message()
            .contains("cannot index array with string key")
    );

    // 2. Integer index on object should fail.
    let program2 = compile(r#"val = obj[0]"#).unwrap();
    let mut obj_map = html_extractor_runtime::Object::new();
    obj_map.insert("name", Value::from("Antigravity"));
    let inputs2 = Inputs::new().insert("obj", Value::Object(obj_map), Visibility::Private);
    let error2 = Engine::new().execute(&program2, inputs2).await.unwrap_err();
    assert!(
        error2
            .message()
            .contains("cannot index object with integer index")
    );

    // 3. Subscript index not number or string should fail.
    let program3 = compile(r#"val = obj[null]"#).unwrap();
    let mut obj_map = html_extractor_runtime::Object::new();
    obj_map.insert("name", Value::from("Antigravity"));
    let inputs3 = Inputs::new().insert("obj", Value::Object(obj_map), Visibility::Private);
    let error3 = Engine::new().execute(&program3, inputs3).await.unwrap_err();
    assert!(
        error3
            .message()
            .contains("subscript index must be a number or a string")
    );
}
