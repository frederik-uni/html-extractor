use std::{
    fmt,
    fs::{self, read_dir},
    future::Future,
    io::{self, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
};

use html_extractor_core::{compile, format};
use html_extractor_runtime::{
    Engine, HttpClient, HttpPolicy, HttpRequest, ReqwestHttpClient, Response,
};

use crate::{
    args::{Args, Command},
    diagnostic::render,
    input::{InputError, StdinInput, build_inputs},
};

#[derive(Debug)]
pub(crate) struct CliError {
    message: String,
    exit_code: u8,
}

impl CliError {
    fn new(message: impl Into<String>, exit_code: u8) -> Self {
        Self {
            message: message.into(),
            exit_code,
        }
    }

    pub(crate) fn stdin(error: io::Error) -> Self {
        Self::new(format!("stdin: error: failed to read input: {error}"), 1)
    }

    #[must_use]
    pub(crate) const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliError {}

pub(crate) async fn run_with_io(
    args: Args,
    stdin: StdinInput,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    match args.command {
        Some(Command::Fmt { path, write }) => run_fmt(path, write, stdout),
        Some(Command::Test { path, reset_cache }) => {
            run_tests(path, reset_cache, stdout, stderr).await
        }
        None => {
            let Some(file) = args.file else {
                return Err(CliError::new("missing extractor file", 2));
            };
            run_extract(file, args.variables, args.variable, stdin, stdout, stderr).await
        }
    }
}

fn run_fmt(file: PathBuf, write: bool, stdout: &mut dyn Write) -> Result<(), CliError> {
    let source = fs::read_to_string(&file).map_err(|error| {
        CliError::new(
            format!(
                "{}: error: failed to read extractor: {error}",
                file.display()
            ),
            1,
        )
    })?;
    let formatted = format(&source).map_err(|error| {
        let messages = error
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                render(
                    &file,
                    &source,
                    "error",
                    diagnostic.message(),
                    Some(diagnostic.primary_span()),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        CliError::new(messages, 1)
    })?;
    if write {
        fs::write(&file, formatted).map_err(|error| {
            CliError::new(
                format!(
                    "{}: error: failed to write extractor: {error}",
                    file.display()
                ),
                1,
            )
        })?;
    } else {
        stdout
            .write_all(formatted.as_bytes())
            .map_err(|error| stream_error("stdout", error))?;
    }
    Ok(())
}

async fn run_extract(
    file: PathBuf,
    variables: Option<String>,
    variable: Vec<String>,
    stdin: StdinInput,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    let source = fs::read_to_string(&file).map_err(|error| {
        CliError::new(
            format!(
                "{}: error: failed to read extractor: {error}",
                file.display()
            ),
            1,
        )
    })?;
    let program = compile_source(&file, &source)?;
    let inputs = build_inputs(stdin, variables.as_deref(), &variable).map_err(variable_error)?;
    let result = engine_for(&file, None)?
        .execute(&program, inputs)
        .await
        .map_err(|error| {
            CliError::new(
                render(&file, &source, "error", error.message(), error.span()),
                1,
            )
        })?;

    for warning in result.warnings() {
        writeln!(
            stderr,
            "{}",
            render(&file, &source, "warning", warning.message(), warning.span(),)
        )
        .map_err(|error| stream_error("stderr", error))?;
    }

    let json = result
        .to_json()
        .map_err(|error| CliError::new(file_error(&file, &format!("{error}")), 1))?;
    let mut output = Vec::new();
    serde_json::to_writer_pretty(&mut output, &json).map_err(|error| {
        CliError::new(
            file_error(&file, &format!("failed to serialize JSON: {error}")),
            1,
        )
    })?;
    output.push(b'\n');
    stdout
        .write_all(&output)
        .map_err(|error| stream_error("stdout", error))?;
    Ok(())
}

async fn run_tests(
    path: PathBuf,
    reset_cache: bool,
    stdout: &mut dyn Write,
    _stderr: &mut dyn Write,
) -> Result<(), CliError> {
    let cache_dir = PathBuf::from(".cache")
        .join("html-extractor")
        .join("requests");
    if reset_cache && cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).map_err(|error| {
            CliError::new(
                format!(
                    "{}: error: failed to reset cache: {error}",
                    cache_dir.display()
                ),
                1,
            )
        })?;
    }
    fs::create_dir_all(&cache_dir).map_err(|error| {
        CliError::new(
            format!(
                "{}: error: failed to create cache: {error}",
                cache_dir.display()
            ),
            1,
        )
    })?;

    let files = extractor_files(&path)?;
    let mut cases = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).map_err(|error| {
            CliError::new(
                format!(
                    "{}: error: failed to read extractor: {error}",
                    file.display()
                ),
                1,
            )
        })?;
        let program = compile_source(file, &source)?;
        cases.push((file.clone(), source, program));
    }

    let total = cases
        .iter()
        .map(|(_, _, program)| program.tests().len())
        .sum::<usize>();
    writeln!(stdout, "running {total} tests").map_err(|error| stream_error("stdout", error))?;
    writeln!(stdout).map_err(|error| stream_error("stdout", error))?;

    let mut passed = 0usize;
    for (file, source, program) in cases {
        let label = test_label(&file);
        write!(stdout, "test {label} ... ").map_err(|error| stream_error("stdout", error))?;
        let report = engine_for(&file, Some(cache_dir.clone()))?
            .test(&program)
            .await;
        match report {
            Ok(report) => {
                passed += report.passed();
                writeln!(stdout, "ok").map_err(|error| stream_error("stdout", error))?;
            }
            Err(error) => {
                writeln!(stdout, "FAILED").map_err(|error| stream_error("stdout", error))?;
                writeln!(stdout).map_err(|error| stream_error("stdout", error))?;
                writeln!(
                    stdout,
                    "failures:\n\n---- {label} stdout ----\n{}",
                    render(&file, &source, "error", error.message(), error.span())
                )
                .map_err(|error| stream_error("stdout", error))?;
                writeln!(stdout).map_err(|error| stream_error("stdout", error))?;
                writeln!(stdout, "test result: FAILED. {passed} passed; 1 failed")
                    .map_err(|error| stream_error("stdout", error))?;
                return Err(CliError::new("test failed", 1));
            }
        }
    }

    writeln!(stdout).map_err(|error| stream_error("stdout", error))?;
    writeln!(
        stdout,
        "test result: ok. {passed} passed; 0 failed; 0 ignored; 0 filtered out"
    )
    .map_err(|error| stream_error("stdout", error))?;
    Ok(())
}

