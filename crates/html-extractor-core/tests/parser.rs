use html_extractor_core::{
    AssignmentOperator, BinaryOperator, ExpressionKind, InterpolatedStringPart, StatementKind,
    Visibility, parse,
};

#[test]
fn parses_script_functions_block_if_and_loops() {
    let program = parse(
        r#"
        fun classify(value) {
            if value < 0 { return "negative" }
            if (value == 0) { "zero" } else { "positive" }
        }
        while count < 10 { count = count + 1 }
        while (count < 20) { break }
        loop { break }
        result = if ready { "yes" } else { "no" }
        if ready { seen = true }
        "#,
    )
    .expect("control-flow source must parse");

    assert_eq!(program.functions().len(), 1);
    let function = &program.functions()[0];
    assert_eq!(function.name(), "classify");
    assert_eq!(function.parameters(), ["value"]);
    assert!(matches!(
        function.body()[0].kind(),
        StatementKind::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::BlockIf { else_branch: None, .. })
    ));
    assert!(matches!(
        function.body()[1].kind(),
        StatementKind::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::BlockIf { else_branch: Some(_), .. })
    ));

    assert_eq!(program.statements().len(), 5);
    assert!(matches!(
        program.statements()[0].kind(),
        StatementKind::While { .. }
    ));
    assert!(
        matches!(program.statements()[1].kind(), StatementKind::While { body, .. }
        if matches!(body[0].kind(), StatementKind::Break))
    );
    assert!(
        matches!(program.statements()[2].kind(), StatementKind::Loop { body }
        if matches!(body[0].kind(), StatementKind::Break))
    );
    assert!(matches!(
        program.statements()[3].kind(),
        StatementKind::Assignment { value, .. }
            if matches!(value.kind(), ExpressionKind::BlockIf { else_branch: Some(_), .. })
    ));
    assert!(matches!(
        program.statements()[4].kind(),
        StatementKind::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::BlockIf { else_branch: None, .. })
    ));
}

#[test]
fn parses_assignments_and_ordered_literals() {
    let program = parse(
        r#"
        name = "A"
        _meta = { count: 2, flags: [true, null] }
        "done"
        "#,
    )
    .expect("source must parse");

    assert_eq!(program.statements().len(), 3);

    let StatementKind::Assignment {
        name,
        visibility,
        value,
        ..
    } = program.statements()[0].kind()
    else {
        panic!("expected assignment");
    };
    assert_eq!(name, "name");
    assert_eq!(*visibility, Visibility::Public);
    assert!(matches!(value.kind(), ExpressionKind::String(value) if value == "A"));

    let StatementKind::Assignment {
        visibility, value, ..
    } = program.statements()[1].kind()
    else {
        panic!("expected private assignment");
    };
    assert_eq!(*visibility, Visibility::Private);
    let ExpressionKind::Object(fields) = value.kind() else {
        panic!("expected object");
    };
    assert_eq!(fields[0].name(), "count");
    assert_eq!(fields[1].name(), "flags");

    assert!(matches!(
        program.statements()[2].kind(),
        StatementKind::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::String(value) if value == "done")
    ));
}

#[test]
fn parses_augmented_assignments() {
    let program = parse(
        r#"
        count += 1
        count -= step
        "#,
    )
    .expect("augmented assignments should parse");

    assert_eq!(program.statements().len(), 2);
    assert!(matches!(
        program.statements()[0].kind(),
        StatementKind::Assignment {
            operator: AssignmentOperator::Add,
            name,
            ..
        } if name == "count"
    ));
    assert!(matches!(
        program.statements()[1].kind(),
        StatementKind::Assignment {
            operator: AssignmentOperator::Subtract,
            name,
            ..
        } if name == "count"
    ));
}

#[test]
fn parses_script_test_blocks_with_and_without_inputs() {
    let program = parse(
        r#"
        value = input
        test(input = "demo", _search = "") {
            assertEq(input, it.value, "script output")
        }
        test {
            assertEq(double(2), 4, "function result")
        }
        fun double(value) { value * 2 }
        "#,
    )
    .expect("test blocks should parse");

    assert_eq!(program.tests().len(), 2);
    assert!(program.tests()[0].runs_script());
    assert_eq!(program.tests()[0].inputs().len(), 2);
    assert_eq!(program.tests()[0].body().len(), 1);
    assert!(!program.tests()[1].runs_script());
    assert!(program.tests()[1].inputs().is_empty());
}

