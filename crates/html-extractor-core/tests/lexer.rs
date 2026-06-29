use html_extractor_core::{InterpolatedTokenPart, Span, Token, TokenKind, lex};

fn lex_ok(source: &str) -> Vec<Token> {
    lex(source).unwrap_or_else(|error| panic!("lexing failed: {error:?}"))
}

fn kinds(source: &str) -> Vec<TokenKind> {
    lex_ok(source)
        .into_iter()
        .map(|token| token.kind().clone())
        .collect()
}

#[test]
fn empty_source_contains_only_eof() {
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
}

#[test]
fn eof_span_is_at_end_of_source() {
    let tokens = lex_ok("abc");
    assert_eq!(tokens.last().unwrap().span(), Span::new(3, 3));
}

#[test]
fn lexes_public_identifier() {
    assert_eq!(
        kinds("name"),
        vec![TokenKind::Identifier("name".into()), TokenKind::Eof]
    );
}

#[test]
fn lexes_private_identifier_as_one_token() {
    assert_eq!(
        kinds("_raw"),
        vec![TokenKind::Identifier("_raw".into()), TokenKind::Eof]
    );
}

#[test]
fn lexes_qualified_function_syntax() {
    assert_eq!(
        kinds("prices.parse_price(raw)"),
        vec![
            TokenKind::Identifier("prices".into()),
            TokenKind::Dot,
            TokenKind::Identifier("parse_price".into()),
            TokenKind::LeftParen,
            TokenKind::Identifier("raw".into()),
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_multi_selection_method_suffix() {
    assert_eq!(
        kinds(".css@"),
        vec![
            TokenKind::Dot,
            TokenKind::Identifier("css".into()),
            TokenKind::At,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_required_result_operator() {
    assert_eq!(kinds("!"), vec![TokenKind::Bang, TokenKind::Eof]);
}

#[test]
fn lexes_all_delimiters() {
    assert_eq!(
        kinds("(){}[],:"),
        vec![
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::Comma,
            TokenKind::Colon,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_arithmetic_operators() {
    assert_eq!(
        kinds("+ - * / %"),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_assignment_comparison_and_lambda_operators() {
    assert_eq!(
        kinds("= == != < <= > >= => && ||"),
        vec![
            TokenKind::Equal,
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::Less,
            TokenKind::LessEqual,
            TokenKind::Greater,
            TokenKind::GreaterEqual,
            TokenKind::FatArrow,
            TokenKind::AmpAmp,
            TokenKind::PipePipe,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_integer_and_decimal_numbers() {
    assert_eq!(
        kinds("0 42 12.50"),
        vec![
            TokenKind::Number("0".into()),
            TokenKind::Number("42".into()),
            TokenKind::Number("12.50".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn leaves_a_second_decimal_point_for_the_parser() {
    assert_eq!(
        kinds("1.2.3"),
        vec![
            TokenKind::Number("1.2".into()),
            TokenKind::Dot,
            TokenKind::Number("3".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_string_contents_without_quotes() {
    assert_eq!(
        kinds(r#""product""#),
        vec![TokenKind::String("product".into()), TokenKind::Eof]
    );
}

#[test]
fn decodes_supported_string_escapes() {
    assert_eq!(
        kinds(r#""quote: \" slash: \\ line:\n tab:\t return:\r""#),
        vec![
            TokenKind::String("quote: \" slash: \\ line:\n tab:\t return:\r".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn comment_markers_inside_strings_are_text() {
    assert_eq!(
        kinds(r#""// text /* text */""#),
        vec![
            TokenKind::String("// text /* text */".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn skips_line_comments() {
    assert_eq!(
        kinds("a // ignored\nb"),
        vec![
            TokenKind::Identifier("a".into()),
            TokenKind::Identifier("b".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn line_comment_may_end_at_eof() {
    assert_eq!(kinds("// ignored"), vec![TokenKind::Eof]);
}

#[test]
fn skips_multiline_block_comments() {
    assert_eq!(
        kinds("a /* first\nsecond */ b"),
        vec![
            TokenKind::Identifier("a".into()),
            TokenKind::Identifier("b".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn token_spans_remain_byte_offsets_after_comments() {
    let tokens = lex_ok("// one\nβ");
    assert_eq!(tokens[0].span(), Span::new(7, 9));
}

#[test]
fn identifier_spans_use_utf8_byte_offsets() {
    let tokens = lex_ok("βeta");
    assert_eq!(tokens[0].kind(), &TokenKind::Identifier("βeta".into()));
    assert_eq!(tokens[0].span(), Span::new(0, 5));
}

#[test]
fn lexes_interpolated_string_parts() {
    let tokens = lex_ok(r#"f"hello {name} {1 + offset} $""#);
    let TokenKind::InterpolatedString(parts) = tokens[0].kind() else {
        panic!("expected interpolated string");
    };
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0], InterpolatedTokenPart::Literal("hello ".into()));
    assert!(matches!(
        &parts[1],
        InterpolatedTokenPart::Expression(tokens)
            if tokens.iter().any(|token| token.kind() == &TokenKind::Identifier("name".into()))
    ));
    assert_eq!(parts[2], InterpolatedTokenPart::Literal(" ".into()));
    assert!(matches!(
        &parts[3],
        InterpolatedTokenPart::Expression(tokens)
            if tokens.iter().any(|token| token.kind() == &TokenKind::Plus)
    ));
    assert_eq!(parts[4], InterpolatedTokenPart::Literal(" $".into()));
    assert_eq!(tokens[0].span(), Span::new(0, 30));
}

#[test]
fn reports_unterminated_interpolated_expression() {
    let error = lex(r#"f"bad {1 + 2""#).unwrap_err();
    assert_eq!(
        error.diagnostics()[0].message(),
        "unterminated interpolation expression"
    );
}

#[test]
fn reports_unterminated_interpolated_string() {
    let error = lex(r#"f"unfinished {name}"#).unwrap_err();
    assert_eq!(
        error.diagnostics()[0].message(),
        "unterminated interpolated string literal"
    );
}

#[test]
fn reports_embedded_lexer_errors_at_their_outer_source_span() {
    let error = lex(r#"f"{1 & 2}""#).unwrap_err();
    assert_eq!(error.diagnostics()[0].primary_span(), Span::new(5, 6));
}

#[test]
fn preserves_global_spans_in_nested_interpolated_strings() {
    let tokens = lex_ok(r#"f"{f"{name}"}""#);
    let TokenKind::InterpolatedString(outer_parts) = tokens[0].kind() else {
        panic!("expected outer interpolated string");
    };
    let InterpolatedTokenPart::Expression(outer_tokens) = &outer_parts[0] else {
        panic!("expected outer expression");
    };
    let TokenKind::InterpolatedString(inner_parts) = outer_tokens[0].kind() else {
        panic!("expected inner interpolated string");
    };
    let InterpolatedTokenPart::Expression(inner_tokens) = &inner_parts[0] else {
        panic!("expected inner expression");
    };
    assert_eq!(inner_tokens[0].span(), Span::new(6, 10));
}

#[test]
fn reports_unterminated_string_at_opening_quote() {
    let error = lex(r#""unfinished"#).unwrap_err();
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(diagnostic.message(), "unterminated string literal");
    assert_eq!(diagnostic.primary_span(), Span::new(0, 1));
}

#[test]
fn reports_unterminated_block_comment_at_opening_delimiter() {
    let error = lex("prefix /* unfinished").unwrap_err();
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(diagnostic.message(), "unterminated block comment");
    assert_eq!(diagnostic.primary_span(), Span::new(7, 9));
}

#[test]
fn reports_unsupported_string_escape_at_escape_sequence() {
    let error = lex(r#""bad \q""#).unwrap_err();
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(diagnostic.message(), "unsupported string escape `\\q`");
    assert_eq!(diagnostic.primary_span(), Span::new(5, 7));
}

#[test]
fn reports_unexpected_characters_at_their_utf8_span() {
    let error = lex("name & value").unwrap_err();
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(diagnostic.message(), "unexpected character `&`");
    assert_eq!(diagnostic.primary_span(), Span::new(5, 6));
}

#[test]
fn representative_design_example_has_expected_lexemes() {
    let source = r#"
        _response = fetch("https://example.com/products")
        products = _response.css@(".product")!.map {
            name = it.css(".name")!.text()
        }
    "#;

    let tokens = lex_ok(source);
    assert!(
        tokens
            .iter()
            .any(|token| token.kind() == &TokenKind::Identifier("_response".into()))
    );
    assert!(tokens.iter().any(|token| token.kind() == &TokenKind::At));
    assert_eq!(
        tokens
            .iter()
            .filter(|token| token.kind() == &TokenKind::Bang)
            .count(),
        2
    );
    assert_eq!(tokens.last().unwrap().kind(), &TokenKind::Eof);
}

#[test]
fn lexes_dollar_free_identifiers_assignments_and_calls() {
    assert_eq!(
        kinds("name = _input\nresponse = fetch(name)"),
        vec![
            TokenKind::Identifier("name".into()),
            TokenKind::Equal,
            TokenKind::Identifier("_input".into()),
            TokenKind::Identifier("response".into()),
            TokenKind::Equal,
            TokenKind::Identifier("fetch".into()),
            TokenKind::LeftParen,
            TokenKind::Identifier("name".into()),
            TokenKind::RightParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn rejects_legacy_dollar_syntax_immediately() {
    let error = lex("$fetch(\"https://example.test\")").unwrap_err();
    let diagnostic = &error.diagnostics()[0];
    assert_eq!(diagnostic.message(), "unexpected character `$`");
    assert_eq!(diagnostic.primary_span(), Span::new(0, 1));
}

#[test]
fn lexes_python_style_interpolation_and_literal_braces() {
    let tokens = lex_ok(r#"f"value={name} literal={{ok}} dollar=$""#);
    let TokenKind::InterpolatedString(parts) = tokens[0].kind() else {
        panic!("expected interpolated string");
    };
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], InterpolatedTokenPart::Literal("value=".into()));
    assert!(matches!(
        &parts[1],
        InterpolatedTokenPart::Expression(tokens)
            if tokens[0].kind() == &TokenKind::Identifier("name".into())
    ));
    assert_eq!(
        parts[2],
        InterpolatedTokenPart::Literal(" literal={ok} dollar=$".into())
    );
}

#[test]
fn rejects_empty_and_unmatched_python_interpolation_braces() {
    for (source, message) in [
        (r#"f"{}""#, "empty interpolation expression"),
        (r#"f"{name""#, "unterminated interpolation expression"),
        (r#"f"name}""#, "unmatched `}` in interpolated string"),
    ] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.diagnostics()[0].message(), message);
    }
}
