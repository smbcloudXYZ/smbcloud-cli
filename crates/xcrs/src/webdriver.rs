//! A small W3C WebDriver client for the desktop target.
//!
//! It talks plain JSON over HTTP with `reqwest` to a WebDriver endpoint such as
//! `tauri-driver`, which launches a Tauri application and proxies the platform's native
//! driver (WebKitWebDriver on Linux, msedgedriver on Windows). No client library is used.
//!
//! One session per driver URL is kept for the life of the process. `tauri-driver` starts the
//! application when a session is created, so the session is the launched app.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Mutex;

/// Where `tauri-driver` listens unless told otherwise.
pub const DEFAULT_WEBDRIVER_URL: &str = "http://127.0.0.1:4444";

/// The most elements `ui_element_list` returns, so a large page cannot flood the agent.
const MAX_ELEMENTS: usize = 200;

/// The element key the W3C spec uses in element references.
const ELEMENT_KEY: &str = "element-6066-11e4-a52e-4f735466cecf";

/// Interactive elements and labelled landmarks, in document order.
const LIST_ELEMENTS_SCRIPT: &str = r#"
const selector = 'a[href], button, input, select, textarea, summary, [role], [tabindex], [aria-label], [data-testid]';
const visible = (el) => {
  const r = el.getBoundingClientRect();
  const s = getComputedStyle(el);
  return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none';
};
const out = [];
for (const el of document.querySelectorAll(selector)) {
  if (!visible(el)) continue;
  const r = el.getBoundingClientRect();
  const label = (el.getAttribute('aria-label') || el.getAttribute('title') || el.innerText || el.value || el.getAttribute('placeholder') || '').trim().replace(/\s+/g, ' ').slice(0, 120);
  const id = el.id || el.getAttribute('name') || el.getAttribute('data-testid') || '';
  if (!label && !id) continue;
  out.push({
    tag: el.tagName.toLowerCase(),
    role: el.getAttribute('role') || '',
    id,
    label,
    type: el.getAttribute('type') || '',
    disabled: el.disabled === true || el.getAttribute('aria-disabled') === 'true',
    rect: { x: Math.round(r.x), y: Math.round(r.y), width: Math.round(r.width), height: Math.round(r.height) },
  });
}
return out;
"#;

fn sessions() -> &'static Mutex<HashMap<String, String>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A client for one WebDriver endpoint.
#[derive(Debug, Clone)]
pub struct WebDriver {
    base: String,
    http: reqwest::Client,
}

/// Quote a string as an XPath 1.0 literal. XPath has no escape, so a value holding both quote
/// kinds is built with `concat`.
pub fn xpath_literal(value: &str) -> String {
    if !value.contains('\'') {
        return format!("'{value}'");
    }
    if !value.contains('"') {
        return format!("\"{value}\"");
    }
    let parts: Vec<String> = value.split('\'').map(|part| format!("'{part}'")).collect();
    format!("concat({})", parts.join(", \"'\", "))
}

/// The XPath that finds an element by the label forms `ui_tap` accepts: id, name, test id,
/// ARIA label, title, or its exact visible text.
pub fn element_xpath(label: &str) -> String {
    let literal = xpath_literal(label);
    format!(
        "//*[@id={literal} or @name={literal} or @data-testid={literal} or @aria-label={literal} \
         or @title={literal} or normalize-space(text())={literal}]"
    )
}