#[test]
fn parses_computed_assignment_targets() {
    let program = parse(
        r#"
        _key = "title"
        [_key] = "Example"
        "#,
    )
    .expect("computed assignment target should parse");

    assert_eq!(program.statements().len(), 2);
}

#[test]
fn applies_binary_precedence() {
    let program = parse("ok = 1 + 2 * 3 >= 7 == true").unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    let ExpressionKind::Binary { operator, left, .. } = value.kind() else {
        panic!("expected equality");
    };
    assert_eq!(*operator, BinaryOperator::Equal);
    let ExpressionKind::Binary { operator, left, .. } = left.kind() else {
        panic!("expected comparison");
    };
    assert_eq!(*operator, BinaryOperator::GreaterEqual);
    let ExpressionKind::Binary {
        operator, right, ..
    } = left.kind()
    else {
        panic!("expected addition");
    };
    assert_eq!(*operator, BinaryOperator::Add);
    assert!(matches!(
        right.kind(),
        ExpressionKind::Binary {
            operator: BinaryOperator::Multiply,
            ..
        }
    ));
}

#[test]
fn parses_logical_operators_with_lower_precedence_than_not() {
    let program = parse("ok = !false && true || false").unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    let ExpressionKind::Binary { operator, left, .. } = value.kind() else {
        panic!("expected logical or");
    };
    assert_eq!(*operator, BinaryOperator::Or);
    let ExpressionKind::Binary {
        operator,
        left,
        right,
    } = left.kind()
    else {
        panic!("expected logical and");
    };
    assert_eq!(*operator, BinaryOperator::And);
    assert!(matches!(
        left.kind(),
        ExpressionKind::Unary {
            operator: html_extractor_core::UnaryOperator::Not,
            ..
        }
    ));
    assert!(matches!(right.kind(), ExpressionKind::Boolean(true)));
}

#[test]
fn parses_calls_lambdas_blocks_and_required_binding() {
    let program = parse(
        r#"
        price = parse_price(raw)
        _response = fetch(url)
        positive = numbers.filter(x => x > 0)
        title = html.css("h1")!.text()
        items = nodes.map { name = it.text() }
        "#,
    )
    .unwrap();

    let values: Vec<_> = program
        .statements()
        .iter()
        .map(|statement| match statement.kind() {
            StatementKind::Assignment { value, .. } => value.kind(),
            StatementKind::ComputedAssignment { .. } => panic!("expected static assignment"),
            StatementKind::Expression(_) => panic!("expected assignment"),
            _ => panic!("expected assignment"),
        })
        .collect();

    assert!(
        matches!(values[0], ExpressionKind::FunctionCall { name, .. } if name == "parse_price")
    );
    assert!(matches!(values[1], ExpressionKind::FunctionCall { name, .. } if name == "fetch"));
    assert!(
        matches!(values[2], ExpressionKind::MethodCall { arguments, .. }
        if matches!(arguments[0].kind(), ExpressionKind::Lambda { parameters, .. } if parameters == &["x"]))
    );
    assert!(
        matches!(values[3], ExpressionKind::MethodCall { receiver, name, .. }
        if name == "text" && matches!(receiver.kind(), ExpressionKind::Required(_)))
    );
    assert!(
        matches!(values[4], ExpressionKind::FunctionalBlock { method, body, .. }
        if method == "map" && body.len() == 1)
    );
}

#[test]
fn parses_dot_access_as_nested_subscripts() {
    let program = parse("value = it.is_premium.0").unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    let ExpressionKind::Subscript { receiver, index } = value.kind() else {
        panic!("expected numeric subscript");
    };
    assert!(matches!(index.kind(), ExpressionKind::Number(value) if value == "0"));

    let ExpressionKind::Subscript { receiver, index } = receiver.kind() else {
        panic!("expected property subscript");
    };
    assert!(matches!(index.kind(), ExpressionKind::String(value) if value == "is_premium"));
    assert!(matches!(receiver.kind(), ExpressionKind::Variable(name) if name == "it"));
}

#[test]
fn dot_access_can_be_followed_by_a_method_call() {
    let program = parse("value = it.items.0.text()").unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    assert!(matches!(
        value.kind(),
        ExpressionKind::MethodCall { receiver, name, .. }
            if name == "text" && matches!(receiver.kind(), ExpressionKind::Subscript { .. })
    ));
}

