//! The notice object, kept notices, the timer and the routes (at the kit's paths and at an app's own).

use super::*;
use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use kit_server_core::testing::TestHost;
use kit_server_core::{Account, Db};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

/// The test app: a disk notice the app keeps itself and a kept probe.
#[derive(Clone)]
struct App {
    host: TestHost,
    /// the disk is broken
    broken: Arc<AtomicBool>,
    /// the probe's runs
    probes: Arc<AtomicUsize>,
    /// the probe waits this long
    slow: Arc<AtomicUsize>,
    notices: Notices<App>,
}

impl Host for App {
    fn db(&self) -> &dyn Db {
        self.host.db()
    }
    async fn signed_in(&self, parts: &mut Parts) -> kit_server_core::Result<Account> {
        self.host.signed_in(parts).await
    }
}

impl NoticesHost for App {
    fn notices(&self) -> Notices<App> {
        self.notices.clone()
    }
}

/// The disk notice as the app makes it: the whole notice for admins, only `ok` for members.
fn disk(app: &App, admin: bool) -> Notice {
    let broken = app.broken.load(Ordering::SeqCst);
    if !admin {
        return Notice::default().with("ok", json!(!broken));
    }
    let text = broken.then_some("The data folder /data is read-only – check the disk or the mount");
    Notice::from(json!({ "ok": !broken, "text": text, "checking": false, "waiting": 0 }))
}

fn app() -> App {
    let notices =
        Notices::new()
            .register(NoticeCheck::new("disk", Severity::Warn, disk, |app: App, admin| async move { disk(&app, admin) }).toasts(
                Toasts::new("The data folder still can't be written", "Still checking the data folder", "The data folder can be written"),
            ))
            .register(
                NoticeCheck::kept("sync", Severity::Info, |app: App| async move {
                    app.probes.fetch_add(1, Ordering::SeqCst);
                    let ms = app.slow.load(Ordering::SeqCst) as u64;
                    tokio::time::sleep(Duration::from_millis(ms)).await;
                    Notice::failing("The last sync failed – check the sync address")
                })
                .every(Duration::from_secs(60)),
            );
    App { host: TestHost::new(), broken: Arc::default(), probes: Arc::default(), slow: Arc::default(), notices }
}

