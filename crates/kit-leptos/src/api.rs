//! The calls the components make to the server, through one replaceable [`Backend`].
//!
//! The default, [`HttpBackend`], is a `fetch` client for the configured app ([`crate::config`]):
//! `fetch(<api> + path)` with the session cookie and `X-Requested-With: <app id>`, JSON in and out, the server's
//! sentence (`detail`) as the error, "Not connected to the server" when it can't be reached, and the window
//! events `<id>:server-down`, `<id>:server-up` and `<id>:unauthorized` the shell listens for.
//!
//! `<id>:server-down` means the app's server gave no answer of its own: the request failed, or something in front
//! of it (a proxy's page, without the server's `x-<id>` mark) answered. A 502–504 that carries the mark is the
//! server's own answer about something behind it, so it is an error, not a down server. One failure is not
//! enough ([`DownWatch`]): the event goes out after [`DOWN_AFTER_FAILURES`] failed calls in a row, or
//! [`DOWN_AFTER_MS`] after a failure with no call answered since. A cancelled call (aborted, or cut off because
//! the page is being left) counts for nothing. An app with its own
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

/// What a call says when it was cancelled before an answer came (aborted, or the page was being left).
pub const CANCELLED: &str = "The request was cancelled";

/// Failed calls in a row after which the server counts as down.
pub const DOWN_AFTER_FAILURES: u32 = 2;

/// Milliseconds after a failed call, with no call answered since, after which the server counts as down.
pub const DOWN_AFTER_MS: u64 = 5_000;

/// The server the components talk to. Paths are relative to `/api` (`/changelog/new`).
pub trait Backend {
    /// A JSON call: `method` (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`) on `path` with an optional JSON `body`.
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

/// `PATCH /api<path>` with a JSON body.
pub async fn patch(path: &str, body: Value) -> ApiResult {
    backend().call("PATCH", path, Some(body)).await
}

/// `DELETE /api<path>`.
pub async fn del(path: &str) -> ApiResult {
    backend().call("DELETE", path, None).await
}

/// `POST /api<path>` with a multipart form.
pub async fn post_form(path: &str, form: web_sys::FormData) -> ApiResult {
    backend().post_form(path, form).await
}

/// Headers an app adds to a call, worked out per call from its method and path (a CSRF token, `If-Match`).
pub type ExtraHeaders = Rc<dyn Fn(&str, &str) -> Vec<(String, String)>>;

/// The `fetch` client for one app.
#[derive(Clone)]
pub struct HttpBackend {
    /// The app's id in lower case: the `X-Requested-With` header and the events' prefix.
    app: String,
    /// The prefix of every path (`/api`).
    base: String,
    /// The app's own headers for each call (none by default).
    extra: Option<ExtraHeaders>,
}

impl std::fmt::Debug for HttpBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpBackend").field("app", &self.app).field("base", &self.base).field("extra", &self.extra.is_some()).finish()
    }
}

impl HttpBackend {
    /// The client for the app with id `app` (`kit-demo`: `X-Requested-With: kit-demo`, `kit-demo:server-down`,
    /// …), under `/api`.
    pub fn new(app: &str) -> Self {
        HttpBackend { app: app.to_ascii_lowercase(), base: "/api".into(), extra: None }
    }

    /// The client for the configured app ([`crate::config::get`]): its id, under its [`api`](crate::config::Paths::api)
    /// prefix.
    pub fn from_config() -> Self {
        let c = crate::config::get();
        HttpBackend { app: c.app.id.to_ascii_lowercase(), base: c.paths.api.clone(), extra: None }
    }

    /// The same client under another prefix (`/v1`).
    pub fn with_base(mut self, base: &str) -> Self {
        self.base = base.to_string();
        self
    }

    /// The same client with the app's own headers on every call: `headers(method, path)` gives them (for
    /// example a CSRF token on everything but `GET`, or `If-Match` on a save). Called on every call, so a
    /// header can change (a token after signing in).
    pub fn with_headers(mut self, headers: impl Fn(&str, &str) -> Vec<(String, String)> + 'static) -> Self {
        self.extra = Some(Rc::new(headers));
        self
    }

