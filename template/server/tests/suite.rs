//! Two of these apps running side by side and linked (kit-server-suite): one is the account source, the other
//! shows both apps' pages in its sidebar, searches both, and signs people in with their account from the first.

use serde_json::{json, Value};
use std::path::PathBuf;

/// The password both owners sign in with.
const PASSWORD: &str = "correct horse battery";

/// A running app: its address and state.
struct Running {
    /// `http://<host>:<port>`
    base: String,
    /// the API key that signs in as its owner
    key: &'static str,
}

/// Start an app on `host` (a free port) with its own data folder; `source`: it is an account source.
async fn start(name: &str, host: &str, key: &'static str, source: bool) -> Running {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{host}:{}", listener.local_addr().unwrap().port());
    let dir: PathBuf = std::env::temp_dir().join(format!("app-suite-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let web = dir.join("static");
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(web.join("index.html"), "<!doctype html><title>app</title>").unwrap();
    let (b, title, id) = (base.clone(), name.to_uppercase(), name.to_string());
    let st = server::AppState::open(&dir.join("data"), web, move |var| match var {
        "APP_API_KEY" => Some(key.to_string()),
        "APP_PUBLIC_URL" => Some(b.clone()),
        "APP_NAME" => Some(title.clone()),
        "APP_SUITE_ID" => Some(id.clone()),
        "APP_ACCOUNT_SOURCE" => source.then(|| "true".to_string()),
        _ => None,
    })
    .await
    .unwrap();
    server::auth::ensure_owner(&st, None, Some(PASSWORD.into())).unwrap();
    let app = server::app(st);
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Running { base, key }
}

/// A client for this machine (no proxy), keeping cookies like a browser.
fn browser() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().cookie_store(true).build().unwrap()
}

/// The status and JSON answer of a request as the app's owner (its API key).
async fn owner(c: &reqwest::Client, app: &Running, method: reqwest::Method, path: &str, body: Option<Value>) -> (u16, Value) {
    let mut r = c.request(method, format!("{}{path}", app.base)).header("x-api-key", app.key);
    if let Some(b) = body {
        r = r.json(&b);
    }
    let resp = r.send().await.unwrap();
    (resp.status().as_u16(), resp.json().await.unwrap_or(Value::Null))
}

#[tokio::test]
async fn two_linked_apps_share_the_sidebar_search_and_sign_in() {
    // two hosts, so the two apps' session cookies (same name) stay apart, as on two real addresses
    let source = start("one", "127.0.0.1", "key-one", true).await;
    let linked = start("two", "localhost", "key-two", false).await;
    let c = browser();

    // the manifest says what each app is
    let (_, m) = owner(&c, &source, reqwest::Method::GET, "/api/kit/manifest", None).await;
    assert_eq!((m["name"].as_str(), m["base_url"].as_str()), (Some("ONE"), Some(source.base.as_str())));
    assert_eq!(m["sections"][0]["path"], "/#/");
    assert_eq!(m["sso"]["issuer"], source.base);

    // the source's admin makes a key for the other app; its admin links with it as the account source
    let (s, k) =
        owner(&c, &source, reqwest::Method::POST, "/api/kit/suite/keys", Some(json!({ "name": "TWO", "redirect_url": linked.base }))).await;
    assert_eq!(s, 200, "{k}");
    let body = json!({ "url": source.base, "key": k["key"], "account_source": true });
    let (s, l) = owner(&c, &linked, reqwest::Method::POST, "/api/kit/suite/links", Some(body)).await;
    assert_eq!(s, 200, "{l}");

    // one sidebar with both apps' pages, each opening its own app
    let (_, sb) = owner(&c, &linked, reqwest::Method::GET, "/api/kit/suite/sidebar", None).await;
    let urls: Vec<&str> = sb["items"].as_array().unwrap().iter().map(|i| i["url"].as_str().unwrap()).collect();
    assert_eq!(
        urls,
        [
            format!("{}/#/", linked.base),
            format!("{}/#/bug-reports", linked.base),
            format!("{}/#/", source.base),
            format!("{}/#/bug-reports", source.base)
        ]
    );
    let (_, sb) = owner(
        &c,
        &linked,
        reqwest::Method::PUT,
        "/api/kit/suite/sidebar",
        Some(json!({ "order": ["two/x"], "hidden": [sb["items"][3]["key"]] })),
    )
    .await;
    assert_eq!(sb["items"][3]["hidden"], true);

    // one search across both
    let (_, found) = owner(&c, &linked, reqwest::Method::GET, "/api/kit/suite/search?q=bug", None).await;
    let groups = found["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2, "{found}");
    assert_eq!(groups[1]["app_name"], "ONE");
    assert_eq!(groups[1]["results"][0]["url"], format!("{}/#/bug-reports", source.base));

    // sign in at the source, then into the other app through it: a new account there, an admin like at the source
    let r =
        c.post(format!("{}/api/auth/login", source.base)).json(&json!({ "username": "admin", "password": PASSWORD })).send().await.unwrap();
    assert_eq!(r.status(), 200);
    let (_, sso) = owner(&c, &linked, reqwest::Method::GET, "/api/kit/sso", None).await;
    assert_eq!(sso["source"]["name"], "ONE");
    for _ in 0..2 {
        let r = c.get(format!("{}/api/kit/sso/start?next=/%23/bug-reports", linked.base)).send().await.unwrap();
        assert_eq!(r.status(), 200, "the redirects end on the web app");
        // (an HTTP client drops the `#/bug-reports` of the last redirect, as browsers keep it)
        assert_eq!(r.url().as_str(), format!("{}/", linked.base));
        let me: Value = c.get(format!("{}/api/auth/me", linked.base)).send().await.unwrap().json().await.unwrap();
        // the linked app has an `admin` of its own, so the source's admin gets the next free name, once
        assert_eq!((me["username"].as_str(), me["role"].as_str()), (Some("admin-2"), Some("admin")), "{me}");
    }

    // not signed in at the source: its sign-in page first, which comes back to the sign-in after
    let fresh = browser();
    let r = fresh.get(format!("{}/api/kit/sso/start", linked.base)).send().await.unwrap();
    assert!(r.url().as_str().starts_with(&format!("{}/?next=%2Fapi%2Fkit%2Foidc%2Fauthorize", source.base)), "{}", r.url());
}

#[test]
fn pages_are_searchable() {
    assert_eq!(server::suite::search_pages("HOME", 5)[0].url, "/#/");
    assert!(server::suite::search_pages("nothing", 5).is_empty());
    assert_eq!(server::suite::search_pages("o", 1).len(), 1);
}
