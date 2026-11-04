//! App crash reports (Encore's Diagnostics → "App crashes", from `routes/client_crash.rs`). The Android app
//! writes a report when it crashes (Java/Kotlin stack, or a native crash / ANR / low-memory kill read back from
//! the system on the next start) and uploads it here the next time it runs. Admins see the newest 50.
//!
//! ```text
//! POST   /api/diagnostics/client-crash  {kind, device, device_id, app_version, os, time, summary, text, key} → {ok, id, duplicate}
//! GET    /api/diagnostics/client-crash  (admin) → {crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}
//! DELETE /api/diagnostics/client-crash  (admin) → {ok}
//! ```

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use kit_server_core::db::now;
use kit_server_core::{err, p, Admin, Host, Result, RowExt, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};

/// How many reports are kept (the oldest go first).
pub const KEEP: i64 = 50;
/// A report's text is cut to this many bytes (an ANR trace holds every thread of the process).
pub const MAX_TEXT: usize = 256 * 1024;

/// The crash report routes: `/api/diagnostics/client-crash` (POST, GET, DELETE).
pub fn router<S: Host>() -> Router<S> {
    Router::new().route("/api/diagnostics/client-crash", get(list::<S>).post(upload::<S>).delete(clear::<S>))
}

/// Create the client_crashes table when it isn't there (Encore's table, unchanged).
fn ensure_table(st: &impl Host) -> Result<()> {
    // created on first use: an older server simply never reads it
    st.db().execute_batch(
        "CREATE TABLE IF NOT EXISTS client_crashes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            username TEXT,
            kind TEXT,
            device TEXT,
            device_id TEXT,
            app_version TEXT,
            os TEXT,
            time REAL,
            received_at REAL,
            first_line TEXT,
            text TEXT,
            dedupe TEXT
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_client_crashes_dedupe ON client_crashes(dedupe);",
    )
}

/// `POST /api/diagnostics/client-crash` body.
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

/// POST /api/diagnostics/client-crash: keep a crash report from an app.
pub async fn upload<S: Host>(State(st): State<S>, SignedIn(u): SignedIn, Json(b): Json<CrashBody>) -> Result<Json<Value>> {
    if b.text.trim().is_empty() && b.summary.trim().is_empty() {
        return Err(err(400, "Empty crash report"));
    }
    ensure_table(&st)?;
    let kind = if b.kind.trim().is_empty() { "crash".to_string() } else { b.kind.trim().chars().take(40).collect() };
    let small = |s: &str| s.trim().chars().take(200).collect::<String>();
    let received = now();
    let time = if b.time > 0.0 && b.time.is_finite() { b.time } else { received };
    let dedupe = if b.key.trim().is_empty() { None } else { Some(format!("{}|{}|{}", u.id, small(&b.device_id), small(&b.key))) };
    if let Some(d) = &dedupe {
        if let Some(r) = st.db().one("SELECT id FROM client_crashes WHERE dedupe=?", p![d.clone()])? {
            return Ok(Json(json!({ "ok": true, "id": r.i("id"), "duplicate": true })));
        }
    }
    let id = st.db().execute(
        "INSERT INTO client_crashes (user_id, username, kind, device, device_id, app_version, os, time, received_at, first_line, text, dedupe) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
        p![
            u.id,
            u.username,
            kind,
            small(&b.device),
            small(&b.device_id),
            small(&b.app_version),
            small(&b.os),
            time,
            received,
            first_line(&b.summary, &b.text),
            cut(&b.text, MAX_TEXT),
            dedupe
        ],
    )?;
    st.db().execute("DELETE FROM client_crashes WHERE id NOT IN (SELECT id FROM client_crashes ORDER BY id DESC LIMIT ?)", p![KEEP])?;
    tracing::warn!("app crash report from {} ({}): {}", u.username, small(&b.device), first_line(&b.summary, &b.text));
    Ok(Json(json!({ "ok": true, "id": id, "duplicate": false })))
}

/// GET /api/diagnostics/client-crash: the crash reports, newest first (admins).
pub async fn list<S: Host>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let rows = st.db().query(
        "SELECT id, kind, username user, device, device_id, app_version, os, time, received_at, first_line, text FROM client_crashes ORDER BY time DESC, id DESC LIMIT ?",
        p![KEEP],
    )?;
    Ok(Json(json!({ "crashes": rows })))
}

/// DELETE /api/diagnostics/client-crash: delete every crash report (admins).
pub async fn clear<S: Host>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    ensure_table(&st)?;
    st.db().execute("DELETE FROM client_crashes", p![])?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use kit_server_core::testing::TestHost;
    use tower::ServiceExt;

    async fn call(app: &Router, method: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
        let mut b = Request::builder().method(method).uri("/api/diagnostics/client-crash").header("x-test-user", user.to_string());
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
        let app = router().with_state(TestHost::new());
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
        let app = router().with_state(TestHost::new());
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
        assert_eq!(list.len(), KEEP as usize);
        assert_eq!(list[0]["first_line"], json!("Error number 54"));
        assert_eq!(list[49]["first_line"], json!("Error number 5"));
    }

    #[test]
    fn first_line_rules() {
        assert_eq!(first_line("", "\n\n  Boom: x \n at y"), "Boom: x");
        assert_eq!(first_line(" ANR ", "trace"), "ANR");
        assert_eq!(first_line("", ""), "");
        assert_eq!(cut("abc", 10), "abc");
    }
}
