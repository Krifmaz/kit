//! The sign-up routes through the router on [`TestApp`] (the kit's [`TestHost`] with the sign-up data in SQLite,
//! or in Postgres with feature `postgres`): the setting, invites (one-time, multi-use, expiry, revoke, who used
//! them), open sign-up (limits, email confirmation, approval), who may do what, and an account source.

use super::*;
use axum::body::Body;
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use kit_server_core::testing::TestHost;
use kit_server_core::{err, Account};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;
use tower::ServiceExt;

/// What the test app holds besides the kit's data: its accounts, mails and limit counts.
#[derive(Default)]
struct World {
    /// accounts made by sign-up: id → (name, role, presets, enabled)
    accounts: HashMap<i64, (String, String, Vec<String>, bool)>,
    /// mails "sent": (to, subject, text)
    mails: Vec<(String, String, String)>,
    /// rate limit counts by key
    counts: HashMap<String, usize>,
    /// the account source, when set
    source: Option<AccountSource>,
}

/// The tests' app.
#[derive(Clone)]
struct TestApp {
    /// the fixed accounts (1 admin, 2 member) and the database
    host: TestHost,
    /// the sign-up data
    store: Arc<dyn SignupStore>,
    /// accounts, mails, counts
    world: Arc<Mutex<World>>,
}

impl TestApp {
    fn sqlite() -> TestApp {
        let host = TestHost::new();
        let store = SqliteSignup::new(host.db.clone()).unwrap();
        TestApp { host, store: Arc::new(store), world: Default::default() }
    }

    #[cfg(feature = "postgres")]
    async fn postgres() -> TestApp {
        let store = PgSignup::new(kit_server_core::pg::test_db().await).await.unwrap();
        TestApp { host: TestHost::new(), store: Arc::new(store), world: Default::default() }
    }

    fn world(&self) -> std::sync::MutexGuard<'_, World> {
        self.world.lock().unwrap()
    }
}

impl Host for TestApp {
    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        self.host.signed_in(parts).await
    }
}

/// A mailer that keeps what it sends in the world.
struct KeepMail(Arc<Mutex<World>>);

#[kit_server_core::async_trait]
impl Mailer for KeepMail {
    async fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        self.0.lock().unwrap().mails.push((to.into(), subject.into(), text.into()));
        Ok(())
    }
}

impl SignupHost for TestApp {
    fn signup(&self) -> &dyn SignupStore {
        self.store.as_ref()
    }
    fn app_name(&self) -> String {
        "Demo".into()
    }
    fn roles(&self) -> Vec<Choice> {
        vec![Choice::new("member", "Member"), Choice::new("viewer", "Viewer"), Choice::new("admin", "Admin")]
    }
    fn presets(&self) -> Vec<Choice> {
        vec![Choice::new("shared", "Shared folder"), Choice::new("upload", "May upload")]
    }
    async fn create_account(&self, a: NewAccount) -> Result<i64> {
        let mut w = self.world();
        if a.username.len() < 3 {
            return Err(err(400, "Username must be 3-32 characters"));
        }
        if w.accounts.values().any(|(n, ..)| *n == a.username) {
            return Err(err(409, "Username already taken"));
        }
        let id = 100 + w.accounts.len() as i64;
        w.accounts.insert(id, (a.username, a.role, a.presets, a.enabled));
        Ok(id)
    }
    async fn enable_account(&self, id: i64) -> Result<()> {
        self.world().accounts.get_mut(&id).ok_or_else(|| err(404, "no account"))?.3 = true;
        Ok(())
    }
    async fn remove_account(&self, id: i64) -> Result<()> {
        self.world().accounts.remove(&id);
        Ok(())
    }
    fn limited(&self, key: &str, limit: usize, _window_secs: f64) -> bool {
        let mut w = self.world();
        let n = w.counts.entry(key.to_string()).or_default();
        if *n >= limit {
            return true;
        }
        *n += 1;
        false
    }
    fn base_url(&self) -> String {
        "https://demo.example.com".into()
    }
    fn mailer(&self) -> Arc<dyn Mailer> {
        Arc::new(KeepMail(self.world.clone()))
    }
    async fn account_source(&self) -> Option<AccountSource> {
        self.world().source.clone()
    }
}

