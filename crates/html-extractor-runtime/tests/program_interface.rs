use html_extractor_core::compile;
use html_extractor_runtime::Program;

fn inspect(program: &dyn Program) -> (usize, usize, usize) {
    (
        program.functions().len(),
        program.statements().len(),
        program.requirements().inputs().len(),
    )
}

#[test]
fn runtime_accepts_a_compiled_program_through_its_read_only_interface() {
    let program =
        compile("fun identity(value) { value } title = html.css(\"h1\")!.text()").unwrap();

    assert_eq!(inspect(&program), (1, 1, 1));
    assert_eq!(
        Program::source(&program),
        "fun identity(value) { value } title = html.css(\"h1\")!.text()"
    );
}

#[tokio::test]
async fn test_blocks_with_inputs_run_script_and_bare_test_blocks_do_not() {
    let program = compile(
        r#"
        value = url
        fun double(value) { value * 2 }

        test(url = "https://example.test", _search = "") {
            assertEq(url, it.value, "script variables are visible")
            assertEq("https://example.test", it.value, "script output is bound to it")
        }

        test {
            assertEq(double(2), 4, "bare tests can exercise functions")
        }
        "#,
    )
    .unwrap();

    let report = html_extractor_runtime::Engine::new()
        .test(&program)
        .await
        .unwrap();

    assert_eq!(report.total(), 2);
    assert_eq!(report.passed(), 2);
}
