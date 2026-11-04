//! App crash reports. A client app writes a report when it crashes (a stack trace, or a native crash / ANR /
//! low-memory kill read back from the system on the next start) and uploads it here the next time it runs.
//! Admins see the newest ([`CrashConfig::keep`], 50 by default).
//!
//! ```text
//! POST   /api/crash-reports  {kind, device, device_id, app_version, os, time, summary, text, key} → {ok, id, duplicate}
//! GET    /api/crash-reports  (admin) → {crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}
//! DELETE /api/crash-reports  (admin) → {ok}
//! ```
//!
//! The path is [`CrashPaths`] (default `/api/crash-reports`). The reports are kept in the app's [`CrashStore`]
//! ([`CrashHost::crashes`]).

use crate::store::{crash_json, Crash, CrashStore, NewCrash};
use axum::extract::State;
use axum::routing::get;
use axum::{Extension, Json, Router};
use kit_server_core::db::now;
use kit_server_core::{err, Admin, Host, Result, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// Where the crash report route is served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashPaths {
    /// `POST` (upload), `GET` (list, admins), `DELETE` (clear, admins); default `/api/crash-reports`
    pub crashes: String,
}

impl Default for CrashPaths {
    fn default() -> Self {
        CrashPaths { crashes: "/api/crash-reports".into() }
    }
}

/// Where the crash reports are served and how many are kept.
#[derive(Debug, Clone, PartialEq)]
pub struct CrashConfig {
    /// the route's path
    pub paths: CrashPaths,
    /// how many reports are kept, the oldest going first (default 50)
    pub keep: i64,
    /// a report's text is cut to this many bytes (default 256 KiB: an ANR trace holds every thread of the process)
    pub max_text: usize,
}

impl Default for CrashConfig {
    fn default() -> Self {
        CrashConfig { paths: CrashPaths::default(), keep: 50, max_text: 256 * 1024 }
    }
}

/// The configuration the handlers are given.
type Cfg = Extension<Arc<CrashConfig>>;

/// What the crash report routes need from the app besides [`Host`]: where the reports are kept.
pub trait CrashHost: Host {
    /// Where the crash reports are kept: a `SqliteReports` or `PgReports` the app made once at start (the same
    /// one it gives as [`BugReportsHost::reports`](crate::BugReportsHost::reports)), or the app's own
    /// [`CrashStore`].
    fn crashes(&self) -> &dyn CrashStore;
}

/// The crash report route at its default path and limits ([`CrashConfig::default`]).
pub fn router<S: CrashHost>() -> Router<S> {
    router_with(CrashConfig::default())
}

/// The crash report route (POST, GET, DELETE) at `config.paths`.
pub fn router_with<S: CrashHost>(config: CrashConfig) -> Router<S> {
    Router::new().route(&config.paths.crashes, get(list::<S>).post(upload::<S>).delete(clear::<S>)).layer(Extension(Arc::new(config)))
}

/// `POST /api/crash-reports` body.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct CrashBody {
    /// what happened ("crash" when empty)
    pub kind: String,
    /// the device's name
    pub device: String,
    /// the device's id
    pub device_id: String,
    /// the app's version
    pub app_version: String,
    /// the operating system
    pub os: String,
    /// when it happened (epoch seconds); 0 = when it arrived
    pub time: f64,
    /// the exception line ("java.lang.IllegalStateException: …"); empty = the first line of `text`
    pub summary: String,
    /// the stack trace and log
    pub text: String,
    /// the same crash sent twice (an upload whose answer got lost) is kept once: per device and key
    pub key: String,
}

/// `s` cut to at most `max` bytes, on a character boundary.
fn cut(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n… (cut at {} KB)", &s[..end], max / 1024)
}

/// The line shown in the list: the summary, else the first non-empty line of the text.
pub fn first_line(summary: &str, text: &str) -> String {
    let line = if summary.trim().is_empty() { text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("") } else { summary.trim() };
    line.chars().take(300).collect()
}

/// POST /api/crash-reports: keep a crash report from an app.
async fn upload<S: CrashHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    SignedIn(u): SignedIn,
    Json(b): Json<CrashBody>,
) -> Result<Json<Value>> {
    if b.text.trim().is_empty() && b.summary.trim().is_empty() {
        return Err(err(400, "Empty crash report"));
    }
    let kind = if b.kind.trim().is_empty() { "crash".to_string() } else { b.kind.trim().chars().take(40).collect() };
    let small = |s: &str| s.trim().chars().take(200).collect::<String>();
    let received = now();
    let time = if b.time > 0.0 && b.time.is_finite() { b.time } else { received };
    let dedupe = if b.key.trim().is_empty() { None } else { Some(format!("{}|{}|{}", u.id, small(&b.device_id), small(&b.key))) };
    let crash = Crash {
        id: 0,
        user_id: u.id,
        username: u.username.clone(),
        kind,
        device: small(&b.device),
        device_id: small(&b.device_id),
        app_version: small(&b.app_version),
        os: small(&b.os),
        time,
        received_at: received,
        first_line: first_line(&b.summary, &b.text),
        text: cut(&b.text, cfg.max_text),
    };
    let (id, duplicate) = st.crashes().add_crash(&NewCrash { crash, dedupe }, cfg.keep).await?;
    if !duplicate {
        tracing::warn!("app crash report from {} ({}): {}", u.username, small(&b.device), first_line(&b.summary, &b.text));
    }
    Ok(Json(json!({ "ok": true, "id": id, "duplicate": duplicate })))
}

