//! The settings routes on a test app: admins only, secrets masked, saves filtered, the owner sees a secret.
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use kit_server_core::testing::TestHost;
use kit_server_core::{err, Account, Host, Result};
use kit_server_settings::file::parse;
use kit_server_settings::{SettingsHost, SettingsPaths, SqliteSettings, Store, Table, Values};
use serde_json::{json, Map, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

const FILE: &str = include_str!("fixtures/settings.toml");

/// The test app: kit's TestHost, a store and what the hooks saw.
#[derive(Clone)]
struct App {
    host: TestHost,
    store: Store,
    log: Arc<Mutex<Vec<String>>>,
    saved: Arc<Mutex<Vec<String>>>,
}

impl Host for App {
    async fn signed_in(&self, parts: &mut axum::http::request::Parts) -> Result<Account> {
        self.host.signed_in(parts).await
    }
}

impl SettingsHost for App {
    fn settings(&self) -> &dyn Values {
        &self.store
    }
    fn kept(&self) -> &'static [&'static str] {
        &["api_key"]
    }
    fn unrevealed(&self, key: &str) -> Option<&'static str> {
        (key == "api_key").then_some("Not this one")
    }
    fn before_save(&self, values: &Map<String, Value>) -> Result<Vec<Value>> {
        if values.get("app_name") == Some(&json!("")) {
            return Err(err(400, "app_name must not be empty"));
        }
        Ok(values.contains_key("rate").then(|| json!({ "key": "rate", "message": "slow" })).into_iter().collect())
    }
    fn after_save(&self, keys: &[String]) {
        self.saved.lock().unwrap().extend(keys.iter().cloned());
    }
    fn log(&self, message: &str) {
        self.log.lock().unwrap().push(message.to_string());
    }
}

fn table() -> Table {
    static T: std::sync::OnceLock<Table> = std::sync::OnceLock::new();
    *T.get_or_init(|| parse(FILE, &[]).unwrap())
}

/// The saved values of the test app's database.
fn saved_on(host: &TestHost) -> SqliteSettings {
    SqliteSettings::new(host.db.clone()).unwrap()
}

async fn app() -> (App, Router) {
    let host = TestHost::new();
    let store =
        Store::load_with(saved_on(&host), table(), |name| (name == "DEMO_S3_SECRET").then(|| " \"from-env\" ".to_string())).await.unwrap();
    let app = App { host, store, log: Default::default(), saved: Default::default() };
    let router = kit_server_settings::router().with_state(app.clone());
    (app, router)
}

async fn call(r: &Router, method: &str, path: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path).header("x-test-user", user.to_string());
    if body.is_some() {
        req = req.header("content-type", "application/json");
    }
    let body = body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty);
    let resp = r.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn the_routes_are_for_admins() {
    let (_, r) = app().await;
    for path in ["/api/settings", "/api/settings/schema"] {
        assert_eq!(call(&r, "GET", path, 0, None).await, (StatusCode::UNAUTHORIZED, json!({ "detail": "Not authenticated" })));
        assert_eq!(call(&r, "GET", path, 2, None).await, (StatusCode::FORBIDDEN, json!({ "detail": "Admin only" })));
        assert_eq!(call(&r, "GET", path, 1, None).await.0, StatusCode::OK);
    }
}

#[tokio::test]
async fn values_come_with_where_they_come_from() {
    let (app, r) = app().await;
    app.store.set_many(&json!({ "api_key": "k", "session_days": "12" })).await.unwrap();
    let (_, v) = call(&r, "GET", "/api/settings", 1, None).await;
    assert_eq!(v["app_name"], "Demo");
    assert_eq!(v["session_days"], 12);
    assert_eq!(v["s3_secret"], "•••• (saved)", "a secret set by the environment is masked");
    assert_eq!(v["api_key"], "•••• (saved)");
    assert_eq!(v["_meta"]["s3_secret"], json!({ "source": "env", "env": "DEMO_S3_SECRET" }));
    assert_eq!(v["_meta"]["session_days"], json!({ "source": "ui", "env": null }));
    assert_eq!(v["_meta"]["app_name"], json!({ "source": "default", "env": "APP_NAME" }));
    assert_eq!(app.store.dump(&["api_key"])["api_key"], "k");
    let (_, s) = call(&r, "GET", "/api/settings/schema", 1, None).await;
    assert_eq!(s, kit_server_settings::schema::answer(&table(), |k| k == "s3_secret"));
}

