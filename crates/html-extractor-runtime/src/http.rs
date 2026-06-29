use std::{
    collections::{BTreeMap, HashSet},
    fmt, fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, OnceLock},
};

use futures::future::BoxFuture;

use crate::{ExecutionError, HtmlDocument, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpRequest {
    url: String,
    method: HttpMethod,
    body: Option<Vec<u8>>,
    content_type: Option<String>,
    headers: Option<Vec<(String, String)>>,
    cloudflare: bool,
}

impl HttpRequest {
    #[must_use]
    pub fn get(url: impl Into<String>, headers: Option<Vec<(String, String)>>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Get,
            body: None,
            content_type: None,
            headers,
            cloudflare: false,
        }
    }

    #[must_use]
    pub fn post(
        url: impl Into<String>,
        body: Option<Vec<u8>>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Post,
            body,
            content_type,
            headers,
            cloudflare: false,
        }
    }

    #[must_use]
    pub const fn with_cloudflare(mut self, cloudflare: bool) -> Self {
        self.cloudflare = cloudflare;
        self
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub const fn method(&self) -> HttpMethod {
        self.method
    }

    #[must_use]
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    #[must_use]
    pub const fn cloudflare(&self) -> bool {
        self.cloudflare
    }
}

#[derive(Clone, Debug)]
pub struct HttpPolicy {
    allowed_schemes: HashSet<String>,
    max_response_bytes: usize,
}

impl Default for HttpPolicy {
    fn default() -> Self {
        Self {
            allowed_schemes: ["http".to_owned(), "https".to_owned()].into(),
            max_response_bytes: 8 * 1024 * 1024,
        }
    }
}

impl HttpPolicy {
    #[must_use]
    pub fn allow_schemes(mut self, schemes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_schemes = schemes.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub const fn max_response_bytes(mut self, bytes: usize) -> Self {
        self.max_response_bytes = bytes;
        self
    }

    pub(crate) fn validate_url(&self, url: &str) -> Result<(), ExecutionError> {
        let parsed = reqwest::Url::parse(url)
            .map_err(|error| ExecutionError::new(format!("invalid URL: {error}"), None))?;
        if self.allowed_schemes.contains(parsed.scheme()) {
            Ok(())
        } else {
            Err(ExecutionError::new(
                format!("URL scheme `{}` is not allowed", parsed.scheme()),
                None,
            ))
        }
    }

    pub(crate) fn validate_response(&self, response: &Response) -> Result<(), ExecutionError> {
        if response.body().len() <= self.max_response_bytes {
            Ok(())
        } else {
            Err(ExecutionError::new(
                format!(
                    "response body exceeds {} byte limit",
                    self.max_response_bytes
                ),
                None,
            ))
        }
    }
}

pub trait HttpClient: fmt::Debug + Send + Sync {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, ExecutionError>>;
}

const CLOUDFLARE_CHALLENGE_PREFIX_BYTES: usize = 512;
const CLOUDFLARE_STORE_ENV: &str = "HTML_EXTRACTOR_CLOUDFLARE_STORE";

fn is_cloudflare_challenge(body: &[u8]) -> bool {
    let prefix = &body[..body.len().min(CLOUDFLARE_CHALLENGE_PREFIX_BYTES)];
    prefix.starts_with(b"<!DOCTYPE html>")
        && prefix
            .windows(b"<title>Just a moment...</title>".len())
            .any(|window| window == b"<title>Just a moment...</title>")
}

#[derive(Debug)]
struct CloudflareStore {
    path: PathBuf,
    domains: BTreeMap<String, CloudflareDomain>,
}

#[derive(Debug, Default)]
struct CloudflareDomain {
    user_agent: Option<String>,
    cookies: BTreeMap<String, String>,
}

impl CloudflareStore {
    fn default_path() -> PathBuf {
        std::env::var_os(CLOUDFLARE_STORE_ENV).map_or_else(
            || PathBuf::from(".html-extractor-cloudflare-cookies.json"),
            PathBuf::from,
        )
    }

