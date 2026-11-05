//! The setup route through the router on a test app that keeps its accounts in memory: the screen shows only
//! while there is no owner, the code is needed and checked, a failed account keeps the code, the code stops
//! working once the owner exists, and wrong codes are limited.

use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// The tests' app: its setup code and its accounts (name, password, email).
#[derive(Clone, Default)]
struct App {
    /// the setup code
    code: Arc<SetupCode>,
    /// the accounts made
    owners: Arc<Mutex<Vec<NewOwner>>>,
}

impl App {
    fn open(code: &str) -> App {
        App { code: Arc::new(SetupCode::with_code(code)), owners: Arc::default() }
    }
}

impl SetupHost for App {
    fn setup_code(&self) -> &SetupCode {
        &self.code
    }
    async fn has_owner(&self) -> Result<bool> {
        Ok(!self.owners.lock().unwrap().is_empty())
    }
    async fn create_owner(&self, owner: NewOwner) -> Result<()> {
        if owner.password.len() < 8 {
            return Err(err(400, "Password must be at least 8 characters"));
        }
        self.owners.lock().unwrap().push(owner);
        Ok(())
    }
}

/// One request to the routes: its status and JSON body.
async fn call(app: &App, method: &str, body: Value, from: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(method)
        .uri("/api/setup")
        .header("content-type", "application/json")
        .header("x-forwarded-for", from)
        .body(if method == "GET" { Body::empty() } else { Body::from(body.to_string()) })
        .unwrap();
    let res = router::<App>().with_state(app.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn owner(code: &str, pw: &str) -> Value {
    json!({ "code": code, "username": " ana ", "password": pw, "email": "ana@example.com" })
}

#[tokio::test]
async fn the_owner_is_made_once_with_the_code() {
    let app = App::open("ABCD-EFGH-JKMN");
    assert_eq!(call(&app, "GET", Value::Null, "a").await, (StatusCode::OK, json!({ "needed": true })));

    // no code, a wrong code
    let (s, b) = call(&app, "POST", owner("", "correct horse"), "a").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::BAD_REQUEST, Words::default().need_code.as_str()));
    let (s, b) = call(&app, "POST", owner("ABCD-EFGH-JKMM", "correct horse"), "a").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::FORBIDDEN, Words::default().wrong_code.as_str()));

    // the app turns the password down: its words, and the code still works
    let (s, b) = call(&app, "POST", owner("abcd efgh jkmn", "short"), "a").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::BAD_REQUEST, "Password must be at least 8 characters"));
    assert!(app.code.is_open());

    // typed in lower case with spaces: made, trimmed
    let (s, b) = call(&app, "POST", owner("abcd efgh jkmn", "correct horse"), "a").await;
    assert_eq!((s, b), (StatusCode::OK, json!({ "ok": true, "username": "ana" })));
    let made = app.owners.lock().unwrap().clone();
    assert_eq!(made, vec![NewOwner { username: "ana".into(), password: "correct horse".into(), email: "ana@example.com".into() }]);

    // the code is spent: the screen is gone and the same code makes nobody else
    assert!(!app.code.is_open());
    assert_eq!(app.code.code(), None);
    assert_eq!(call(&app, "GET", Value::Null, "a").await.1, json!({ "needed": false }));
    let (s, b) = call(&app, "POST", owner("ABCD-EFGH-JKMN", "correct horse"), "b").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::CONFLICT, Words::default().done_already.as_str()));
    assert_eq!(app.owners.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn closed_setup_shows_nothing_and_takes_nothing() {
    let app = App { code: Arc::new(SetupCode::closed()), owners: Arc::default() };
    assert_eq!(call(&app, "GET", Value::Null, "a").await.1, json!({ "needed": false }));
    let (s, b) = call(&app, "POST", owner("ABCD-EFGH-JKMN", "correct horse"), "a").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::CONFLICT, Words::default().closed.as_str()));
    assert!(app.owners.lock().unwrap().is_empty());
    // an owner made from settings closes an open code too
    let app = App::open("ABCD-EFGH-JKMN");
    app.owners.lock().unwrap().push(NewOwner { username: "admin".into(), password: "x".repeat(8), email: String::new() });
    assert_eq!(call(&app, "GET", Value::Null, "a").await.1, json!({ "needed": false }));
    assert_eq!(call(&app, "POST", owner("ABCD-EFGH-JKMN", "correct horse"), "a").await.0, StatusCode::CONFLICT);
    assert!(!app.code.is_open());
}

#[tokio::test]
async fn wrong_codes_are_limited_per_address_and_in_all() {
    let app = App::open("ABCD-EFGH-JKMN");
    for _ in 0..10 {
        assert_eq!(call(&app, "POST", owner("WRONG", "correct horse"), "a").await.0, StatusCode::FORBIDDEN);
    }
    // address a is stopped, even with the right code; b still gets through
    let (s, b) = call(&app, "POST", owner("ABCD-EFGH-JKMN", "correct horse"), "a").await;
    assert_eq!((s, b["detail"].as_str().unwrap()), (StatusCode::TOO_MANY_REQUESTS, Words::default().too_many.as_str()));
    // 40 more from other addresses: everyone is stopped
    for i in 0..40 {
        assert_eq!(call(&app, "POST", owner("WRONG", "correct horse"), &format!("x{i}")).await.0, StatusCode::FORBIDDEN);
    }
    assert_eq!(call(&app, "POST", owner("ABCD-EFGH-JKMN", "correct horse"), "b").await.0, StatusCode::TOO_MANY_REQUESTS);
    assert!(app.owners.lock().unwrap().is_empty());
}

#[tokio::test]
async fn a_bad_email_is_turned_down_before_the_code_is_used() {
    let app = App::open("ABCD-EFGH-JKMN");
    let mut b = owner("ABCD-EFGH-JKMN", "correct horse");
    b["email"] = json!("not an email");
    let (s, body) = call(&app, "POST", b, "a").await;
    assert_eq!((s, body["detail"].as_str().unwrap()), (StatusCode::BAD_REQUEST, Words::default().email_bad.as_str()));
    assert!(app.code.is_open());
}

#[test]
fn codes_are_random_readable_and_never_printed_by_debug() {
    let (a, b) = (SetupCode::new(), SetupCode::new());
    let ca = a.code().unwrap();
    assert_eq!(ca.len(), CODE_LEN + 2, "three groups of four");
    assert_eq!(ca.matches('-').count(), 2);
    assert_ne!(Some(ca.clone()), b.code());
    assert!(normalize(&ca).bytes().all(|c| ALPHABET.contains(&c)));
    assert!(!format!("{a:?}").contains(&normalize(&ca)[..4]), "Debug must not show the code");
    assert!(a.banner("Notes").unwrap().contains(&ca));
    assert_eq!(SetupCode::closed().banner("Notes"), None);
    assert_eq!(normalize(" ab-cd 23\n"), "ABCD23");
    assert_eq!(grouped("ABCDEFGHJKMN"), "ABCD-EFGH-JKMN");
    assert!(!SetupCode::at_start(false, "Notes").is_open());
    assert!(SetupCode::at_start(true, "Notes").is_open());
    let c = SetupCode::closed();
    c.open("Notes");
    assert!(c.is_open() && c.code() != a.code());
}
