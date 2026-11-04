//! The calls the components make to the server, through one replaceable [`Backend`].
//!
//! The default, [`HttpBackend`], is a `fetch` client for the configured app ([`crate::config`]):
//! `fetch(<api> + path)` with the session cookie and `X-Requested-With: <app id>`, JSON in and out, the server's
//! sentence (`detail`) as the error, "Not connected to the server" when it can't be reached, and the window
//! events `<id>:server-down`, `<id>:server-up` and `<id>:unauthorized` the shell listens for. An app with its own
//! API client passes it in with [`set_backend`]; the demo passes a fake one, so no server is needed.

use serde_json::Value;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// What a call answers: the JSON, or an [`ApiError`].
pub type ApiResult = Result<Value, ApiError>;
/// A call under way.
pub type ApiFuture = Pin<Box<dyn Future<Output = ApiResult>>>;

/// A failed call: the HTTP status (0 when no answer came), the sentence to show and the whole answer.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiError {
    /// The HTTP status, 0 when the server could not be reached.
    pub status: u16,
    /// The server's sentence (`detail`), or what went wrong in plain words.
    pub message: String,
    /// The whole answer.
    pub data: Value,
}

impl ApiError {
    /// An error with this sentence and no answer (status 0).
    pub fn new(message: impl Into<String>) -> Self {
        ApiError { status: 0, message: message.into(), data: Value::Null }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiError {}

/// What a failed call says when the server could not be reached.
pub const NOT_CONNECTED: &str = "Not connected to the server";

/// The server the components talk to. Paths are relative to `/api` (`/changelog/new`).
pub trait Backend {
    /// A JSON call: `method` (`GET`, `POST`, `PUT`, `DELETE`) on `path` with an optional JSON `body`.
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> ApiFuture;
    /// A multipart form (a file upload) posted to `path`.
    fn post_form(&self, path: &str, form: web_sys::FormData) -> ApiFuture;
}

thread_local! {
    static BACKEND: RefCell<Option<Rc<dyn Backend>>> = const { RefCell::new(None) };
}

/// Uses `b` for every call from here on (the app's own client, or a fake one in a demo). Until one is set, the
/// calls go through [`HttpBackend::from_config`].
pub fn set_backend(b: impl Backend + 'static) {
    BACKEND.with(|x| *x.borrow_mut() = Some(Rc::new(b)));
}

fn backend() -> Rc<dyn Backend> {
    BACKEND.with(|b| b.borrow().clone()).unwrap_or_else(|| Rc::new(HttpBackend::from_config()))
}

/// `GET /api<path>`.
pub async fn get(path: &str) -> ApiResult {
    backend().call("GET", path, None).await
}

/// `POST /api<path>` with a JSON body.
pub async fn post(path: &str, body: Value) -> ApiResult {
    backend().call("POST", path, Some(body)).await
}

/// `POST /api<path>` with the empty body `{}`.
pub async fn post_empty(path: &str) -> ApiResult {
    backend().call("POST", path, Some(serde_json::json!({}))).await
}

/// `PUT /api<path>` with a JSON body.
pub async fn put(path: &str, body: Value) -> ApiResult {
    backend().call("PUT", path, Some(body)).await
}

/// `DELETE /api<path>`.
pub async fn del(path: &str) -> ApiResult {
    backend().call("DELETE", path, None).await
}

/// `POST /api<path>` with a multipart form.
pub async fn post_form(path: &str, form: web_sys::FormData) -> ApiResult {
    backend().post_form(path, form).await
}

/// The `fetch` client for one app.
#[derive(Clone, Debug)]
pub struct HttpBackend {
    /// The app's id in lower case: the `X-Requested-With` header and the events' prefix.
    app: String,
    /// The prefix of every path (`/api`).
    base: String,
}

impl HttpBackend {
    /// The client for the app with id `app` (`kit-demo`: `X-Requested-With: kit-demo`, `kit-demo:server-down`,
    /// …), under `/api`.
    pub fn new(app: &str) -> Self {
        HttpBackend { app: app.to_ascii_lowercase(), base: "/api".into() }
    }

    /// The client for the configured app ([`crate::config::get`]): its id, under its [`api`](crate::config::Paths::api)
    /// prefix.
    pub fn from_config() -> Self {
        let c = crate::config::get();
        HttpBackend { app: c.app.id.to_ascii_lowercase(), base: c.paths.api.clone() }
    }

    /// The same client under another prefix (`/v1`).
    pub fn with_base(mut self, base: &str) -> Self {
        self.base = base.to_string();
        self
    }

    fn event(&self, what: &str) {
        crate::rt::dispatch(&format!("{}:{what}", self.app));
    }
}

/// Whether an answer came from something in front of the server, not the server: a kit server marks its answers
/// with `x-<app id>` and sends its errors as JSON objects, a proxy in front of a server that is down answers with
/// its own page.
pub fn proxy_answer(status: u16, marked: bool, body: &str) -> bool {
    (status == 404 || status >= 500) && !marked && !serde_json::from_str::<Value>(body).is_ok_and(|v| v.is_object())
}

/// The sentence of a failed answer: the server's `detail` (a string, or the `msg` of each entry), else the
/// status text, else "Request failed".
pub fn error_message(data: &Value, status_text: &str) -> String {
    let msg = match data.get("detail") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => {
            a.iter().map(|m| m.get("msg").and_then(Value::as_str).unwrap_or("").to_string()).collect::<Vec<_>>().join(", ")
        }
        _ => String::new(),
    };
    if !msg.is_empty() {
        msg
    } else if !status_text.is_empty() {
        status_text.to_string()
    } else {
        "Request failed".to_string()
    }
}

impl HttpBackend {
    async fn answer(self, res: gloo_net::http::Response, path: String) -> ApiResult {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        let proxy = proxy_answer(status, res.headers().get(&format!("x-{}", self.app)).is_some(), &text);
        self.event(if proxy || (502..=504).contains(&status) { "server-down" } else { "server-up" });
        if proxy {
            return Err(ApiError::new(NOT_CONNECTED));
        }
        let data: Value = if text.is_empty() { Value::Null } else { serde_json::from_str(&text).unwrap_or(Value::String(text.clone())) };
        if !res.ok() {
            if status == 401 && !path.starts_with("/auth/") {
                self.event("unauthorized");
            }
            return Err(ApiError { status, message: error_message(&data, &res.status_text()), data });
        }
        Ok(data)
    }
}

impl Backend for HttpBackend {
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> ApiFuture {
        let (me, method, path) = (self.clone(), method.to_string(), path.to_string());
        Box::pin(async move {
            use gloo_net::http::{Method, RequestBuilder};
            let m = match method.as_str() {
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH,
                _ => Method::GET,
            };
            let mut req = RequestBuilder::new(&format!("{}{path}", me.base))
                .method(m)
                .credentials(web_sys::RequestCredentials::SameOrigin)
                .header("X-Requested-With", &me.app);
            if body.is_some() {
                req = req.header("Content-Type", "application/json");
            }
            let built = match body {
                Some(b) => req.body(b.to_string()),
                None => req.build(),
            };
            let Ok(r) = built else { return Err(ApiError::new("Bad request")) };
            match r.send().await {
                Ok(res) => me.answer(res, path).await,
                Err(_) => {
                    me.event("server-down");
                    Err(ApiError::new(NOT_CONNECTED))
                }
            }
        })
    }

