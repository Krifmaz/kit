//! The check object, the registry and both routes through a router on the kit's test app.

use super::*;
use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use kit_server_core::testing::TestHost;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceExt;

/// The test app: the kit's test host, with sections of its own in the answer when `sections` is on.
#[derive(Clone)]
struct App {
    host: TestHost,
    sections: bool,
    runs: Arc<AtomicUsize>,
}

impl Host for App {
    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        self.host.signed_in(parts).await
    }
}

impl Diagnostics for App {
    fn diagnostics(&self) -> Registry<App> {
        Registry::new()
            .group("services", |app: App| async move {
                app.runs.fetch_add(1, Ordering::SeqCst);
                // the slow one first: the order stays the order of the groups, not of who finishes first
                tokio::time::sleep(Duration::from_millis(30)).await;
                Ok(vec![Check::new("Mail server", Kind::Service, true, "OK")
                    .request("GET https://mail.example.com/health")
                    .with("status", json!(200))
                    .with("ms", json!(180))])
            })
            .group("not set up", |_app: App| async move { Ok(vec![]) })
            .group("folders", |_app: App| async move {
                Ok(vec![
                    Check::new("Data folder", Kind::Path, false, "/data doesn't exist inside the container")
                        .request("/data")
                        .fix("Check the volume mapping in your compose file"),
                    Check::new("Upload folder", Kind::Path, true, "OK – 0 entries").request("/uploads").status(Status::Warn),
                ])
            })
            .check("tool", |_app: App| async move { Check::new("git", Kind::Tool, true, "git version 2.47.0").request("git --version") })
            .test(
                "email",
                |_app: App, who: Account| async move { Ok(json!({ "ok": true, "message": format!("Sent to {}", who.username) })) },
            )
            .test("broken", |_app: App, _who: Account| async move { Err("sync: config path: config failed: start sync".to_string()) })
            .tests(|_app: App, id: String, _who: Account| async move {
                (id == "chat").then(|| Ok(json!({ "ok": false, "message": "Turn Chat on and save first" })))
            })
    }

    async fn answer(&self, checks: Vec<Value>) -> Result<Value> {
        if !self.sections {
            return Ok(json!({ "checks": checks }));
        }
        Ok(json!({ "version": "1.2.3", "checks": checks, "time": 1 }))
    }
}

fn app(sections: bool) -> App {
    App { host: TestHost::new(), sections, runs: Arc::default() }
}

async fn call(app: &App, method: &str, uri: &str, user: i64) -> (StatusCode, Value) {
    let r = router::<App>().with_state(app.clone());
    let req = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string()).body(Body::empty()).unwrap();
    let resp = r.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[test]
fn a_check_is_its_json() {
    let c = Check::new("Data folder", Kind::Path, false, "Not set").request("");
    assert_eq!(c.clone().into_json(), json!({ "name": "Data folder", "kind": "path", "request": "", "ok": false, "detail": "Not set" }));
    assert_eq!(
        (c.name(), c.kind(), c.ok(), c.detail(), c.outcome(), c.fix_text()),
        ("Data folder", Some(Kind::Path), false, "Not set", Status::Fail, None)
    );
    // an app's own check, built as JSON, comes out exactly as it went in
    let own = json!({ "name": "Search index", "kind": "service", "request": "GET http://search/api", "ok": true, "status": 200, "body": "{}", "detail": "OK", "ms": 4 });
    let c = Check::from(own.clone());
    assert_eq!(c.outcome(), Status::Ok, "an HTTP status code is not a kit status");
    assert_eq!(c.get("ms"), Some(&json!(4)));
    assert_eq!(Value::from(c), own);
    // the kit status wins over ok; fix is read back
    let c = Check::new("x", Kind::Tool, true, "old").status(Status::Warn).fix("Update it");
    assert_eq!((c.outcome(), c.fix_text()), (Status::Warn, Some("Update it")));
    assert_eq!(Check::new("x", Kind::Service, false, "").status(Status::Ok).outcome(), Status::Ok);
    assert_eq!(Check::from(json!("not an object")), Check::default());
    assert_eq!(Kind::parse("folder"), None);
    assert_eq!(serde_json::to_value(Kind::Tool).unwrap(), json!("tool"));
    assert_eq!(serde_json::to_value(Status::Fail).unwrap(), json!("fail"));
}