impl WebDriver {
    /// Build a client. `url` defaults to [`DEFAULT_WEBDRIVER_URL`]; only http and https are
    /// accepted.
    pub fn new(url: Option<&str>) -> Result<Self> {
        let base = url
            .unwrap_or(DEFAULT_WEBDRIVER_URL)
            .trim()
            .trim_end_matches('/');
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            bail!("webdriver_url must start with http:// or https://, got {base:?}");
        }
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("building the HTTP client")?;
        Ok(Self {
            base: base.to_string(),
            http,
        })
    }

    pub fn url(&self) -> &str {
        &self.base
    }

    /// Send one WebDriver command and return its `value`. A non-2xx answer becomes an error
    /// carrying the driver's `error` code and message.
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base, path);
        let mut request = self.http.request(method.clone(), &url);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("{method} {url}: is a WebDriver server listening?"))?;
        let status = response.status();
        let raw = response.text().await.unwrap_or_default();
        let json: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let value = json.get("value").cloned().unwrap_or(Value::Null);
        if !status.is_success() {
            if value.get("error").and_then(Value::as_str).is_none() {
                // Not WebDriver JSON (a proxy page, a plain-text refusal): the body is the clue.
                let body: String = raw.trim().chars().take(300).collect();
                bail!("webdriver error ({status}): {body}");
            }
            let code = value
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("error");
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("no message");
            bail!("webdriver {code} ({status}): {message}");
        }
        Ok(value)
    }

    /// Whether the driver answers `/status`.
    pub async fn is_ready(&self) -> bool {
        self.http
            .get(format!("{}/status", self.base))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
    }

    /// Start the application and remember the session. Replaces a session already held for
    /// this URL, deleting it first.
    pub async fn start_session(&self, application: &str) -> Result<String> {
        let application = application.trim();
        if application.is_empty() {
            bail!("the application path is empty");
        }
        let _ = self.end_session().await;
        let value = self
            .send(
                reqwest::Method::POST,
                "/session",
                Some(json!({
                    "capabilities": { "alwaysMatch": { "tauri:options": { "application": application } } }
                })),
            )
            .await?;
        let id = value
            .get("sessionId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("the driver's answer has no sessionId"))?
            .to_string();
        sessions()
            .lock()
            .await
            .insert(self.base.clone(), id.clone());
        Ok(id)
    }

    /// Delete the held session, if any. Returns whether one existed.
    pub async fn end_session(&self) -> Result<bool> {
        // Delete first and forget the id only after the driver agrees, so a failed delete can be
        // retried with app_terminate.
        let id = sessions().lock().await.get(&self.base).cloned();
        match id {
            Some(id) => {
                let deleted = self
                    .send(reqwest::Method::DELETE, &format!("/session/{id}"), None)
                    .await;
                if let Err(error) = &deleted {
                    // A session the driver no longer knows is gone; anything else stays held.
                    if !error.to_string().contains("invalid session id") {
                        deleted?;
                    }
                }
                sessions().lock().await.remove(&self.base);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    async fn session(&self) -> Result<String> {
        sessions()
            .lock()
            .await
            .get(&self.base)
            .cloned()
            .ok_or_else(|| {
                anyhow!(
                    "no desktop session on {}: start the app with app_install_launch (app_path) \
                 or app_launch (app_id is the binary path) first",
                    self.base
                )
            })
    }

    async fn on_session(
        &self,
        method: reqwest::Method,
        suffix: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let id = self.session().await?;
        self.send(method, &format!("/session/{id}{suffix}"), body)
            .await
    }

    /// The current document as HTML.
    pub async fn page_source(&self) -> Result<String> {
        let value = self
            .on_session(reqwest::Method::GET, "/source", None)
            .await?;
        value
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("the page source is not a string"))
    }

    /// Visible interactive elements with a label or id, capped at `MAX_ELEMENTS`.
    pub async fn list_elements(&self) -> Result<Vec<Value>> {
        let value = self
            .on_session(
                reqwest::Method::POST,
                "/execute/sync",
                Some(json!({ "script": LIST_ELEMENTS_SCRIPT, "args": [] })),
            )
            .await?;
        let mut elements = value.as_array().cloned().unwrap_or_default();
        elements.truncate(MAX_ELEMENTS);
        Ok(elements)
    }

    /// Find the displayed elements for a label. Returns their element references.
    async fn displayed_matches(&self, label: &str) -> Result<Vec<String>> {
        let found = self
            .on_session(
                reqwest::Method::POST,
                "/elements",
                Some(json!({ "using": "xpath", "value": element_xpath(label) })),
            )
            .await?;
        let mut shown = Vec::new();
        for item in found.as_array().cloned().unwrap_or_default() {
            let Some(reference) = item.get(ELEMENT_KEY).and_then(Value::as_str) else {
                continue;
            };
            let displayed = self
                .on_session(
                    reqwest::Method::GET,
                    &format!("/element/{reference}/displayed"),
                    None,
                )
                .await?;
            if displayed.as_bool().unwrap_or(false) {
                shown.push(reference.to_string());
            }
        }
        Ok(shown)
    }

    /// Click the one displayed element that matches `label`. Zero or several matches are an
    /// error, so a vague label never clicks the wrong thing.
    pub async fn click_label(&self, label: &str) -> Result<()> {
        let matches = self.displayed_matches(label).await?;
        match matches.as_slice() {
            [one] => {
                self.on_session(
                    reqwest::Method::POST,
                    &format!("/element/{one}/click"),
                    Some(json!({})),
                )
                .await?;
                Ok(())
            }
            [] => bail!("no displayed element matches {label:?}"),
            many => bail!(
                "{} displayed elements match {label:?}; use a more specific label",
                many.len()
            ),
        }
    }

    /// Type into the element that has focus.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        let active = self
            .on_session(reqwest::Method::GET, "/element/active", None)
            .await?;
        let reference = active
            .get(ELEMENT_KEY)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("no element has focus; tap a field first"))?
            .to_string();
        self.on_session(
            reqwest::Method::POST,
            &format!("/element/{reference}/value"),
            // W3C `text`, plus the legacy `value` array: tauri-driver can drop `text` when it
            // proxies to WebKitWebDriver, which then rejects the request.
            Some(json!({ "text": text, "value": text.chars().map(String::from).collect::<Vec<_>>() })),
        )
        .await?;
        Ok(())
    }

    /// A PNG screenshot of the window.
    pub async fn screenshot_png(&self) -> Result<Vec<u8>> {
        let value = self
            .on_session(reqwest::Method::GET, "/screenshot", None)
            .await?;
        let encoded = value
            .as_str()
            .ok_or_else(|| anyhow!("the screenshot is not a string"))?;
        decode_base64(encoded)
    }

    /// The screenshot, still base64, for callers that pass it straight on.
    pub async fn screenshot_base64(&self) -> Result<String> {
        let value = self
            .on_session(reqwest::Method::GET, "/screenshot", None)
            .await?;
        value
            .as_str()
            .map(|s| s.trim().to_string())
            .ok_or_else(|| anyhow!("the screenshot is not a string"))
    }
}