/// One request: `user` 0 for none, 1 admin, 2 member; `from` the address.
async fn call(app: &TestApp, method: &str, path: &str, user: i64, from: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path).header("x-test-user", user.to_string()).header("x-forwarded-for", from);
    if body.is_some() {
        req = req.header("content-type", "application/json");
    }
    let req = req.body(body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty)).unwrap();
    let resp = router::<TestApp>().with_state(app.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// The token in an invite link or a mail's link.
fn token_in(text: &str) -> String {
    let at = text.find("token=").or_else(|| text.find("invite=")).expect("a link");
    text[at..].split('=').nth(1).unwrap().chars().take_while(|c| c.is_ascii_hexdigit()).collect()
}

/// Every test, on one store.
async fn all(app: TestApp) {
    setting_and_who_may_change_it(&app).await;
    invites(&app).await;
    open_sign_up(&app).await;
    account_source(&app).await;
}

#[tokio::test]
async fn sqlite() {
    all(TestApp::sqlite()).await;
}

#[cfg(feature = "postgres")]
#[tokio::test]
async fn postgres() {
    all(TestApp::postgres().await).await;
}

async fn setting_and_who_may_change_it(app: &TestApp) {
    // invite only by default, for anyone to read
    let (s, p) = call(app, "GET", "/api/signup", 0, "a", None).await;
    assert_eq!((s, p), (StatusCode::OK, json!({ "who": "invite", "verify_email": false, "approval": false, "managed_by": null })));
    // admins only
    assert_eq!(call(app, "GET", "/api/signup/settings", 0, "a", None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(app, "GET", "/api/signup/settings", 2, "a", None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(app, "PUT", "/api/signup/settings", 2, "a", Some(json!({ "who": "open" }))).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(app, "GET", "/api/signup/invites", 2, "a", None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(app, "GET", "/api/signup/pending", 2, "a", None).await.0, StatusCode::FORBIDDEN);
    let (s, v) = call(app, "GET", "/api/signup/settings", 1, "a", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["settings"]["who"], "invite");
    assert_eq!(v["roles"][0], json!({ "id": "member", "label": "Member" }));
    assert_eq!(v["presets"].as_array().unwrap().len(), 2);
    assert_eq!((v["read_only"].clone(), v["mail_delivers"].clone()), (json!(false), json!(true)));
    // bad values are refused with their reason
    let (s, v) = call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "who": "everyone" }))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(v["detail"].as_str().unwrap().starts_with("The sign-up setting isn't valid"));
    assert_eq!(call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "open_role": "king" }))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "open_presets": ["x"] }))).await.0, StatusCode::BAD_REQUEST);
    // closed: no sign-up, no invite check
    let (s, v) = call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "who": "closed", "per_hour": 0 }))).await;
    assert_eq!((s, v["settings"]["who"].clone(), v["settings"]["per_hour"].clone()), (StatusCode::OK, json!("closed"), json!(1)));
    let (s, v) = call(app, "POST", "/api/signup", 0, "a", Some(json!({ "username": "zed", "password": "password1" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::FORBIDDEN, Words::default().closed.as_str()));
    assert_eq!(call(app, "POST", "/api/signup/invite", 0, "a", Some(json!({ "token": "x" }))).await.0, StatusCode::FORBIDDEN);
    call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "who": "invite" }))).await;
    let (s, v) = call(app, "POST", "/api/signup", 0, "a", Some(json!({ "username": "zed", "password": "password1" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::FORBIDDEN, Words::default().invite_only.as_str()));
}

async fn invites(app: &TestApp) {
    // made by an admin: a one-time link shown once, kept as a hash
    let (s, v) = call(
        app,
        "POST",
        "/api/signup/invites",
        1,
        "a",
        Some(json!({ "note": "for Ana", "role": "viewer", "presets": ["upload", "shared"] })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let once = v["token"].as_str().unwrap().to_string();
    assert!(v["link"].as_str().unwrap().starts_with("https://demo.example.com/#/sign-up?invite="));
    assert_eq!(token_in(v["link"].as_str().unwrap()), once);
    let inv = &v["invite"];
    assert_eq!((inv["max_uses"].clone(), inv["status"].clone(), inv["role"].clone()), (json!(1), json!("active"), json!("viewer")));
    assert_eq!(inv["presets"], json!(["shared", "upload"]), "in the app's order");
    assert_eq!(inv["created_by_name"], "admin");
    assert!(inv.get("token_hash").is_none());
    let once_id = inv["id"].as_i64().unwrap();
    // bad invites are refused
    for bad in [json!({ "role": "king" }), json!({ "presets": ["nope"] }), json!({ "max_uses": -1 }), json!({ "expires_in_hours": -2 })] {
        assert_eq!(call(app, "POST", "/api/signup/invites", 1, "a", Some(bad.clone())).await.0, StatusCode::BAD_REQUEST, "{bad}");
    }
    // anyone may check a link
    let (s, v) = call(app, "POST", "/api/signup/invite", 0, "b", Some(json!({ "token": once }))).await;
    assert_eq!((s, v["role"].clone(), v["presets"].clone()), (StatusCode::OK, json!("Viewer"), json!(["Shared folder", "May upload"])));
    let (s, v) = call(app, "POST", "/api/signup/invite", 0, "b", Some(json!({ "token": "0".repeat(64) }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::NOT_FOUND, Words::default().invite_unknown.as_str()));
    // a failed account gives the use back
    let (s, _) = call(app, "POST", "/api/signup", 0, "b", Some(json!({ "username": "an", "password": "password1", "invite": once }))).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "b", Some(json!({ "username": "ana2", "password": "password1", "invite": once }))).await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("active")), "{v}");
    let id = v["account_id"].as_i64().unwrap();
    assert_eq!(app.world().accounts[&id], ("ana2".into(), "viewer".into(), vec!["shared".into(), "upload".into()], true));
    // used once: used up
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "b", Some(json!({ "username": "ana3", "password": "password1", "invite": once }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::GONE, Words::default().invite_used_up.as_str()));
    // multi-use with a limit, then revoked; who used it is listed
    let (_, v) = call(app, "POST", "/api/signup/invites", 1, "a", Some(json!({ "max_uses": 3, "expires_in_hours": 24 }))).await;
    let many = v["token"].as_str().unwrap().to_string();
    let many_id = v["invite"]["id"].as_i64().unwrap();
    assert_eq!(v["invite"]["role"], "member", "the app's first role");
    assert!(v["invite"]["expires_at"].as_f64().unwrap() > kit_server_core::db::now() + 23.0 * 3600.0);
    for name in ["bea", "cal"] {
        let (s, _) =
            call(app, "POST", "/api/signup", 0, "c", Some(json!({ "username": name, "password": "password1", "invite": many }))).await;
        assert_eq!(s, StatusCode::OK);
    }
    let (s, v) = call(app, "POST", &format!("/api/signup/invites/{many_id}/revoke"), 1, "a", None).await;
    assert_eq!((s, v["invite"]["status"].clone()), (StatusCode::OK, json!("revoked")));
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "c", Some(json!({ "username": "dan", "password": "password1", "invite": many }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::GONE, Words::default().invite_revoked.as_str()));
    assert_eq!(call(app, "POST", "/api/signup/invites/999/revoke", 1, "a", None).await.0, StatusCode::NOT_FOUND);
    let (_, v) = call(app, "GET", &format!("/api/signup/invites/{many_id}"), 1, "a", None).await;
    let names: Vec<&str> = v["invite"]["used_by"].as_array().unwrap().iter().map(|u| u["username"].as_str().unwrap()).collect();
    assert_eq!((names, v["invite"]["uses"].clone()), (vec!["bea", "cal"], json!(2)));
    let (_, v) = call(app, "GET", "/api/signup/invites", 1, "a", None).await;
    let list = v["invites"].as_array().unwrap();
    assert_eq!(list.iter().map(|i| i["id"].as_i64().unwrap()).collect::<Vec<_>>(), vec![many_id, once_id]);
    assert_eq!(list[1]["status"], "used_up");
    assert_eq!(list[1]["used_by"][0]["username"], "ana2");
    // an expired invite
    let past = NewInvite {
        token_hash: token::hash("old"),
        note: String::new(),
        role: "member".into(),
        presets: vec![],
        max_uses: 0,
        expires_at: 1.0,
        created_by: 1,
        created_by_name: "admin".into(),
    };
    app.store.create_invite(&past).await.unwrap();
    let (s, v) = call(app, "POST", "/api/signup/invite", 0, "c", Some(json!({ "token": "old" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::GONE, Words::default().invite_expired.as_str()));
    // too many tries from one address
    for _ in 0..40 {
        call(app, "POST", "/api/signup/invite", 0, "guesser", Some(json!({ "token": "x" }))).await;
    }
    let (s, v) = call(app, "POST", "/api/signup/invite", 0, "guesser", Some(json!({ "token": once }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::TOO_MANY_REQUESTS, Words::default().too_many.as_str()));
}

async fn open_sign_up(app: &TestApp) {
    // open, nothing to wait for: the account is on, with the open role and presets
    let set = json!({ "who": "open", "open_role": "viewer", "open_presets": ["shared"], "per_hour": 2, "per_day": 5 });
    assert_eq!(call(app, "PUT", "/api/signup/settings", 1, "a", Some(set)).await.0, StatusCode::OK);
    let (s, v) = call(app, "POST", "/api/signup", 0, "o1", Some(json!({ "username": "olga", "password": "password1" }))).await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("active")));
    assert_eq!(app.world().accounts[&v["account_id"].as_i64().unwrap()].1, "viewer");
    assert_eq!(app.world().accounts[&v["account_id"].as_i64().unwrap()].2, vec!["shared".to_string()]);
    // a taken name, a bad email
    assert_eq!(
        call(app, "POST", "/api/signup", 0, "o2", Some(json!({ "username": "olga", "password": "password1" }))).await.0,
        StatusCode::CONFLICT
    );
    let (s, v) = call(app, "POST", "/api/signup", 0, "o2", Some(json!({ "username": "oz", "password": "p", "email": "nope" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::BAD_REQUEST, Words::default().email_bad.as_str()));
    // per address: 2 an hour
    call(app, "POST", "/api/signup", 0, "o1", Some(json!({ "username": "olaf", "password": "password1" }))).await;
    let (s, v) = call(app, "POST", "/api/signup", 0, "o1", Some(json!({ "username": "omar", "password": "password1" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::TOO_MANY_REQUESTS, Words::default().too_many.as_str()));

    // email confirmation and approval
    let set = json!({ "who": "open", "verify_email": true, "approval": true, "per_day": 0 });
    call(app, "PUT", "/api/signup/settings", 1, "a", Some(set)).await;
    let (_, p) = call(app, "GET", "/api/signup", 0, "a", None).await;
    assert_eq!((p["verify_email"].clone(), p["approval"].clone()), (json!(true), json!(true)));
    let (s, v) = call(app, "POST", "/api/signup", 0, "v1", Some(json!({ "username": "vic", "password": "password1" }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (StatusCode::BAD_REQUEST, Words::default().email_needed.as_str()));
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "v1", Some(json!({ "username": "vic", "password": "password1", "email": "vic@example.com" })))
            .await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("verify_email")));
    let vic = v["account_id"].as_i64().unwrap();
    assert!(!app.world().accounts[&vic].3, "switched off while it waits");
    assert_eq!(waiting(app, vic, &Words::default()).await.unwrap(), Some(Words::default().waiting_email));
    let (to, subject, text) = app.world().mails.last().cloned().unwrap();
    assert_eq!((to.as_str(), subject.as_str()), ("vic@example.com", "Demo: confirm your email"));
    assert!(text.contains("Hello vic") && text.contains("https://demo.example.com/#/sign-up/verify?token=") && text.contains("48 hours"));
    // a wrong link; the right one; used twice
    assert_eq!(call(app, "POST", "/api/signup/verify", 0, "v1", Some(json!({ "token": "abc" }))).await.0, StatusCode::BAD_REQUEST);
    let tok = token_in(&text);
    let (s, v) = call(app, "POST", "/api/signup/verify", 0, "v1", Some(json!({ "token": tok }))).await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("approval")));
    assert_eq!(call(app, "POST", "/api/signup/verify", 0, "v1", Some(json!({ "token": tok }))).await.0, StatusCode::BAD_REQUEST);
    assert_eq!(waiting(app, vic, &Words::default()).await.unwrap(), Some(Words::default().waiting_approval));
    // the admin's list, approve, reject
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "v2", Some(json!({ "username": "wes", "password": "password1", "email": "wes@example.com" })))
            .await;
    assert_eq!(s, StatusCode::OK);
    let wes = v["account_id"].as_i64().unwrap();
    let (_, v) = call(app, "GET", "/api/signup/pending", 1, "a", None).await;
    let pending = v["pending"].as_array().unwrap();
    assert_eq!(
        pending.iter().map(|p| (p["username"].as_str().unwrap(), p["waiting_for"].as_str().unwrap())).collect::<Vec<_>>(),
        vec![("vic", "approval"), ("wes", "verify_email")]
    );
    assert_eq!(pending[0]["address"], "v1");
    let (vic_req, wes_req) = (pending[0]["id"].as_i64().unwrap(), pending[1]["id"].as_i64().unwrap());
    let (s, v) = call(app, "POST", &format!("/api/signup/pending/{vic_req}/approve"), 1, "a", None).await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("active")));
    assert!(app.world().accounts[&vic].3);
    assert_eq!(waiting(app, vic, &Words::default()).await.unwrap(), None);
    assert_eq!(call(app, "POST", &format!("/api/signup/pending/{wes_req}/reject"), 1, "a", None).await.0, StatusCode::OK);
    assert!(!app.world().accounts.contains_key(&wes), "a turned-away account is removed");
    assert_eq!(call(app, "POST", &format!("/api/signup/pending/{wes_req}/approve"), 1, "a", None).await.0, StatusCode::NOT_FOUND);
    assert!(call(app, "GET", "/api/signup/pending", 1, "a", None).await.1["pending"].as_array().unwrap().is_empty());
    // an invite skips both, even now
    let (_, v) = call(app, "POST", "/api/signup/invites", 1, "a", Some(json!({}))).await;
    let tok = v["token"].as_str().unwrap().to_string();
    let (s, v) =
        call(app, "POST", "/api/signup", 0, "v3", Some(json!({ "username": "xia", "password": "password1", "invite": tok }))).await;
    assert_eq!((s, v["status"].clone()), (StatusCode::OK, json!("active")));
    // approval alone: no mail
    call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "who": "open", "approval": true }))).await;
    let mails = app.world().mails.len();
    let (_, v) = call(app, "POST", "/api/signup", 0, "v4", Some(json!({ "username": "yan", "password": "password1" }))).await;
    assert_eq!(v["status"], "approval");
    assert_eq!(app.world().mails.len(), mails);
}