#[tokio::test]
async fn every_check_at_once_in_the_order_of_the_groups() {
    let a = app(false);
    let reg = a.diagnostics();
    assert_eq!(reg.groups(), vec!["services", "not set up", "folders", "tool"]);
    let (s, v) = call(&a, "GET", "/api/diagnostics", 1).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let names: Vec<&str> = v["checks"].as_array().unwrap().iter().map(|c| c["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Mail server", "Data folder", "Upload folder", "git"]);
    assert_eq!(v["checks"][0]["status"], json!(200));
    assert_eq!(v["checks"][1]["fix"], json!("Check the volume mapping in your compose file"));
    assert_eq!(v["checks"][2]["status"], json!("warn"));
    assert_eq!(v.as_object().unwrap().len(), 1, "only checks: {v}");
    assert_eq!(a.runs.load(Ordering::SeqCst), 1);
    // one group alone
    assert_eq!(reg.run_group(&a, "tool").await.unwrap().unwrap()[0].name(), "git");
    assert!(reg.run_group(&a, "nope").await.is_none());
}

#[tokio::test]
async fn the_app_adds_its_own_sections() {
    let (s, v) = call(&app(true), "GET", "/api/diagnostics", 1).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!((v["version"].as_str(), v["time"].as_i64(), v["checks"].as_array().map(Vec::len)), (Some("1.2.3"), Some(1), Some(4)));
}

#[tokio::test]
async fn a_failing_group_fails_the_page() {
    #[derive(Clone)]
    struct Broken(TestHost);
    impl Host for Broken {
        async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
            self.0.signed_in(parts).await
        }
    }
    impl Diagnostics for Broken {
        fn diagnostics(&self) -> Registry<Broken> {
            Registry::new().group("tools", |_b: Broken| async move { Err(err(500, "spawn failed")) })
        }
    }
    let r = router::<Broken>().with_state(Broken(TestHost::new()));
    let req = Request::builder().uri("/api/diagnostics").header("x-test-user", "1").body(Body::empty()).unwrap();
    let resp = r.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let v: Value = serde_json::from_slice(&axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap()).unwrap();
    assert_eq!(v, json!({ "detail": "spawn failed" }));
    // a registry without tests knows none
    let b = Broken(TestHost::new());
    assert_eq!(run_test(&b, "email", Account::member(0, "")).await.unwrap_err().detail, "Unknown check");
}

#[tokio::test]
async fn admins_only() {
    let a = app(false);
    assert_eq!(call(&a, "GET", "/api/diagnostics", 2).await, (StatusCode::FORBIDDEN, json!({ "detail": "Admin only" })));
    assert_eq!(call(&a, "GET", "/api/diagnostics", 0).await, (StatusCode::UNAUTHORIZED, json!({ "detail": "Not authenticated" })));
    assert_eq!(call(&a, "POST", "/api/diagnostics/email/test", 2).await.0, StatusCode::FORBIDDEN);
    assert_eq!(a.runs.load(Ordering::SeqCst), 0, "nothing ran");
}

#[tokio::test]
async fn one_test_by_name() {
    let a = app(false);
    assert_eq!(
        call(&a, "POST", "/api/diagnostics/email/test", 1).await,
        (StatusCode::OK, json!({ "ok": true, "message": "Sent to admin" }))
    );
    // failed on the way: the whole sentence, as a failed test
    assert_eq!(
        call(&a, "POST", "/api/diagnostics/broken/test", 1).await,
        (StatusCode::OK, json!({ "ok": false, "message": "sync: config path: config failed: start sync" }))
    );
    // the app's own lookup
    assert_eq!(call(&a, "POST", "/api/diagnostics/chat/test", 1).await.1["message"], json!("Turn Chat on and save first"));
    assert_eq!(call(&a, "POST", "/api/diagnostics/nothing/test", 1).await, (StatusCode::NOT_FOUND, json!({ "detail": "Unknown check" })));
    assert_eq!(call(&a, "GET", "/api/diagnostics/email/test", 1).await.0, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn the_app_can_move_the_routes() {
    let a = app(false);
    let paths = DiagnosticsPaths { list: "/admin/checks".into(), test: "/admin/checks/{name}/run".into() };
    let r = router_with::<App>(paths).with_state(a.clone());
    let req = Request::builder().method("POST").uri("/admin/checks/email/run").header("x-test-user", "1").body(Body::empty()).unwrap();
    let resp = r.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let req = Request::builder().uri("/admin/checks").header("x-test-user", "1").body(Body::empty()).unwrap();
    assert_eq!(r.clone().oneshot(req).await.unwrap().status(), StatusCode::OK);
    let req = Request::builder().uri("/api/diagnostics").header("x-test-user", "1").body(Body::empty()).unwrap();
    assert_eq!(r.oneshot(req).await.unwrap().status(), StatusCode::NOT_FOUND, "the default path is not served");
    assert_eq!(DiagnosticsPaths::default().test, "/api/diagnostics/{id}/test");
}