fn compile_source(
    file: &Path,
    source: &str,
) -> Result<html_extractor_core::CompiledProgram, CliError> {
    compile(source).map_err(|error| {
        let messages = error
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                render(
                    file,
                    source,
                    "error",
                    diagnostic.message(),
                    Some(diagnostic.primary_span()),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        CliError::new(messages, 1)
    })
}

fn engine_for(file: &Path, cache_dir: Option<PathBuf>) -> Result<Engine, CliError> {
    let mut builder = Engine::builder();
    if let Some(cache_dir) = cache_dir {
        builder = builder.http_client(Arc::new(CachedHttpClient::new(cache_dir)));
    }
    if let Some(parent) = file.parent() {
        let parent = if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        };
        if let Ok(entries) = read_dir(parent) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.ends_with(".module.lua")
                {
                    let module_name = name.strip_suffix(".module.lua").unwrap();
                    let source_code = fs::read_to_string(&path).map_err(|error| {
                        CliError::new(
                            format!("failed to read Lua module {}: {error}", path.display()),
                            1,
                        )
                    })?;
                    builder = builder.lua_module(module_name, source_code);
                }
            }
        }
    }
    builder
        .build()
        .map_err(|error| CliError::new(render(file, "", "error", error.message(), error.span()), 1))
}