async fn call(r: Router<App>, app: &App, method: &str, uri: &str, user: i64) -> (StatusCode, Value) {
    let req = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string()).body(Body::empty()).unwrap();
    let resp = r.with_state(app.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[test]
fn the_notice_object() {
    let n = Notice::failing("The disk is full").checking(true);
    assert_eq!((n.ok(), n.text(), n.is_checking(), n.shows()), (false, Some("The disk is full"), true, true));
    assert_eq!(n.plain().into_json(), json!({ "ok": false }));
    assert!(!Notice::fine().shows() && Notice::fine().ok());
    // no banner without a sentence; a notice without ok is fine
    assert!(!Notice::from(json!({ "ok": false, "text": "" })).shows());
    assert!(!Notice::from(json!({ "ok": false })).shows());
    assert!(Notice::from(json!({})).ok() && Notice::from(json!(null)).ok());
    // an app's own object comes back as it is
    let own = json!({ "ok": true, "text": null, "checked_at": 1.5, "roots": [] });
    assert_eq!(Value::from(Notice::from(own.clone())), own);
    assert_eq!(Notice::from(own).get("checked_at"), Some(&json!(1.5)));
    assert_eq!(serde_json::to_value(Severity::Err).unwrap(), json!("err"));
}

#[test]
fn check_agains_toast() {
    let t = Toasts::new("The data folder still can't be written", "Still checking the data folder", "The data folder can be written");
    let toast = |v: Value| {
        let n = Notice::from(v);
        let t = t.after(&n);
        (t.text.to_string(), t.kind)
    };
    assert_eq!(toast(json!({ "ok": false, "text": "x" })), ("The data folder still can't be written".into(), "warn"));
    assert_eq!(toast(json!({ "ok": true, "checking": true })), ("Still checking the data folder".into(), "warn"));
    assert_eq!(toast(json!({ "ok": true })), ("The data folder can be written".into(), "ok"));
}

#[tokio::test]
async fn the_default_routes() {
    let a = app();
    a.broken.store(true, Ordering::SeqCst);
    let (s, v) = call(router(), &a, "GET", "/api/notices", 1).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        v,
        json!({ "notices": [
            { "id": "disk", "severity": "warn", "ok": false, "checking": false, "waiting": 0,
              "text": "The data folder /data is read-only – check the disk or the mount" },
            // nothing probed yet: fine
            { "id": "sync", "severity": "info", "ok": true, "text": null, "checking": false },
        ] })
    );
    // a member: whether each is fine, nothing more
    let (_, v) = call(router(), &a, "GET", "/api/notices", 2).await;
    assert_eq!(
        v,
        json!({ "notices": [{ "id": "disk", "severity": "warn", "ok": false }, { "id": "sync", "severity": "info", "ok": true }] })
    );
    assert_eq!(call(router(), &a, "GET", "/api/notices", 0).await.0, StatusCode::UNAUTHORIZED);
    // Check again: admins only; every notice probed
    assert_eq!(call(router(), &a, "POST", "/api/notices/check", 2).await, (StatusCode::FORBIDDEN, json!({ "detail": "Admin only" })));
    a.broken.store(false, Ordering::SeqCst);
    let (s, v) = call(router(), &a, "POST", "/api/notices/check", 1).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["notices"][0]["ok"], json!(true));
    assert_eq!(
        v["notices"][1],
        json!({ "id": "sync", "severity": "info", "ok": false, "text": "The last sync failed – check the sync address", "checking": false })
    );
    assert_eq!(a.probes.load(Ordering::SeqCst), 1);
    // kept: read again without probing
    let (_, v) = call(router(), &a, "GET", "/api/notices", 1).await;
    assert_eq!(v["notices"][1]["ok"], json!(false));
    assert_eq!(a.probes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_app_can_move_the_routes() {
    let a = app();
    let r = || router_with::<App>(NoticePaths { list: "/admin/banners".into(), check: "/admin/banners/recheck".into() });
    a.broken.store(true, Ordering::SeqCst);
    let (s, v) = call(r(), &a, "GET", "/admin/banners", 1).await;
    assert_eq!((s, v["notices"][0]["ok"].clone()), (StatusCode::OK, json!(false)));
    assert_eq!(call(r(), &a, "POST", "/admin/banners/recheck", 2).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(r(), &a, "GET", "/api/notices", 1).await.0, StatusCode::NOT_FOUND, "the default path is not served");
    assert_eq!(NoticePaths::default(), NoticePaths { list: "/api/notices".into(), check: "/api/notices/check".into() });
}

#[tokio::test]
async fn one_notice_inside_the_apps_own_answer() {
    let a = app();
    a.broken.store(true, Ordering::SeqCst);
    // the app's own object, without id or severity
    let n = a.notices().current(&a, "disk", true).await.unwrap().into_json();
    assert_eq!(n["ok"], json!(false));
    assert!(n.get("id").is_none() && n.get("severity").is_none(), "{n}");
    assert_eq!(a.notices().current(&a, "disk", false).await.unwrap().into_json(), json!({ "ok": false }));
    a.broken.store(false, Ordering::SeqCst);
    let n = a.notices().check(&a, "disk", true).await.unwrap();
    assert_eq!(n.into_json(), json!({ "ok": true, "text": null, "checking": false, "waiting": 0 }));
    // an id the app does not have
    assert!(a.notices().check(&a, "nope", true).await.is_none());
    assert!(a.notices().current(&a, "nope", true).await.is_none());
    assert_eq!(a.notices().get("disk").unwrap().toast_texts().unwrap().fixed, "The data folder can be written");
    assert_eq!(a.notices().get("disk").unwrap().severity(), Severity::Warn);
}

/// A probe that does not end: Check again answers after [`CHECK_WAIT`] with `checking`, and a second Check
/// again joins the probe that runs instead of starting another.
#[tokio::test(start_paused = true)]
async fn a_probe_that_does_not_end_is_not_waited_for() {
    let a = app();
    a.slow.store(60_000, Ordering::SeqCst);
    let kept = a.notices().get("sync").unwrap().clone();
    let t0 = tokio::time::Instant::now();
    let n = kept.check(&a, true).await;
    assert_eq!(t0.elapsed(), CHECK_WAIT);
    assert_eq!(n.into_json(), json!({ "ok": true, "text": null, "checking": true }));
    let n = kept.check(&a, false).await;
    assert_eq!(n.into_json(), json!({ "ok": true }));
    assert_eq!(a.probes.load(Ordering::SeqCst), 1, "one probe at a time");
    tokio::time::sleep(Duration::from_secs(60)).await;
    let n = kept.current(&a, true).await;
    assert_eq!((n.ok(), n.is_checking()), (false, false));
}

#[tokio::test(start_paused = true)]
async fn notices_with_an_interval_are_checked_on_a_timer() {
    let a = app();
    let handles = spawn_rechecks(a.clone());
    assert_eq!(handles.len(), 1, "only the notice with an interval");
    assert_eq!(a.notices().get("sync").unwrap().interval(), Some(Duration::from_secs(60)));
    assert_eq!(a.notices().get("disk").unwrap().interval(), None);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(a.probes.load(Ordering::SeqCst), 1, "the first check at once");
    tokio::time::sleep(Duration::from_secs(120)).await;
    assert_eq!(a.probes.load(Ordering::SeqCst), 3);
    for h in handles {
        h.abort();
    }
}
