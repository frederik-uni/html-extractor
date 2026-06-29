use std::{fs, path::Path};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn command_for(source: &str) -> (TempDir, Command) {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("test.extractor");
    fs::write(&extractor, source).unwrap();
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"));
    command.arg(extractor);
    (fixture, command)
}

#[test]
fn piped_html_and_variables_produce_json() {
    let (_fixture, mut command) =
        command_for("title = html.css(\"title\").text()\noutput_limit = limit");

    command
        .write_stdin("<title>Hello</title>")
        .args(["--variables", r#"{"limit":1}"#, "--variable", "limit=2"])
        .assert()
        .success()
        .stdout("{\n  \"title\": \"Hello\",\n  \"output_limit\": 2\n}\n")
        .stderr("");
}

#[test]
fn empty_redirected_stdin_defines_html() {
    let (_fixture, mut command) = command_for("value = html.css@(\"p\").text()");

    command
        .write_stdin("")
        .assert()
        .success()
        .stdout("{\n  \"value\": []\n}\n")
        .stderr("");
}

#[test]
fn malformed_assignment_uses_status_two() {
    let (_fixture, mut command) = command_for("value = 1");

    command
        .args(["--variable", "missing_equals"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("NAME=VALUE"));
}

#[test]
fn variables_must_be_a_json_object() {
    let (_fixture, mut command) = command_for("value = 1");

    command
        .args(["--variables", "[]"])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("expected a JSON object"));
}

#[test]
fn last_repeated_variable_wins() {
    let (_fixture, mut command) = command_for("output = value");

    command
        .args([
            "--variables",
            r#"{"value":1}"#,
            "--variable",
            "value=2",
            "--variable",
            "value=3",
        ])
        .assert()
        .success()
        .stdout("{\n  \"output\": 3\n}\n")
        .stderr("");
}

#[test]
fn warnings_stay_on_stderr() {
    let (_fixture, mut command) = command_for("value = 1.warn(x => false, \"small value\")");

    command
        .assert()
        .success()
        .stdout("{\n  \"value\": 1\n}\n")
        .stderr(
            predicate::str::contains("warning: small value")
                .and(predicate::str::contains("test.extractor:1:")),
        );
}

#[test]
fn compile_and_runtime_failures_use_status_one() {
    let (_compile_fixture, mut compile_command) = command_for("value =");
    compile_command
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("test.extractor:1:"));

    let (_runtime_fixture, mut runtime_command) = command_for("value = missing");
    runtime_command
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("missing required input `missing`"));
}

#[test]
fn missing_extractor_path_is_a_clap_error() {
    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn unreadable_extractor_uses_status_one() {
    let fixture = TempDir::new().unwrap();
    let missing = fixture.path().join("missing.extractor");

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .arg(&missing)
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(format!(
            "{}: error: failed to read extractor",
            Path::new(&missing).display()
        )));
}

#[test]
fn fmt_command_formats_valid_extractors() {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("messy.extractor");
    fs::write(&extractor, "value=html.css(\"title\").text()").unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["fmt", extractor.to_str().unwrap()])
        .assert()
        .success()
        .stdout("value = html.css(\"title\").text()\n")
        .stderr("");
}

#[test]
fn fmt_command_writes_valid_extractors_in_place() {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("messy.extractor");
    fs::write(&extractor, "value=html.css(\"title\").text()").unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["fmt", "--write", extractor.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(
        fs::read_to_string(&extractor).unwrap(),
        "value = html.css(\"title\").text()\n"
    );
}

#[test]
fn fmt_command_short_writes_valid_extractors_in_place() {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("messy.extractor");
    fs::write(&extractor, "value=html.css(\"title\").text()").unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["fmt", "-w", extractor.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(
        fs::read_to_string(&extractor).unwrap(),
        "value = html.css(\"title\").text()\n"
    );
}

#[test]
fn fmt_command_rejects_invalid_extractors_without_stdout() {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("broken.extractor");
    fs::write(&extractor, "value =").unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["fmt", extractor.to_str().unwrap()])
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("broken.extractor:1:"));
}

#[test]
fn test_command_runs_all_tests_in_one_file() {
    let fixture = TempDir::new().unwrap();
    let extractor = fixture.path().join("tested.extractor");
    fs::write(
        &extractor,
        r#"
        value = url
        test(url = "https://example.test") {
            assertEq("https://example.test", it.value, "url")
        }
        test {
            assertEq(1 + 1, 2, "math")
        }
        "#,
    )
    .unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["test", extractor.to_str().unwrap()])
        .current_dir(fixture.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("running 2 tests")
                .and(predicate::str::contains("test tested.extractor ... ok"))
                .and(predicate::str::contains(
                    "test result: ok. 2 passed; 0 failed",
                )),
        )
        .stderr("");
}

#[test]
fn test_command_runs_extractor_files_in_a_folder() {
    let fixture = TempDir::new().unwrap();
    fs::write(
        fixture.path().join("first.extractor"),
        r#"test { assertEq(1, 1, "one") }"#,
    )
    .unwrap();
    let nested = fixture.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(
        nested.join("second.extractor"),
        r#"test { assertEq(2, 2, "two") }"#,
    )
    .unwrap();
    fs::write(nested.join("ignored.txt"), "test { assertEq(0, 1) }").unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["test", fixture.path().to_str().unwrap()])
        .current_dir(fixture.path())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("running 2 tests")
                .and(predicate::str::contains("test first.extractor ... ok"))
                .and(predicate::str::contains(
                    "test nested/second.extractor ... ok",
                ))
                .and(predicate::str::contains(
                    "test result: ok. 2 passed; 0 failed",
                )),
        )
        .stderr("");
}

#[test]
fn test_command_resets_persistent_cache_folder() {
    let fixture = TempDir::new().unwrap();
    let cache_file = fixture
        .path()
        .join(".cache/html-extractor/requests/old.json");
    fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
    fs::write(&cache_file, "stale").unwrap();
    let extractor = fixture.path().join("tested.extractor");
    fs::write(&extractor, r#"test { assertEq(1, 1, "one") }"#).unwrap();

    Command::new(assert_cmd::cargo::cargo_bin!("html-extractor"))
        .args(["test", extractor.to_str().unwrap(), "--reset-cache"])
        .current_dir(fixture.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "test result: ok. 1 passed; 0 failed",
        ))
        .stderr("");

    assert!(!cache_file.exists());
    assert!(
        fixture
            .path()
            .join(".cache/html-extractor/requests")
            .exists()
    );
}