async fn account_source(app: &TestApp) {
    app.world().source = Some(AccountSource {
        name: "Accounts".into(),
        url: "https://accounts.example.com".into(),
        policy: Some(json!({ "who": "closed", "verify_email": false, "approval": false, "managed_by": null })),
    });
    let (_, p) = call(app, "GET", "/api/signup", 0, "a", None).await;
    assert_eq!(p["who"], "closed");
    assert_eq!(p["managed_by"], json!({ "name": "Accounts", "url": "https://accounts.example.com", "reachable": true }));
    let (_, v) = call(app, "GET", "/api/signup/settings", 1, "a", None).await;
    assert_eq!((v["read_only"].clone(), v["settings"]["who"].clone()), (json!(true), json!("closed")));
    let (s, v) = call(app, "PUT", "/api/signup/settings", 1, "a", Some(json!({ "who": "open" }))).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(v["detail"], "Accounts are managed by Accounts. Sign up there: the same account works here.");
    assert_eq!(v["managed_by"]["url"], "https://accounts.example.com");
    assert_eq!(
        call(app, "POST", "/api/signup", 0, "s", Some(json!({ "username": "sam", "password": "password1" }))).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(call(app, "POST", "/api/signup/invites", 1, "a", Some(json!({}))).await.0, StatusCode::CONFLICT);
    // the source can't be reached: still read-only, and says so
    app.world().source.as_mut().unwrap().policy = None;
    let (_, p) = call(app, "GET", "/api/signup", 0, "a", None).await;
    assert_eq!((p["who"].clone(), p["managed_by"]["reachable"].clone()), (Value::Null, json!(false)));
    app.world().source = None;
    assert_eq!(call(app, "GET", "/api/signup", 0, "a", None).await.1["managed_by"], Value::Null);
}

/// With kit-server-suite: the account-source link's sign-up setting, read from that app over HTTP; none without
/// such a link; no policy while that app can't be reached.
#[cfg(feature = "suite")]
#[tokio::test]
async fn a_suite_account_source_brings_its_setting() {
    use kit_server_suite::{Link, SqliteSuite, Suite, SuiteConfig, SuiteStore};
    // the account source: an app with open sign-up, on a free port
    let source = TestApp::sqlite();
    source.store.save_settings(&json!({ "who": "open" })).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = router::<TestApp>().with_state(source);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    // this app: a suite with links, none the account source yet
    let host = TestHost::new();
    let links = SqliteSuite::new(host.db.clone()).unwrap();
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let suite = Suite::with_http(SuiteConfig::new("here", "Here"), http);
    let link = |url: &str, on: bool| Link {
        id: 0,
        base_url: url.into(),
        api_key: "key".into(),
        account_source: on,
        manifest: None,
        error: None,
        checked_at: 0.0,
        created_at: 0.0,
    };
    links.add_link(&link(&base, false)).await.unwrap();
    assert_eq!(suite_account_source(&suite, &links, "/api/signup").await, None);

    let id = links.links().await.unwrap()[0].id;
    links.set_account_source(Some(id)).await.unwrap();
    let got = suite_account_source(&suite, &links, "/api/signup").await.unwrap();
    assert_eq!((got.name.as_str(), got.url.as_str()), (base.as_str(), base.as_str()));
    assert_eq!(got.policy.unwrap()["who"], "open");

    // gone: still the source, with no setting to show
    let gone = links.add_link(&link("http://127.0.0.1:9", false)).await.unwrap();
    links.set_account_source(Some(gone)).await.unwrap();
    let got = suite_account_source(&suite, &links, "/api/signup").await.unwrap();
    assert_eq!((got.url.as_str(), got.policy), ("http://127.0.0.1:9", None));
}