fn extractor_files(path: &Path) -> Result<Vec<PathBuf>, CliError> {
    if path.is_file() {
        return Ok(vec![path.to_owned()]);
    }
    if !path.is_dir() {
        return Err(CliError::new(
            file_error(path, "test path does not exist"),
            1,
        ));
    }
    let mut files = Vec::new();
    collect_extractor_files(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_extractor_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    for entry in read_dir(path).map_err(|error| {
        CliError::new(
            format!(
                "{}: error: failed to read directory: {error}",
                path.display()
            ),
            1,
        )
    })? {
        let entry = entry.map_err(|error| {
            CliError::new(
                format!(
                    "{}: error: failed to read directory entry: {error}",
                    path.display()
                ),
                1,
            )
        })?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some(".cache") {
            continue;
        }
        if path.is_dir() {
            collect_extractor_files(&path, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("extractor") {
            files.push(path);
        }
    }
    Ok(())
}

fn test_label(path: &Path) -> String {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let current = current.canonicalize().unwrap_or(current);
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    canonical_path
        .strip_prefix(&current)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug)]
struct CachedHttpClient {
    inner: ReqwestHttpClient,
    cache_dir: PathBuf,
}

impl CachedHttpClient {
    fn new(cache_dir: PathBuf) -> Self {
        Self {
            inner: ReqwestHttpClient::default(),
            cache_dir,
        }
    }

    fn cache_path(&self, request: &HttpRequest) -> PathBuf {
        self.cache_dir
            .join(format!("{:016x}.json", request_hash(request)))
    }
}

impl HttpClient for CachedHttpClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        policy: &'a HttpPolicy,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Response, html_extractor_runtime::ExecutionError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            if request.cloudflare() {
                return self.inner.fetch(request, policy).await;
            }
            let path = self.cache_path(request);
            if let Ok(response) = read_cached_response(&path) {
                return Ok(response);
            }
            let response = self.inner.fetch(request, policy).await?;
            write_cached_response(&path, &response)?;
            Ok(response)
        })
    }
}

fn request_hash(request: &HttpRequest) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    let method = match request.method() {
        html_extractor_runtime::HttpMethod::Get => "GET",
        html_extractor_runtime::HttpMethod::Post => "POST",
    };
    update_hash(&mut hash, method.as_bytes());
    update_hash(&mut hash, request.url().as_bytes());
    if let Some(content_type) = request.content_type() {
        update_hash(&mut hash, content_type.as_bytes());
    }
    if let Some(body) = request.body() {
        update_hash(&mut hash, body);
    }
    hash
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
}

fn read_cached_response(path: &Path) -> Result<Response, html_extractor_runtime::ExecutionError> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path).map_err(|error| {
        html_extractor_runtime::ExecutionError::new(
            format!("failed to read cached response: {error}"),
            None,
        )
    })?)
    .map_err(|error| {
        html_extractor_runtime::ExecutionError::new(
            format!("failed to parse cached response: {error}"),
            None,
        )
    })?;
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_u64)
        .and_then(|status| u16::try_from(status).ok())
        .ok_or_else(|| {
            html_extractor_runtime::ExecutionError::new("cached response status is invalid", None)
        })?;
    let url = value
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            html_extractor_runtime::ExecutionError::new("cached response url is invalid", None)
        })?;
    let body = value
        .get("body")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            html_extractor_runtime::ExecutionError::new("cached response body is invalid", None)
        })?
        .iter()
        .map(|byte| {
            byte.as_u64()
                .and_then(|byte| u8::try_from(byte).ok())
                .ok_or_else(|| {
                    html_extractor_runtime::ExecutionError::new(
                        "cached response body byte is invalid",
                        None,
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Response::new(status, url, body))
}

fn write_cached_response(
    path: &Path,
    response: &Response,
) -> Result<(), html_extractor_runtime::ExecutionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            html_extractor_runtime::ExecutionError::new(
                format!("failed to create cache directory: {error}"),
                None,
            )
        })?;
    }
    let value = serde_json::json!({
        "status": response.status(),
        "url": response.url(),
        "body": response.body(),
    });
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        html_extractor_runtime::ExecutionError::new(
            format!("failed to encode cached response: {error}"),
            None,
        )
    })?;
    fs::write(path, bytes).map_err(|error| {
        html_extractor_runtime::ExecutionError::new(
            format!("failed to write cached response: {error}"),
            None,
        )
    })
}

fn variable_error(error: InputError) -> CliError {
    CliError::new(error.to_string(), 2)
}