fn decode_base64(encoded: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .context("decoding the screenshot")
}

/// The capability map for a desktop target, in the shape `device_capabilities` returns for the
/// other targets.
pub fn desktop_capabilities() -> Value {
    json!({
        "platform": "desktop",
        "backend": "w3c-webdriver",
        "supported": [
            "app_install_launch", "app_launch", "app_terminate", "screen_capture",
            "ui_describe", "ui_element_list", "ui_tap", "input_text"
        ],
        "unsupported": [
            "url_open", "input_tap", "input_swipe", "input_button", "input_click",
            "input_spatial_tap", "orientation_get", "orientation_set", "posture_get", "posture_set",
            "input_key", "input_drag", "input_scroll", "window_resize"
        ],
        "notes": "ui_describe returns the page source. app_launch and app_install_launch take the application binary path and start a WebDriver session, which launches the app. The app cannot be attached to once running."
    })
}

/// The error for a tool with no desktop meaning.
pub fn unsupported(tool: &str) -> String {
    format!(
        "{tool} has no meaning on a desktop target. Supported on desktop: app_install_launch, \
         app_launch, app_terminate, screen_capture, ui_describe, ui_element_list, ui_tap, input_text."
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// A recorded request: method, path, body.
    type Seen = Arc<StdMutex<Vec<(String, String, String)>>>;

    /// A stub W3C WebDriver server. `route` maps (method, path) to a status and a JSON body.
    async fn stub<F>(route: F) -> (String, Seen)
    where
        F: Fn(&str, &str, &str) -> (u16, Value) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen: Seen = Arc::new(StdMutex::new(Vec::new()));
        let recorded = seen.clone();
        let route = Arc::new(route);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let recorded = recorded.clone();
                let route = route.clone();
                tokio::spawn(async move {
                    let mut data = Vec::new();
                    let mut buf = [0u8; 4096];
                    let (head_end, length) = loop {
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        if n == 0 {
                            return;
                        }
                        data.extend_from_slice(&buf[..n]);
                        let text = String::from_utf8_lossy(&data).to_string();
                        if let Some(pos) = text.find("\r\n\r\n") {
                            let length = text[..pos]
                                .lines()
                                .find_map(|l| {
                                    l.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                                })
                                .unwrap_or(0);
                            break (pos + 4, length);
                        }
                    };
                    while data.len() < head_end + length {
                        let n = socket.read(&mut buf).await.unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        data.extend_from_slice(&buf[..n]);
                    }
                    let text = String::from_utf8_lossy(&data).to_string();
                    let mut first = text.lines().next().unwrap_or("").split_whitespace();
                    let method = first.next().unwrap_or("").to_string();
                    let path = first.next().unwrap_or("").to_string();
                    let body = text[head_end..].to_string();
                    recorded
                        .lock()
                        .unwrap()
                        .push((method.clone(), path.clone(), body.clone()));
                    let (status, value) = route(&method, &path, &body);
                    let payload = value.to_string();
                    let reply = format!(
                        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{payload}",
                        payload.len()
                    );
                    let _ = socket.write_all(reply.as_bytes()).await;
                });
            }
        });
        (url, seen)
    }

    fn ok(value: Value) -> (u16, Value) {
        (200, json!({ "value": value }))
    }

    #[test]
    fn xpath_literal_quotes_each_kind() {
        assert_eq!(xpath_literal("Sign in"), "'Sign in'");
        assert_eq!(xpath_literal("it's"), "\"it's\"");
        assert_eq!(xpath_literal("a'b\"c"), "concat('a', \"'\", 'b\"c')");
    }

    #[test]
    fn rejects_a_non_http_url() {
        assert!(WebDriver::new(Some("ftp://x")).is_err());
        assert!(WebDriver::new(Some("localhost:4444")).is_err());
        assert_eq!(
            WebDriver::new(Some("http://h:1/")).unwrap().url(),
            "http://h:1"
        );
        assert_eq!(WebDriver::new(None).unwrap().url(), DEFAULT_WEBDRIVER_URL);
    }

    #[tokio::test]
    async fn a_call_without_a_session_says_how_to_start_one() {
        let (url, _) = stub(|_, _, _| ok(Value::Null)).await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        let error = driver.page_source().await.unwrap_err().to_string();
        assert!(error.contains("app_install_launch"), "{error}");
    }

    #[tokio::test]
    async fn session_source_screenshot_and_end() {
        let (url, seen) = stub(|method, path, _| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s1", "capabilities": {} })),
            ("GET", "/session/s1/source") => ok(json!("<html><body>hello</body></html>")),
            ("GET", "/session/s1/screenshot") => ok(json!("aGVsbG8=")),
            ("DELETE", "/session/s1") => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        assert_eq!(driver.start_session("/opt/app").await.unwrap(), "s1");
        assert_eq!(
            driver.page_source().await.unwrap(),
            "<html><body>hello</body></html>"
        );
        assert_eq!(driver.screenshot_png().await.unwrap(), b"hello");
        assert!(driver.end_session().await.unwrap());
        assert!(!driver.end_session().await.unwrap());

        let seen = seen.lock().unwrap();
        let create = seen
            .iter()
            .find(|(m, p, _)| m == "POST" && p == "/session")
            .unwrap();
        let body: Value = serde_json::from_str(&create.2).unwrap();
        assert_eq!(
            body["capabilities"]["alwaysMatch"]["tauri:options"]["application"],
            "/opt/app"
        );
    }

    #[tokio::test]
    async fn a_driver_error_carries_its_code_and_message() {
        let (url, _) = stub(|_, _, _| {
            (
                500,
                json!({ "value": { "error": "session not created", "message": "binary missing" } }),
            )
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        let error = driver.start_session("/nope").await.unwrap_err().to_string();
        assert!(
            error.contains("session not created") && error.contains("binary missing"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn click_label_clicks_exactly_one_displayed_match() {
        let key = ELEMENT_KEY;
        let (url, seen) = stub(move |method, path, body| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s2" })),
            ("POST", "/session/s2/elements") => {
                let using: Value = serde_json::from_str(body).unwrap();
                assert_eq!(using["using"], "xpath");
                assert!(using["value"].as_str().unwrap().contains("'Sign in'"));
                ok(json!([{ key: "e1" }, { key: "e2" }]))
            }
            ("GET", "/session/s2/element/e1/displayed") => ok(json!(true)),
            ("GET", "/session/s2/element/e2/displayed") => ok(json!(false)),
            ("POST", "/session/s2/element/e1/click") => ok(Value::Null),
            ("DELETE", "/session/s2") => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        driver.click_label("Sign in").await.unwrap();
        assert!(seen
            .lock()
            .unwrap()
            .iter()
            .any(|(m, p, _)| m == "POST" && p == "/session/s2/element/e1/click"));
        driver.end_session().await.unwrap();
    }

    #[tokio::test]
    async fn click_label_refuses_zero_and_several_matches() {
        let key = ELEMENT_KEY;
        let (url, _) = stub(move |method, path, body| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s3" })),
            ("POST", "/session/s3/elements") => {
                if body.contains("Many") {
                    ok(json!([{ key: "a" }, { key: "b" }]))
                } else {
                    ok(json!([]))
                }
            }
            (_, p) if p.ends_with("/displayed") => ok(json!(true)),
            ("DELETE", _) => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        let none = driver.click_label("Nothing").await.unwrap_err().to_string();
        assert!(none.contains("no displayed element"), "{none}");
        let many = driver.click_label("Many").await.unwrap_err().to_string();
        assert!(many.contains("2 displayed elements"), "{many}");
        driver.end_session().await.unwrap();
    }

    #[tokio::test]
    async fn type_text_goes_to_the_focused_element() {
        let key = ELEMENT_KEY;
        let (url, seen) = stub(move |method, path, _| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s4" })),
            ("GET", "/session/s4/element/active") => ok(json!({ key: "f1" })),
            ("POST", "/session/s4/element/f1/value") => ok(Value::Null),
            ("DELETE", _) => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        driver.type_text("hello").await.unwrap();
        driver.end_session().await.unwrap();
        let seen = seen.lock().unwrap();
        let typed = seen
            .iter()
            .find(|(_, p, _)| p.ends_with("/f1/value"))
            .unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&typed.2).unwrap()["text"],
            "hello"
        );
    }

    #[tokio::test]
    async fn list_elements_returns_the_scripts_rows_up_to_the_cap() {
        let rows: Vec<Value> = (0..250)
            .map(|i| json!({ "tag": "button", "label": format!("b{i}") }))
            .collect();
        let (url, seen) = stub(move |method, path, _| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s5" })),
            ("POST", "/session/s5/execute/sync") => ok(json!(rows)),
            ("DELETE", _) => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        let elements = driver.list_elements().await.unwrap();
        assert_eq!(elements.len(), MAX_ELEMENTS);
        assert!(seen
            .lock()
            .unwrap()
            .iter()
            .any(|(_, p, b)| p.ends_with("/execute/sync") && b.contains("querySelectorAll")));
        driver.end_session().await.unwrap();
    }

    #[tokio::test]
    async fn type_text_sends_both_the_text_and_the_legacy_value_array() {
        let key = ELEMENT_KEY;
        let (url, seen) = stub(move |method, path, _| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s6" })),
            ("GET", "/session/s6/element/active") => ok(json!({ key: "f1" })),
            ("POST", "/session/s6/element/f1/value") => ok(Value::Null),
            ("DELETE", _) => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        driver.type_text("ab").await.unwrap();
        driver.end_session().await.unwrap();
        let seen = seen.lock().unwrap();
        let typed = seen
            .iter()
            .find(|(_, p, _)| p.ends_with("/f1/value"))
            .unwrap();
        let body: Value = serde_json::from_str(&typed.2).unwrap();
        assert_eq!(body["text"], "ab");
        assert_eq!(body["value"], json!(["a", "b"]));
    }

    #[tokio::test]
    async fn a_non_json_error_body_is_kept_in_the_message() {
        let (url, _) = stub(|_, _, _| (502, Value::String("bad gateway from proxy".into()))).await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        let error = driver
            .start_session("/opt/app")
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("502") && error.contains("bad gateway"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn a_json_error_without_the_webdriver_shape_keeps_its_body() {
        let (url, _) = stub(|_, _, _| (503, json!({ "message": "no healthy upstream" }))).await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        let error = driver
            .start_session("/opt/app")
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("503") && error.contains("no healthy upstream"),
            "{error}"
        );
    }

    #[tokio::test]
    async fn a_failed_delete_keeps_the_session_so_it_can_be_retried() {
        let fail = Arc::new(StdMutex::new(true));
        let flag = fail.clone();
        let (url, _) = stub(move |method, path, _| match (method, path) {
            ("POST", "/session") => ok(json!({ "sessionId": "s7" })),
            ("DELETE", "/session/s7") if *flag.lock().unwrap() => (
                500,
                json!({ "value": { "error": "unknown error", "message": "busy" } }),
            ),
            ("DELETE", "/session/s7") => ok(Value::Null),
            _ => (
                404,
                json!({ "value": { "error": "unknown command", "message": path } }),
            ),
        })
        .await;
        let driver = WebDriver::new(Some(&url)).unwrap();
        driver.start_session("/opt/app").await.unwrap();
        assert!(driver.end_session().await.is_err());
        *fail.lock().unwrap() = false;
        assert!(driver.end_session().await.unwrap());
    }

    #[test]
    fn capabilities_and_unsupported_name_the_tools() {
        let caps = desktop_capabilities();
        assert_eq!(caps["platform"], "desktop");
        assert!(caps["supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "ui_tap"));
        assert!(unsupported("input_swipe").starts_with("input_swipe has no meaning"));
    }
}
