use html_extractor_core::{
    BuiltinFunctionKind, ExpressionKind, FunctionIdentity, InputConstraint, InterpolatedStringPart,
    StatementKind, builtin_function, compile,
};

#[test]
fn script_functions_do_not_become_external_requirements() {
    let program = compile(
        r#"
        fun alpha(value) { beta(value) }
        fun beta(value) { value }
        result = alpha(input)
        "#,
    )
    .unwrap();

    assert!(program.requirements().functions().is_empty());
    assert_eq!(program.requirements().inputs().len(), 1);
    assert_eq!(program.requirements().inputs()[0].name(), "input");
}

#[test]
fn rejects_invalid_script_function_declarations() {
    let duplicate = compile("fun same() { null } fun same() { null }").unwrap_err();
    assert!(
        duplicate.diagnostics()[0]
            .message()
            .contains("duplicate function")
    );

    let parameters = compile("fun same(value, value) { value }").unwrap_err();
    assert!(
        parameters.diagnostics()[0]
            .message()
            .contains("duplicate parameter")
    );

    let reserved = compile("fun fetch() { null }").unwrap_err();
    assert!(reserved.diagnostics()[0].message().contains("reserved"));

    let nested = compile("fun outer() { fun inner() { null } null }").unwrap_err();
    assert!(
        nested.diagnostics()[0]
            .message()
            .contains("only allowed at top level")
    );
}

#[test]
fn validates_return_and_break_contexts() {
    let returned = compile("return 1").unwrap_err();
    assert!(
        returned.diagnostics()[0]
            .message()
            .contains("only valid inside a function")
    );

    let broken = compile("break").unwrap_err();
    assert!(
        broken.diagnostics()[0]
            .message()
            .contains("only valid inside a loop")
    );

    let function_break = compile("fun stop() { break }").unwrap_err();
    assert!(
        function_break.diagnostics()[0]
            .message()
            .contains("only valid inside a loop")
    );

    compile("fun stop() { loop { return 1 } } while true { break }").unwrap();
}

#[test]
fn block_if_requires_else_only_when_its_value_is_used() {
    compile("if ready { seen = true }").unwrap();
    compile("fun note(ready) { if ready { seen = true } null }").unwrap();

    for source in [
        "value = if ready { 1 }",
        "value = call(if ready { 1 })",
        "fun choose(ready) { if ready { 1 } }",
    ] {
        let error = compile(source).unwrap_err();
        assert!(
            error.diagnostics()[0]
                .message()
                .contains("requires an `else` branch"),
            "unexpected diagnostic for {source:?}: {:?}",
            error.diagnostics()
        );
    }
}

#[test]
fn exposes_one_global_builtin_function_index() {
    assert!(builtin_function("href").is_some());
    assert!(builtin_function("contains").is_some());
    assert!(builtin_function("urlencode").is_some());
    assert!(builtin_function("map").is_some());
    assert_eq!(
        builtin_function("fetch").unwrap().kind(),
        BuiltinFunctionKind::Core
    );
    assert_eq!(
        builtin_function("map").unwrap().kind(),
        BuiltinFunctionKind::Lambda
    );
    assert_eq!(
        builtin_function("href").unwrap().kind(),
        BuiltinFunctionKind::Value
    );
    assert!(builtin_function("custom").is_none());
}

#[test]
fn infers_inputs_with_top_to_bottom_and_child_scope_rules() {
    let program = compile(
        r#"
        currency = "EUR"
        products = html.css@(".product").map {
            price = parse_price(it.text(), currency)
            from_host = missing
        }
        outside = price
        "#,
    )
    .unwrap();

    let names: Vec<_> = program
        .requirements()
        .inputs()
        .iter()
        .map(|input| input.name())
        .collect();
    assert_eq!(names, ["html", "missing", "price"]);
    assert_eq!(
        program.requirements().inputs()[0].constraint(),
        InputConstraint::HtmlLike
    );
}