fn stream_error(stream: &str, error: io::Error) -> CliError {
    CliError::new(
        format!("{stream}: error: failed to write output: {error}"),
        1,
    )
}

fn file_error(path: &Path, message: &str) -> String {
    format!("{}: error: {message}", path.display())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::{read_cached_response, run_with_io, write_cached_response};
    use crate::{args::Args, input::StdinInput};

    fn args(path: &Path) -> Args {
        Args {
            command: None,
            file: Some(path.to_owned()),
            variables: None,
            variable: Vec::new(),
        }
    }

    #[tokio::test]
    async fn piped_html_executes_and_writes_pretty_json() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("page.extractor");
        fs::write(&path, "title = html.css(\"title\").text()").unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run_with_io(
            args(&path),
            StdinInput::Redirected("<title>Hello</title>".into()),
            &mut stdout,
            &mut stderr,
        )
        .await
        .unwrap();

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "{\n  \"title\": \"Hello\"\n}\n"
        );
        assert!(stderr.is_empty());
    }

    #[tokio::test]
    async fn terminal_stdin_leaves_html_undefined_without_blocking() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("value.extractor");
        fs::write(&path, "value = 1").unwrap();
        let mut stdout = Vec::new();

        run_with_io(
            args(&path),
            StdinInput::Terminal,
            &mut stdout,
            &mut Vec::new(),
        )
        .await
        .unwrap();

        assert_eq!(String::from_utf8(stdout).unwrap(), "{\n  \"value\": 1\n}\n");
    }

    #[tokio::test]
    async fn explicit_html_variable_overrides_redirected_html() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("override.extractor");
        fs::write(&path, "value = html").unwrap();
        let mut args = args(&path);
        args.variable.push("html=explicit".into());
        let mut stdout = Vec::new();

        run_with_io(
            args,
            StdinInput::Redirected("<p>stdin</p>".into()),
            &mut stdout,
            &mut Vec::new(),
        )
        .await
        .unwrap();

        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "{\n  \"value\": \"explicit\"\n}\n"
        );
    }

    #[tokio::test]
    async fn warnings_are_source_located_on_stderr() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("warning.extractor");
        fs::write(&path, "value = 1.warn(x => false, \"small value\")").unwrap();
        let mut stderr = Vec::new();

        run_with_io(
            args(&path),
            StdinInput::Terminal,
            &mut Vec::new(),
            &mut stderr,
        )
        .await
        .unwrap();

        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("warning.extractor:1:"));
        assert!(stderr.contains("warning: small value"));
    }

    #[tokio::test]
    async fn compile_failure_is_source_located_and_writes_no_stdout() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("invalid.extractor");
        fs::write(&path, "value =").unwrap();
        let mut stdout = Vec::new();

        let error = run_with_io(
            args(&path),
            StdinInput::Terminal,
            &mut stdout,
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.exit_code(), 1);
        assert!(error.to_string().contains("invalid.extractor:1:"));
        assert!(stdout.is_empty());
    }

    #[tokio::test]
    async fn serialization_failure_writes_no_partial_stdout() {
        let fixture = TempDir::new().unwrap();
        let path = fixture.path().join("native.extractor");
        fs::write(&path, "value = html").unwrap();
        let mut stdout = Vec::new();

        let error = run_with_io(
            args(&path),
            StdinInput::Redirected("<p>native</p>".into()),
            &mut stdout,
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert_eq!(error.exit_code(), 1);
        assert!(error.to_string().contains("cannot serialize"));
        assert!(stdout.is_empty());
    }

    #[test]
    fn cached_responses_round_trip_through_persistent_files() {
        let fixture = TempDir::new().unwrap();
        let path = fixture
            .path()
            .join(".cache/html-extractor/requests/response.json");
        let response = html_extractor_runtime::Response::new(
            200,
            "https://example.test/final",
            b"ok".to_vec(),
        );

        write_cached_response(&path, &response).unwrap();
        let cached = read_cached_response(&path).unwrap();

        assert_eq!(cached.status(), 200);
        assert_eq!(cached.url(), "https://example.test/final");
        assert_eq!(cached.body(), b"ok");
    }
}