    fn load(path: impl Into<PathBuf>) -> Result<Self, ExecutionError> {
        let path = path.into();
        let domains = match fs::read(&path) {
            Ok(bytes) => parse_store(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
            Err(error) => {
                return Err(ExecutionError::new(
                    format!("failed to read Cloudflare cookie store: {error}"),
                    None,
                ));
            }
        };
        Ok(Self { path, domains })
    }

    fn load_default() -> Result<Self, ExecutionError> {
        Self::load(Self::default_path())
    }

    fn save(&self) -> Result<(), ExecutionError> {
        if let Some(parent) = self
            .path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|error| {
                ExecutionError::new(
                    format!("failed to create Cloudflare cookie store directory: {error}"),
                    None,
                )
            })?;
        }
        fs::write(&self.path, render_store(&self.domains)).map_err(|error| {
            ExecutionError::new(
                format!("failed to write Cloudflare cookie store: {error}"),
                None,
            )
        })
    }

    fn user_agent(&self, url: &str) -> Result<Option<&str>, ExecutionError> {
        Ok(self
            .domains
            .get(&domain_key(url)?)
            .and_then(|domain| domain.user_agent.as_deref()))
    }

    fn cookie_header(&self, url: &str) -> Result<Option<String>, ExecutionError> {
        let Some(domain) = self.domains.get(&domain_key(url)?) else {
            return Ok(None);
        };
        if domain.cookies.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            domain
                .cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; "),
        ))
    }

    fn set_bypass(
        &mut self,
        url: &str,
        user_agent: impl Into<String>,
        cf_clearance: impl Into<String>,
    ) -> Result<(), ExecutionError> {
        let domain = self.domains.entry(domain_key(url)?).or_default();
        domain.user_agent = Some(user_agent.into());
        domain
            .cookies
            .insert("cf_clearance".to_string(), cf_clearance.into());
        self.save()
    }

    fn clear_domain(&mut self, url: &str) -> Result<(), ExecutionError> {
        self.domains.remove(&domain_key(url)?);
        self.save()
    }

    fn store_response_cookies(
        &mut self,
        request_url: &str,
        response: &Response,
    ) -> Result<(), ExecutionError> {
        if is_cloudflare_challenge(response.body()) {
            self.clear_domain(request_url)?;
            return Ok(());
        }

        let key = domain_key(response.url()).or_else(|_| domain_key(request_url))?;
        let domain = self.domains.entry(key).or_default();
        for (_, value) in response
            .headers()
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("set-cookie"))
        {
            if let Some((name, value)) = parse_set_cookie(value) {
                domain.cookies.insert(name, value);
            }
        }
        self.save()
    }
}

#[derive(Debug, Eq, PartialEq)]
struct CloudflareBypass {
    cf_clearance: String,
    user_agent: String,
}

fn cloudflare_bypass_python() -> PathBuf {
    PathBuf::from("bypass")
        .join("venv")
        .join("bin")
        .join("python")
}

fn run_cloudflare_bypass(url: &str) -> Result<CloudflareBypass, ExecutionError> {
    let output = Command::new(cloudflare_bypass_python())
        .arg("bypass/bypass.py")
        .arg(url)
        .output()
        .map_err(|error| {
            ExecutionError::new(format!("failed to run Cloudflare bypass: {error}"), None)
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ExecutionError::new(
            format!("Cloudflare bypass failed: {}", stderr.trim()),
            None,
        ));
    }
    parse_bypass_output(&output.stdout)
}

fn parse_bypass_output(stdout: &[u8]) -> Result<CloudflareBypass, ExecutionError> {
    let text = std::str::from_utf8(stdout).map_err(|error| {
        ExecutionError::new(
            format!("Cloudflare bypass output is not UTF-8: {error}"),
            None,
        )
    })?;
    let mut lines = text.lines();
    let cf_clearance = lines.next().unwrap_or_default().trim();
    let user_agent = lines.next().unwrap_or_default().trim();
    if cf_clearance.is_empty() || user_agent.is_empty() {
        return Err(ExecutionError::new(
            "Cloudflare bypass output must contain cf_clearance and user agent",
            None,
        ));
    }
    Ok(CloudflareBypass {
        cf_clearance: cf_clearance.to_string(),
        user_agent: user_agent.to_string(),
    })
}