#[tokio::test]
async fn a_save_leaves_out_unknown_masked_locked_and_kept_values() {
    let (app, r) = app().await;
    let body = json!({ "app_name": "Home", "rate": "0.5", "nope": 1, "s3_secret": "x", "api_key": "mine", "auto_approve": "•••• (saved)" });
    let (code, v) = call(&r, "PUT", "/api/settings", 1, Some(body)).await;
    assert_eq!(code, StatusCode::OK);
    assert_eq!((v["app_name"].clone(), v["rate"].clone()), (json!("Home"), json!(0.5)));
    assert_eq!(v["_warnings"], json!([{ "key": "rate", "message": "slow" }]));
    assert_eq!(app.store.str("api_key"), "");
    assert_eq!(app.store.str("s3_secret"), "from-env");
    assert_eq!(*app.saved.lock().unwrap(), ["app_name", "rate"]);
    assert_eq!(*app.log.lock().unwrap(), ["admin updated settings: app_name, rate"]);
    // a refused save changes nothing and logs nothing
    let (code, v) = call(&r, "PUT", "/api/settings", 1, Some(json!({ "app_name": "" }))).await;
    assert_eq!((code, v), (StatusCode::BAD_REQUEST, json!({ "detail": "app_name must not be empty" })));
    assert_eq!(app.store.str("app_name"), "Home");
    assert_eq!(app.log.lock().unwrap().len(), 1);
    // saved for good: a new store on the same database reads it
    let again = Store::load_with(saved_on(&app.host), table(), |_| None).await.unwrap();
    assert_eq!(again.float("rate"), 0.5);
    assert_eq!(call(&r, "PUT", "/api/settings", 2, Some(json!({}))).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_owner_alone_sees_a_saved_secret() {
    let (app, r) = app().await;
    assert_eq!(
        call(&r, "GET", "/api/settings/secret/s3_secret", 1, None).await,
        (StatusCode::OK, json!({ "key": "s3_secret", "value": "from-env" }))
    );
    assert_eq!(app.log.lock().unwrap().last().unwrap(), "admin viewed the saved s3_secret");
    assert_eq!(call(&r, "GET", "/api/settings/secret/session_days", 1, None).await.1, json!({ "key": "session_days", "value": "30" }));
    assert_eq!(
        call(&r, "GET", "/api/settings/secret/s3_secret", 4, None).await,
        (StatusCode::FORBIDDEN, json!({ "detail": "Only the server owner can do this" }))
    );
    assert_eq!(
        call(&r, "GET", "/api/settings/secret/nope", 1, None).await,
        (StatusCode::NOT_FOUND, json!({ "detail": "Unknown setting" }))
    );
    assert_eq!(
        call(&r, "GET", "/api/settings/secret/api_key", 1, None).await,
        (StatusCode::FORBIDDEN, json!({ "detail": "Not this one" }))
    );
}

#[tokio::test]
async fn the_store_reads_environment_then_saved_then_default() {
    let host = TestHost::new();
    let store = Store::load_with(saved_on(&host), table(), |name| (name == "APP_NAME").then(|| "'Env name'".to_string())).await.unwrap();
    let g = store.generation();
    store.set_many(&json!({ "app_name": "Saved", "auto_approve": "yes", "nope": 1 })).await.unwrap();
    assert!(store.generation() > g);
    assert_eq!(store.str("app_name"), "Env name");
    assert!(store.env_locked("app_name") && !store.env_locked("rate"));
    assert!(store.bool("auto_approve"));
    assert_eq!((store.int("session_days"), store.float("rate"), store.get("nope")), (30, 1.0, Value::Null));
    assert_eq!(store.def("zz_limit").unwrap().default, json!(1));
    let mut saved = store.saved().await.unwrap();
    assert_eq!(saved["app_name"], "Saved");
    saved.insert("session_days".into(), json!("7"));
    store.apply(&saved);
    assert_eq!(store.int("session_days"), 7);
}

#[tokio::test]
async fn the_routes_can_live_elsewhere() {
    let (app, _) = app().await;
    let paths = SettingsPaths {
        values: "/api/v2/config".into(),
        schema: "/api/v2/config-schema".into(),
        secret: "/api/v2/config-secret/{key}".into(),
    };
    let r = kit_server_settings::router_with(paths).with_state(app);
    assert_eq!(call(&r, "GET", "/api/v2/config", 1, None).await.1["app_name"], "Demo");
    assert_eq!(call(&r, "GET", "/api/v2/config-schema", 1, None).await.0, StatusCode::OK);
    assert_eq!(call(&r, "GET", "/api/v2/config-secret/session_days", 1, None).await.1["value"], "30");
    assert_eq!(call(&r, "GET", "/api/settings", 1, None).await.0, StatusCode::NOT_FOUND);
}

#[test]
fn the_default_paths_are_neutral() {
    let p = SettingsPaths::default();
    assert_eq!(
        (p.values.as_str(), p.schema.as_str(), p.secret.as_str()),
        ("/api/settings", "/api/settings/schema", "/api/settings/secret/{key}")
    );
}