    /// The app's own headers for this call.
    fn extra_headers(&self, method: &str, path: &str) -> Vec<(String, String)> {
        self.extra.as_ref().map(|f| f(method, path)).unwrap_or_default()
    }

    fn event(&self, what: &str) {
        crate::rt::dispatch(&format!("{}:{what}", self.app));
    }

    /// The server answered: `<id>:server-up`, and the run of failures ends.
    fn up(&self) {
        WATCH.with(|w| w.borrow_mut().entry(self.app.clone()).or_default().success());
        self.event("server-up");
    }

    /// The server did not answer: `<id>:server-down` once [`DownWatch`] says so, else a check
    /// [`DOWN_AFTER_MS`] later in case nothing is answered by then.
    fn down(&self) {
        let now = crate::rt::now_ms();
        let (is_down, since) = WATCH.with(|w| {
            let mut w = w.borrow_mut();
            let d = w.entry(self.app.clone()).or_default();
            (d.failure(now), d.since())
        });
        if is_down {
            self.event("server-down");
            return;
        }
        let me = self.clone();
        crate::rt::after(DOWN_AFTER_MS, move || {
            let still = WATCH.with(|w| w.borrow().get(&me.app).is_some_and(|d| d.since() == since && d.overdue(crate::rt::now_ms())));
            if still {
                me.event("server-down");
            }
        });
    }

    /// A call that failed before an answer came: cancelled ([`CANCELLED`], no event) or not connected.
    fn failed(&self, e: &gloo_net::Error) -> ApiError {
        let name = match e {
            gloo_net::Error::JsError(j) => j.name.as_str(),
            _ => "",
        };
        if cancelled(name, leaving()) {
            return ApiError::new(CANCELLED);
        }
        self.down();
        ApiError::new(NOT_CONNECTED)
    }
}

/// Whether an answer came from something in front of the server, not the server: a kit server marks its answers
/// with `x-<app id>` and sends its errors as JSON objects, a proxy in front of a server that is down answers with
/// its own page.
pub fn proxy_answer(status: u16, marked: bool, body: &str) -> bool {
    (status == 404 || status >= 500) && !marked && !serde_json::from_str::<Value>(body).is_ok_and(|v| v.is_object())
}

/// Whether an answer means the app's server did not answer: something in front of it did ([`proxy_answer`]),
/// or it is a 502–504 without the server's `x-<app id>` mark. A marked 502–504 is the server's own answer about
/// something behind it, so the server is up.
pub fn server_down(status: u16, marked: bool, body: &str) -> bool {
    proxy_answer(status, marked, body) || ((502..=504).contains(&status) && !marked)
}

/// Whether a call that failed was cancelled rather than unanswered: the browser's error is an `AbortError`, or
/// the page was being left when it failed.
pub fn cancelled(error_name: &str, leaving: bool) -> bool {
    leaving || error_name == "AbortError"
}

/// Keeps one failure from raising the "server isn't answering" banner: the server counts as down after
/// [`DOWN_AFTER_FAILURES`] failed calls in a row, or [`DOWN_AFTER_MS`] after the first failure of a run with no
/// call answered since. Any answered call ends the run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DownWatch {
    /// Failed calls since the last answered one.
    failures: u32,
    /// When the first of them failed (milliseconds, [`crate::rt::now_ms`]).
    since: Option<u64>,
}

impl DownWatch {
    /// A call was answered: the run of failures ends.
    pub fn success(&mut self) {
        *self = DownWatch::default();
    }

    /// A call failed at `now`; answers whether the server now counts as down.
    pub fn failure(&mut self, now: u64) -> bool {
        self.failures += 1;
        self.since.get_or_insert(now);
        self.overdue(now)
    }