fn domain_key(url: &str) -> Result<String, ExecutionError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|error| ExecutionError::new(format!("invalid URL: {error}"), None))?;
    parsed
        .host_str()
        .map(|host| host.to_ascii_lowercase())
        .ok_or_else(|| ExecutionError::new("URL has no host", None))
}

fn parse_set_cookie(header: &str) -> Option<(String, String)> {
    let pair = header.split(';').next()?.trim();
    let (name, value) = pair.split_once('=')?;
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), value.to_string()))
}

fn parse_store(bytes: &[u8]) -> Result<BTreeMap<String, CloudflareDomain>, ExecutionError> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        ExecutionError::new(
            format!("failed to parse Cloudflare cookie store: {error}"),
            None,
        )
    })?;
    let mut domains = BTreeMap::new();
    let Some(object) = value.as_object() else {
        return Ok(domains);
    };
    for (host, domain_value) in object {
        let mut domain = CloudflareDomain::default();
        if let Some(user_agent) = domain_value.get("user_agent").and_then(|v| v.as_str()) {
            domain.user_agent = Some(user_agent.to_string());
        }
        if let Some(cookies) = domain_value.get("cookies").and_then(|v| v.as_object()) {
            for (name, value) in cookies {
                if let Some(value) = value.as_str() {
                    domain.cookies.insert(name.clone(), value.to_string());
                }
            }
        }
        domains.insert(host.clone(), domain);
    }
    Ok(domains)
}

fn render_store(domains: &BTreeMap<String, CloudflareDomain>) -> Vec<u8> {
    let mut root = serde_json::Map::new();
    for (host, domain) in domains {
        let mut cookies = serde_json::Map::new();
        for (name, value) in &domain.cookies {
            cookies.insert(name.clone(), serde_json::Value::String(value.clone()));
        }
        let mut object = serde_json::Map::new();
        object.insert(
            "user_agent".to_string(),
            domain
                .user_agent
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        );
        object.insert("cookies".to_string(), serde_json::Value::Object(cookies));
        root.insert(host.clone(), serde_json::Value::Object(object));
    }
    serde_json::to_vec_pretty(&serde_json::Value::Object(root)).unwrap()
}

#[derive(Debug, Default)]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl HttpClient for ReqwestHttpClient {
    fn fetch<'a>(
        &'a self,
        request: &'a HttpRequest,
        _policy: &'a HttpPolicy,
    ) -> BoxFuture<'a, Result<Response, ExecutionError>> {
        Box::pin(async move {
            if !request.cloudflare() {
                return self.send_once(request, None, None).await;
            }

            let mut store = CloudflareStore::load_default()?;
            let stored_user_agent = store.user_agent(request.url())?.map(str::to_owned);
            let stored_cookie = store.cookie_header(request.url())?;
            let response = self
                .send_once(
                    request,
                    stored_user_agent.as_deref(),
                    stored_cookie.as_deref(),
                )
                .await?;

            if !is_cloudflare_challenge(response.body()) {
                store.store_response_cookies(request.url(), &response)?;
                return Ok(response);
            }

            store.clear_domain(request.url())?;
            let bypass = run_cloudflare_bypass(request.url())?;
            store.set_bypass(request.url(), &bypass.user_agent, &bypass.cf_clearance)?;
            let cookie = store.cookie_header(request.url())?;
            let response = self
                .send_once(request, Some(&bypass.user_agent), cookie.as_deref())
                .await?;
            store.store_response_cookies(request.url(), &response)?;
            Ok(response)
        })
    }
}