/// GET /api/crash-reports: the crash reports, newest first (admins).
async fn list<S: CrashHost>(State(st): State<S>, Extension(cfg): Cfg, Admin(_): Admin) -> Result<Json<Value>> {
    let crashes: Vec<Value> = st.crashes().crashes(cfg.keep).await?.iter().map(crash_json).collect();
    Ok(Json(json!({ "crashes": crashes })))
}

/// DELETE /api/crash-reports: delete every crash report (admins).
async fn clear<S: CrashHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    st.crashes().clear_crashes().await?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestApp;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    async fn call(app: &Router, method: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
        call_at(app, "/api/crash-reports", method, user, body).await
    }

    async fn call_at(app: &Router, uri: &str, method: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string());
        if body.is_some() {
            b = b.header("content-type", "application/json");
        }
        let req = b.body(body.map(|v| Body::from(v.to_string())).unwrap_or_default()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 16 << 20).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn upload_dedupe_list_cut_and_clear() {
        upload_dedupe_list_cut_and_clear_on(router().with_state(TestApp::sqlite())).await;
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn upload_dedupe_list_cut_and_clear_on_postgres() {
        upload_dedupe_list_cut_and_clear_on(router().with_state(TestApp::postgres().await)).await;
    }

    /// Upload, the same crash twice, an empty one, list, a long text cut, clear: on the app's stores.
    async fn upload_dedupe_list_cut_and_clear_on(app: Router) {
        let report =
            |key: &str, text: &str| json!({ "device": "Pixel 8", "device_id": "dev-1", "text": text, "key": key, "time": 1_700_000_000.0 });
        let r = call(&app, "POST", 2, Some(report("k1", "\n  java.lang.IllegalStateException: boom\n\tat X"))).await;
        assert_eq!(r.0, StatusCode::OK, "{:?}", r.1);
        assert_eq!(r.1["duplicate"], json!(false));
        let again = call(&app, "POST", 2, Some(report("k1", "same"))).await;
        assert_eq!((again.1["duplicate"].clone(), again.1["id"].clone()), (json!(true), r.1["id"].clone()));
        assert_eq!(call(&app, "POST", 2, Some(json!({ "text": "  " }))).await.1["detail"], json!("Empty crash report"));
        assert_eq!(call(&app, "POST", 0, Some(report("k2", "x"))).await.0, StatusCode::UNAUTHORIZED);
        let long = "é".repeat(200_000);
        call(&app, "POST", 2, Some(json!({ "text": long, "time": 1_800_000_000.0 }))).await;

        assert_eq!(call(&app, "GET", 2, None).await.0, StatusCode::FORBIDDEN);
        let (status, v) = call(&app, "GET", 1, None).await;
        assert_eq!(status, StatusCode::OK);
        let list = v["crashes"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0]["text"].as_str().unwrap().ends_with("(cut at 256 KB)"));
        assert_eq!(list[0]["kind"], json!("crash"));
        assert_eq!(list[1]["first_line"], json!("java.lang.IllegalStateException: boom"));
        assert_eq!(list[1]["user"], json!("ana"));

        assert_eq!(call(&app, "DELETE", 2, None).await.0, StatusCode::FORBIDDEN);
        assert_eq!(call(&app, "DELETE", 1, None).await.1, json!({ "ok": true }));
        assert_eq!(call(&app, "GET", 1, None).await.1["crashes"], json!([]));
    }

    #[tokio::test]
    async fn keeps_the_newest_fifty() {
        let app = router().with_state(TestApp::sqlite());
        for i in 0..55 {
            let r = call(
                &app,
                "POST",
                2,
                Some(json!({ "text": format!("Error number {i}"), "key": format!("k{i}"), "time": 1_700_000_000.0 + i as f64 })),
            )
            .await;
            assert_eq!(r.0, StatusCode::OK);
        }
        let v = call(&app, "GET", 1, None).await.1;
        let list = v["crashes"].as_array().unwrap();
        assert_eq!(list.len(), 50);
        assert_eq!(list[0]["first_line"], json!("Error number 54"));
        assert_eq!(list[49]["first_line"], json!("Error number 5"));
    }

    #[tokio::test]
    async fn the_path_and_limits_are_the_apps() {
        let config = CrashConfig { paths: CrashPaths { crashes: "/crashes".into() }, keep: 2, max_text: 1024 };
        let app = router_with(config).with_state(TestApp::sqlite());
        for i in 0..3 {
            let body = json!({ "text": format!("Boom {i}\n{}", "x".repeat(2000)), "time": 1_700_000_000.0 + i as f64 });
            assert_eq!(call_at(&app, "/crashes", "POST", 2, Some(body)).await.0, StatusCode::OK);
        }
        let list = call_at(&app, "/crashes", "GET", 1, None).await.1["crashes"].as_array().unwrap().clone();
        assert_eq!(list.len(), 2);
        assert!(list[0]["text"].as_str().unwrap().ends_with("x\n… (cut at 1 KB)"));
        assert_eq!(list[0]["first_line"], json!("Boom 2"));
        assert_eq!(call(&app, "GET", 1, None).await.0, StatusCode::NOT_FOUND);
    }

    #[test]
    fn first_line_rules() {
        assert_eq!(first_line("", "\n\n  Boom: x \n at y"), "Boom: x");
        assert_eq!(first_line(" ANR ", "trace"), "ANR");
        assert_eq!(first_line("", ""), "");
        assert_eq!(cut("abc", 10), "abc");
    }
}
