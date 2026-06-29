use html_extractor_core::{TokenKind, compile, lex};

macro_rules! valid_fixture {
    ($name:ident, $path:literal) => {
        #[test]
        fn $name() {
            let source = include_str!($path);
            let tokens = lex(source)
                .unwrap_or_else(|error| panic!("{} must lex successfully: {error:?}", $path));
            assert_eq!(
                tokens.last().map(|token| token.kind()),
                Some(&TokenKind::Eof)
            );
            compile(source)
                .unwrap_or_else(|error| panic!("{} must compile successfully: {error:?}", $path));
        }
    };
}

macro_rules! invalid_fixture {
    ($name:ident, $path:literal, $message:literal) => {
        #[test]
        fn $name() {
            let error = match lex(include_str!($path)) {
                Ok(_) => panic!("{} must fail lexing", $path),
                Err(error) => error,
            };
            assert_eq!(error.diagnostics()[0].message(), $message);
        }
    };
}

valid_fixture!(
    fixture_public_assignment,
    "fixtures/valid/public_assignment.extractor"
);
valid_fixture!(
    fixture_private_assignment,
    "fixtures/valid/private_assignment.extractor"
);
valid_fixture!(
    fixture_visibility_override,
    "fixtures/valid/visibility_override.extractor"
);
valid_fixture!(fixture_literals, "fixtures/valid/literals.extractor");
valid_fixture!(
    fixture_ordered_object,
    "fixtures/valid/ordered_object.extractor"
);
valid_fixture!(fixture_comments, "fixtures/valid/comments.extractor");
valid_fixture!(fixture_css_single, "fixtures/valid/css_single.extractor");
valid_fixture!(
    fixture_css_multiple,
    "fixtures/valid/css_multiple.extractor"
);
valid_fixture!(fixture_nested_css, "fixtures/valid/nested_css.extractor");
valid_fixture!(
    fixture_required_css,
    "fixtures/valid/required_css.extractor"
);
valid_fixture!(
    fixture_required_regex,
    "fixtures/valid/required_regex.extractor"
);
valid_fixture!(fixture_regex_group, "fixtures/valid/regex_group.extractor");
valid_fixture!(
    fixture_filter_lambda,
    "fixtures/valid/filter_lambda.extractor"
);
valid_fixture!(
    fixture_guard_pipeline,
    "fixtures/valid/guard_pipeline.extractor"
);
valid_fixture!(
    fixture_warning_pipeline,
    "fixtures/valid/warning_pipeline.extractor"
);
valid_fixture!(fixture_map_scope, "fixtures/valid/map_scope.extractor");
valid_fixture!(fixture_core_fetch, "fixtures/valid/core_fetch.extractor");
valid_fixture!(
    fixture_response_conversions,
    "fixtures/valid/response_conversions.extractor"
);
valid_fixture!(
    fixture_response_css,
    "fixtures/valid/response_css.extractor"
);
valid_fixture!(
    fixture_response_regex,
    "fixtures/valid/response_regex.extractor"
);
valid_fixture!(
    fixture_unqualified_function,
    "fixtures/valid/unqualified_function.extractor"
);
valid_fixture!(
    fixture_qualified_function,
    "fixtures/valid/qualified_function.extractor"
);
valid_fixture!(
    fixture_self_fetching,
    "fixtures/valid/self_fetching.extractor"
);
valid_fixture!(
    fixture_complete_products,
    "fixtures/valid/complete_products.extractor"
);

invalid_fixture!(
    fixture_unterminated_string,
    "fixtures/invalid/unterminated_string.extractor",
    "unterminated string literal"
);
invalid_fixture!(
    fixture_unterminated_comment,
    "fixtures/invalid/unterminated_comment.extractor",
    "unterminated block comment"
);
invalid_fixture!(
    fixture_bad_escape,
    "fixtures/invalid/bad_escape.extractor",
    "unsupported string escape `\\q`"
);
invalid_fixture!(
    fixture_ampersand,
    "fixtures/invalid/unexpected_ampersand.extractor",
    "unexpected character `&`"
);
invalid_fixture!(
    fixture_pipe,
    "fixtures/invalid/unexpected_pipe.extractor",
    "unexpected character `|`"
);
invalid_fixture!(
    fixture_question_mark,
    "fixtures/invalid/unexpected_question.extractor",
    "unexpected character `?`"
);