#[test]
fn infers_and_merges_function_requirements() {
    let program = compile(
        r#"
        a = parse_price(raw)
        b = parse_price(raw, "EUR")
        c = prices.normalize(a)
        _response = fetch(url)
        "#,
    )
    .unwrap();

    let functions = program.requirements().functions();
    assert_eq!(functions.len(), 2);
    assert_eq!(
        functions[0].identity(),
        &FunctionIdentity::Unqualified("parse_price".into())
    );
    assert_eq!(functions[0].arities(), &[1, 2]);
    assert_eq!(functions[0].uses().len(), 2);
    assert_eq!(
        functions[1].identity(),
        &FunctionIdentity::Qualified {
            module: "prices".into(),
            name: "normalize".into(),
        }
    );
}

#[test]
fn exposes_validated_statements_to_the_runtime() {
    let program = compile("title = html.css(\"h1\")!.text()").unwrap();
    assert_eq!(program.source(), "title = html.css(\"h1\")!.text()");
    assert_eq!(program.statements().len(), 1);
    assert!(matches!(
        program.statements()[0].kind(),
        StatementKind::Assignment { name, .. } if name == "title"
    ));
}

#[test]
fn resolves_qualified_functions_in_the_runtime_ast() {
    let program = compile("value = prices.normalize(raw)").unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };
    assert!(matches!(
        value.kind(),
        ExpressionKind::QualifiedFunctionCall { module, name, .. }
            if module == "prices" && name == "normalize"
    ));
}

#[test]
fn rejects_it_outside_functional_blocks() {
    let error = compile("value = it").unwrap_err();
    assert_eq!(
        error.diagnostics()[0].message(),
        "`it` is only available inside functional blocks"
    );
}

#[test]
fn rejects_required_operator_on_non_matching_operations() {
    for source in ["value = 1!", "value = custom()!", "value = x.text()!"] {
        let error = compile(source).unwrap_err();
        assert_eq!(
            error.diagnostics()[0].message(),
            "`!` is only valid after `css`, `css@`, or `regex`"
        );
    }
}

#[test]
fn accepts_required_css_and_regex_operations() {
    compile("a = html.css(\"h1\")!\nb = text.regex(\"id=(.*)\")!").unwrap();
}

#[test]
fn rejects_statically_invalid_literal_receiver() {
    let error = compile("value = 1.css(\"h1\")").unwrap_err();
    assert_eq!(
        error.diagnostics()[0].message(),
        "method `css` cannot be called on a number"
    );
}

#[test]
fn multi_parameter_lambdas_define_each_parameter_locally() {
    let program = compile("ordered = values.sort_by((a, b) => a - b)").unwrap();
    let inputs: Vec<_> = program
        .requirements()
        .inputs()
        .iter()
        .map(|input| input.name())
        .collect();
    assert_eq!(inputs, ["values"]);
}

#[test]
fn validates_expressions_inside_interpolated_strings() {
    let program =
        compile(r#"url = f"q={query}&offset={(page - 1) * 50}&value={prices.normalize(raw)}""#)
            .unwrap();
    let inputs: Vec<_> = program
        .requirements()
        .inputs()
        .iter()
        .map(|input| input.name())
        .collect();
    assert_eq!(inputs, ["query", "page", "raw"]);

    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };
    let ExpressionKind::InterpolatedString(parts) = value.kind() else {
        panic!("expected interpolated string");
    };
    assert!(parts.iter().any(|part| matches!(
        part,
        InterpolatedStringPart::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::QualifiedFunctionCall { module, name, .. }
                if module == "prices" && name == "normalize")
    )));
}

#[test]
fn rewrites_dollar_free_core_calls_and_collects_interpolation_inputs() {
    let program = compile(
        r#"
        offset = (page - 1) * limit
        url = f"q={query}&offset={offset}"
        response = fetch(url)
        "#,
    )
    .unwrap();

    let inputs: Vec<_> = program
        .requirements()
        .inputs()
        .iter()
        .map(|input| input.name())
        .collect();
    assert_eq!(inputs, ["page", "limit", "query"]);

    let StatementKind::Assignment { value, .. } = program.statements()[2].kind() else {
        panic!("expected response assignment");
    };
    assert!(matches!(value.kind(), ExpressionKind::CoreCall { name, .. } if name == "fetch"));
}