impl ReqwestHttpClient {
    async fn send_once(
        &self,
        request: &HttpRequest,
        user_agent: Option<&str>,
        cookie: Option<&str>,
    ) -> Result<Response, ExecutionError> {
        let method = match request.method() {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
        };
        let mut builder = self.client.request(method, request.url());
        if let Some(body) = request.body() {
            builder = builder.body(body.to_vec());
        }
        let request_headers = request.headers.as_deref().unwrap_or_default();
        if let Some(user_agent) = user_agent {
            builder = builder.header(reqwest::header::USER_AGENT, user_agent);
        } else if !request_headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
        {
            builder = builder.header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.3");
        }
        if let Some(cookie) = cookie {
            builder = builder.header(reqwest::header::COOKIE, cookie);
        }
        for (key, value) in request_headers {
            if user_agent.is_some() && key.eq_ignore_ascii_case("user-agent") {
                continue;
            }
            if cookie.is_some() && key.eq_ignore_ascii_case("cookie") {
                continue;
            }
            builder = builder.header(key, value);
        }
        if let Some(content_type) = request.content_type() {
            builder = builder.header(reqwest::header::CONTENT_TYPE, content_type);
        }
        let response = builder.send().await.map_err(|error| {
            ExecutionError::new(format!("HTTP transport failed: {error}"), None)
        })?;
        let status = response.status().as_u16();
        let final_url = response.url().to_string();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                Some((name.as_str().to_string(), value.to_str().ok()?.to_string()))
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(|error| {
                ExecutionError::new(format!("failed reading response: {error}"), None)
            })?
            .to_vec();
        Ok(Response::new_with_headers(status, final_url, body, headers))
    }
}

#[derive(Clone)]
pub struct Response(Arc<ResponseInner>);

struct ResponseInner {
    status: u16,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    text: OnceLock<Result<String, String>>,
    json: OnceLock<Result<serde_json::Value, String>>,
    html: OnceLock<HtmlDocument>,
}

impl Response {
    #[must_use]
    pub fn new(status: u16, url: impl Into<String>, body: Vec<u8>) -> Self {
        Self::new_with_headers(status, url, body, Vec::new())
    }

    #[must_use]
    pub fn new_with_headers(
        status: u16,
        url: impl Into<String>,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
    ) -> Self {
        Self(Arc::new(ResponseInner {
            status,
            url: url.into(),
            headers,
            body,
            text: OnceLock::new(),
            json: OnceLock::new(),
            html: OnceLock::new(),
        }))
    }

    #[must_use]
    pub fn status(&self) -> u16 {
        self.0.status
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.0.url
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.0.body
    }

    #[must_use]
    pub fn headers(&self) -> &[(String, String)] {
        &self.0.headers
    }

    pub fn text(&self) -> Result<&str, ExecutionError> {
        self.0
            .text
            .get_or_init(|| {
                String::from_utf8(self.0.body.clone()).map_err(|error| error.to_string())
            })
            .as_deref()
            .map_err(|error| ExecutionError::new(format!("response is not UTF-8: {error}"), None))
    }

    pub fn json(&self) -> Result<&serde_json::Value, ExecutionError> {
        self.0
            .json
            .get_or_init(|| serde_json::from_slice(&self.0.body).map_err(|error| error.to_string()))
            .as_ref()
            .map_err(|error| ExecutionError::new(format!("invalid response JSON: {error}"), None))
    }

    pub fn html(&self) -> Result<&HtmlDocument, ExecutionError> {
        let text = self
            .text()?
            .replace("<noscript>", "<notscript>")
            .replace("</noscript>", "</notscript>");
        Ok(self.0.html.get_or_init(|| HtmlDocument::parse(text)))
    }
}

impl fmt::Debug for Response {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Response")
            .field("status", &self.status())
            .field("url", &self.url())
            .field("body_len", &self.body().len())
            .finish()
    }
}

impl PartialEq for Response {
    fn eq(&self, other: &Self) -> bool {
        self.status() == other.status() && self.url() == other.url() && self.body() == other.body()
    }
}