    fn post_form(&self, path: &str, form: web_sys::FormData) -> ApiFuture {
        let (me, path) = (self.clone(), path.to_string());
        Box::pin(async move {
            let built = gloo_net::http::RequestBuilder::new(&format!("{}{path}", me.base))
                .method(gloo_net::http::Method::POST)
                .credentials(web_sys::RequestCredentials::SameOrigin)
                .header("X-Requested-With", &me.app)
                .body(form);
            let Ok(r) = built else { return Err(ApiError::new("Bad request")) };
            match r.send().await {
                Ok(res) => me.answer(res, path).await,
                Err(_) => {
                    me.event("server-down");
                    Err(ApiError::new(NOT_CONNECTED))
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn errors_read_as_the_servers_sentence() {
        assert_eq!(error_message(&json!({ "detail": "No such report" }), "Not Found"), "No such report");
        assert_eq!(error_message(&json!({ "detail": [{ "msg": "a" }, { "msg": "b" }] }), ""), "a, b");
        assert_eq!(error_message(&Value::Null, "Bad Gateway"), "Bad Gateway");
        assert_eq!(error_message(&Value::Null, ""), "Request failed");
        assert!(proxy_answer(502, false, "<html>"));
        assert!(!proxy_answer(502, true, "<html>"));
        assert!(!proxy_answer(500, false, r#"{"detail":"x"}"#));
        assert!(!proxy_answer(400, false, "<html>"));
    }

    #[test]
    fn the_default_client_is_the_configured_app() {
        let b = HttpBackend::from_config();
        assert_eq!((b.app.as_str(), b.base.as_str()), ("app", "/api"));
        let b = HttpBackend::new("Kit-Demo").with_base("/v1");
        assert_eq!((b.app.as_str(), b.base.as_str()), ("kit-demo", "/v1"));
    }
}