#[test]
fn rejects_malformed_dot_access_segments() {
    let decimal = parse("value = it.1.5").unwrap_err();
    assert!(
        decimal.diagnostics()[0]
            .message()
            .contains("expected non-negative integer index after `.`")
    );

    let missing = parse("value = it.").unwrap_err();
    assert!(
        missing.diagnostics()[0]
            .message()
            .contains("expected property name or non-negative integer after `.`")
    );
}

#[test]
fn reports_missing_closing_delimiter() {
    let error = parse("items = [1, 2").unwrap_err();
    assert!(error.diagnostics()[0].message().contains("expected `]`"));
}

#[test]
fn parses_parenthesized_multi_parameter_lambdas() {
    parse("ordered = values.sort_by((a, b) => a - b)").unwrap();
}

#[test]
fn parses_conditional_expression_with_low_precedence() {
    let program =
        parse(r#"url = url if url.starts_with("https://api") else fetch(url).text()"#).unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    let ExpressionKind::IfElse {
        then_branch,
        condition,
        else_branch,
    } = value.kind()
    else {
        panic!("expected conditional expression");
    };
    assert!(matches!(then_branch.kind(), ExpressionKind::Variable(name) if name == "url"));
    assert!(matches!(
        condition.kind(),
        ExpressionKind::MethodCall { name, .. } if name == "starts_with"
    ));
    assert!(matches!(
        else_branch.kind(),
        ExpressionKind::MethodCall { name, .. } if name == "text"
    ));
}

#[test]
fn parses_switch_expression_with_exhaustive_fallback() {
    let program = parse(
        r#"
        label = switch(status) {
            200 => "ok",
            404 => "missing",
            code => f"unexpected {code}",
        }
        "#,
    )
    .unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };

    assert!(matches!(
        value.kind(),
        ExpressionKind::Switch { target, arms }
            if arms.len() == 3 && matches!(target.kind(), ExpressionKind::Variable(name) if name == "status")
    ));
}

#[test]
fn parses_interpolated_string_expressions() {
    let program = parse(r#"url = f"hello {name} {1 + offset}""#).unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };
    let ExpressionKind::InterpolatedString(parts) = value.kind() else {
        panic!("expected interpolated string");
    };
    assert!(matches!(
        &parts[0],
        InterpolatedStringPart::Literal(value) if value == "hello "
    ));
    assert!(matches!(
        &parts[1],
        InterpolatedStringPart::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::Variable(name) if name == "name")
    ));
    assert!(matches!(
        &parts[3],
        InterpolatedStringPart::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::Binary { operator: BinaryOperator::Add, .. })
    ));
}

#[test]
fn rejects_trailing_tokens_in_interpolated_expression() {
    let error = parse(r#"value = f"{1 2}""#).unwrap_err();
    assert!(
        error.diagnostics()[0]
            .message()
            .contains("expected end of interpolation expression")
    );
}

#[test]
fn parses_dollar_free_assignments_references_and_calls() {
    let program = parse("public = input\n_private = public\nresult = fetch(_private)").unwrap();

    let StatementKind::Assignment {
        visibility, value, ..
    } = program.statements()[0].kind()
    else {
        panic!("expected public assignment");
    };
    assert_eq!(*visibility, Visibility::Public);
    assert!(matches!(value.kind(), ExpressionKind::Variable(name) if name == "input"));

    let StatementKind::Assignment { visibility, .. } = program.statements()[1].kind() else {
        panic!("expected private assignment");
    };
    assert_eq!(*visibility, Visibility::Private);

    let StatementKind::Assignment { value, .. } = program.statements()[2].kind() else {
        panic!("expected result assignment");
    };
    assert!(matches!(value.kind(), ExpressionKind::FunctionCall { name, .. } if name == "fetch"));
}

#[test]
fn parses_python_style_interpolation_expressions() {
    let program = parse(r#"url = f"hello {name} {1 + offset}""#).unwrap();
    let StatementKind::Assignment { value, .. } = program.statements()[0].kind() else {
        panic!("expected assignment");
    };
    let ExpressionKind::InterpolatedString(parts) = value.kind() else {
        panic!("expected interpolated string");
    };
    assert!(matches!(
        &parts[1],
        InterpolatedStringPart::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::Variable(name) if name == "name")
    ));
    assert!(matches!(
        &parts[3],
        InterpolatedStringPart::Expression(expression)
            if matches!(expression.kind(), ExpressionKind::Binary { operator: BinaryOperator::Add, .. })
    ));
}