pub(crate) fn json_to_value(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Boolean(*value),
        serde_json::Value::Number(value) => Value::Number(value.clone()),
        serde_json::Value::String(value) => Value::String(value.clone()),
        serde_json::Value::Array(values) => {
            Value::Array(values.iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(values) => {
            let mut object = crate::Object::new();
            for (name, value) in values {
                object.insert(name, json_to_value(value));
            }
            Value::Object(object)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn store_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("html-extractor-{name}-{}.json", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn cloudflare_challenge_detection_uses_fixed_prefix_buffer() {
        let challenge =
            b"<!DOCTYPE html><html lang=\"en-US\"><head><title>Just a moment...</title><meta";
        assert!(is_cloudflare_challenge(challenge));
        assert!(!is_cloudflare_challenge(
            b"<html><head><title>Just a moment...</title>"
        ));

        let mut late_title = b"<!DOCTYPE html>".to_vec();
        late_title.extend(vec![b' '; 600]);
        late_title.extend(b"<title>Just a moment...</title>");
        assert!(!is_cloudflare_challenge(&late_title));
    }

    #[test]
    fn cloudflare_store_persists_user_agent_and_cookies_by_domain() {
        let path = store_path("persist");
        let mut store = CloudflareStore::load(path.clone()).unwrap();
        store
            .set_bypass(
                "https://fixture.test/items",
                "Mozilla/5.0 Test",
                "clearance-token",
            )
            .unwrap();
        store.save().unwrap();

        let store = CloudflareStore::load(path.clone()).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(
            store
                .user_agent("https://fixture.test/other")
                .unwrap()
                .unwrap(),
            "Mozilla/5.0 Test"
        );
        assert_eq!(
            store
                .cookie_header("https://fixture.test/other")
                .unwrap()
                .unwrap(),
            "cf_clearance=clearance-token"
        );
    }

    #[test]
    fn cloudflare_store_skips_challenge_set_cookie_and_clears_domain() {
        let path = store_path("challenge");
        let mut store = CloudflareStore::load(path.clone()).unwrap();
        store
            .set_bypass("https://fixture.test/items", "ua", "old")
            .unwrap();
        let challenge = Response::new_with_headers(
            403,
            "https://fixture.test/items",
            b"<!DOCTYPE html><html><head><title>Just a moment...</title>".to_vec(),
            vec![(
                "set-cookie".to_string(),
                "cf_clearance=bad; Path=/".to_string(),
            )],
        );

        store
            .store_response_cookies("https://fixture.test/items", &challenge)
            .unwrap();

        assert!(
            store
                .cookie_header("https://fixture.test/items")
                .unwrap()
                .is_none()
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn cloudflare_store_updates_set_cookie_for_non_challenge_response() {
        let path = store_path("update");
        let mut store = CloudflareStore::load(path.clone()).unwrap();
        store
            .set_bypass("https://fixture.test/items", "ua", "old")
            .unwrap();
        let response = Response::new_with_headers(
            200,
            "https://fixture.test/items",
            b"ok".to_vec(),
            vec![
                (
                    "set-cookie".to_string(),
                    "cf_clearance=new; Path=/".to_string(),
                ),
                ("set-cookie".to_string(), "session=abc; Path=/".to_string()),
            ],
        );

        store
            .store_response_cookies("https://fixture.test/items", &response)
            .unwrap();
        store.save().unwrap();
        let store = CloudflareStore::load(path.clone()).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!(
            store
                .cookie_header("https://fixture.test/items")
                .unwrap()
                .unwrap(),
            "cf_clearance=new; session=abc"
        );
    }

    #[test]
    fn parses_bypass_stdout_as_cookie_then_user_agent() {
        let output = parse_bypass_output(b"clearance-token\nMozilla/5.0 Test\n").unwrap();

        assert_eq!(
            output,
            CloudflareBypass {
                cf_clearance: "clearance-token".to_string(),
                user_agent: "Mozilla/5.0 Test".to_string(),
            }
        );
    }

    #[test]
    fn cloudflare_bypass_uses_repo_venv_python() {
        assert_eq!(
            cloudflare_bypass_python(),
            PathBuf::from("bypass")
                .join("venv")
                .join("bin")
                .join("python")
        );
    }
}