    /// Whether the server counts as down at `now` (no new failure needed: the timer after a first failure asks).
    pub fn overdue(&self, now: u64) -> bool {
        let Some(since) = self.since else { return false };
        self.failures >= DOWN_AFTER_FAILURES || now.saturating_sub(since) >= DOWN_AFTER_MS
    }

    /// When the current run of failures started, if one runs.
    pub fn since(&self) -> Option<u64> {
        self.since
    }
}

thread_local! {
    /// Each app's run of failures, by app id (a backend is made per call, so the run can't live in it).
    static WATCH: RefCell<std::collections::HashMap<String, DownWatch>> = RefCell::new(Default::default());
    /// The page is being left (`pagehide` until `pageshow`): calls cut off now were cancelled.
    static LEAVING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The `pagehide`/`pageshow` listeners are in place.
    static PAGE_WATCHED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn watch_page() {
    if PAGE_WATCHED.with(|w| w.replace(true)) {
        return;
    }
    crate::rt::on_window("pagehide", |_| LEAVING.with(|l| l.set(true)));
    crate::rt::on_window("pageshow", |_| LEAVING.with(|l| l.set(false)));
}

fn leaving() -> bool {
    LEAVING.with(std::cell::Cell::get)
}

/// The sentence of a failed answer: the server's `detail` (a string, or the `msg` of each entry), else its
/// `message`, else its `error` (when they are text), else the status text, else "Request failed".
pub fn error_message(data: &Value, status_text: &str) -> String {
    let text = |k: &str| data.get(k).and_then(Value::as_str).filter(|s| !s.is_empty()).map(String::from);
    let msg = match data.get("detail") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => {
            a.iter().map(|m| m.get("msg").and_then(Value::as_str).unwrap_or("").to_string()).collect::<Vec<_>>().join(", ")
        }
        _ => text("message").or_else(|| text("error")).unwrap_or_default(),
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
        let text = match res.text().await {
            Ok(t) => t,
            Err(e) => return Err(self.failed(&e)),
        };
        let marked = res.headers().get(&format!("x-{}", self.app)).is_some();
        if server_down(status, marked, &text) {
            self.down();
        } else {
            self.up();
        }
        if proxy_answer(status, marked, &text) {
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
            for (k, v) in me.extra_headers(&method, &path) {
                req = req.header(&k, &v);
            }
            if body.is_some() {
                req = req.header("Content-Type", "application/json");
            }
            let built = match body {
                Some(b) => req.body(b.to_string()),
                None => req.build(),
            };
            let Ok(r) = built else { return Err(ApiError::new("Bad request")) };
            watch_page();
            match r.send().await {
                Ok(res) => me.answer(res, path).await,
                Err(e) => Err(me.failed(&e)),
            }
        })
    }

    fn post_form(&self, path: &str, form: web_sys::FormData) -> ApiFuture {
        let (me, path) = (self.clone(), path.to_string());
        Box::pin(async move {
            let mut req = gloo_net::http::RequestBuilder::new(&format!("{}{path}", me.base))
                .method(gloo_net::http::Method::POST)
                .credentials(web_sys::RequestCredentials::SameOrigin)
                .header("X-Requested-With", &me.app);
            for (k, v) in me.extra_headers("POST", &path) {
                req = req.header(&k, &v);
            }
            let built = req.body(form);
            let Ok(r) = built else { return Err(ApiError::new("Bad request")) };
            watch_page();
            match r.send().await {
                Ok(res) => me.answer(res, path).await,
                Err(e) => Err(me.failed(&e)),
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
        assert_eq!(error_message(&json!({ "error": "forbidden", "message": "Sign in first" }), "Forbidden"), "Sign in first");
        assert_eq!(
            error_message(&json!({ "error": "the dashboard was changed elsewhere" }), "Conflict"),
            "the dashboard was changed elsewhere"
        );
        assert_eq!(error_message(&json!({ "error": 3 }), "Conflict"), "Conflict");
        assert_eq!(error_message(&Value::Null, ""), "Request failed");
        assert!(proxy_answer(502, false, "<html>"));
        assert!(!proxy_answer(502, true, "<html>"));
        assert!(!proxy_answer(500, false, r#"{"detail":"x"}"#));
        assert!(!proxy_answer(400, false, "<html>"));
    }

    #[test]
    fn a_marked_502_to_504_is_the_servers_own_error_not_a_down_server() {
        for code in 502..=504 {
            // the app's own header: the server answered about something behind it
            assert!(!server_down(code, true, r#"{"detail":"the search index is down"}"#), "{code}");
            assert!(!server_down(code, true, "<html>Bad gateway</html>"), "{code}");
            // no header: a proxy's page, or a proxy passing on a bare error
            assert!(server_down(code, false, "<html>Bad gateway</html>"), "{code}");
            assert!(server_down(code, false, r#"{"message":"upstream"}"#), "{code}");
            assert!(server_down(code, false, ""), "{code}");
        }
        assert!(!server_down(500, true, r#"{"detail":"x"}"#));
        assert!(!server_down(500, false, r#"{"detail":"x"}"#));
        assert!(server_down(404, false, "<html>Not found</html>"));
        assert!(!server_down(404, true, "<html>Not found</html>"));
        assert!(!server_down(200, false, "{}"));
        assert!(!server_down(401, false, ""));
    }

    #[test]
    fn one_failure_is_not_a_down_server() {
        let mut w = DownWatch::default();
        assert!(!w.overdue(0));
        assert!(!w.failure(1_000));
        assert!(!w.overdue(1_000 + DOWN_AFTER_MS - 1));
        assert_eq!(w.since(), Some(1_000));
    }

    #[test]
    fn two_failures_in_a_row_are_a_down_server() {
        let mut w = DownWatch::default();
        assert!(!w.failure(1_000));
        assert!(w.failure(1_100));
        assert!(w.failure(1_200), "it stays down while calls keep failing");
    }

    #[test]
    fn five_seconds_without_an_answer_after_a_failure_are_a_down_server() {
        let mut w = DownWatch::default();
        assert!(!w.failure(1_000));
        // the timer after the first failure asks
        assert!(w.overdue(1_000 + DOWN_AFTER_MS));
        // or the next failure, however long after
        let mut w = DownWatch::default();
        w.failure(0);
        w.success();
        assert!(!w.failure(10_000), "a run starts at its own first failure, not at the last answer");
    }

    #[test]
    fn an_answer_in_between_ends_the_run() {
        let mut w = DownWatch::default();
        assert!(!w.failure(1_000));
        w.success();
        assert!(!w.failure(1_100), "failures are counted in a row");
        assert_eq!(w.since(), Some(1_100));
        w.success();
        assert_eq!(w, DownWatch::default());
        assert!(!w.overdue(1_000 + DOWN_AFTER_MS), "the timer of an ended run finds nothing");
    }

    #[test]
    fn cancelled_calls_count_for_nothing() {
        assert!(cancelled("AbortError", false));
        assert!(cancelled("TypeError", true), "cut off while the page is being left");
        assert!(!cancelled("TypeError", false), "a failed fetch is a real failure");
        assert!(!cancelled("", false));
    }

    #[test]
    fn the_default_client_is_the_configured_app() {
        let b = HttpBackend::from_config();
        assert_eq!((b.app.as_str(), b.base.as_str()), ("app", "/api"));
        let b = HttpBackend::new("Kit-Demo").with_base("/v1");
        assert_eq!((b.app.as_str(), b.base.as_str()), ("kit-demo", "/v1"));
        assert!(b.extra_headers("POST", "/x").is_empty());
        let b = b.with_headers(|m, p| if m == "GET" { vec![] } else { vec![("X-CSRF-Token".into(), format!("t{p}"))] });
        assert!(b.extra_headers("GET", "/x").is_empty());
        assert_eq!(b.extra_headers("PUT", "/x"), vec![("X-CSRF-Token".to_string(), "t/x".to_string())]);
    }
}
