//! Two apps on real ports, linked to each other: manifest, keys, links, the sidebar, search and single
//! sign-on end to end.

use super::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use kit_server_core::testing::TestHost;

/// A test app: the kit's test host (accounts by `x-test-user`), its suite and stores.
#[derive(Clone)]
struct App {
    /// accounts and the database
    core: TestHost,
    /// the suite
    suite: Suite,
    /// links and keys
    store: SqliteSuite,
}

impl Host for App {
    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        self.core.signed_in(parts).await
    }
}

impl SuiteHost for App {
    fn suite(&self) -> &Suite {
        &self.suite
    }
    fn suite_store(&self) -> &dyn SuiteStore {
        &self.store
    }
    fn sidebar_prefs(&self) -> &dyn PrefsStore {
        &self.core.prefs
    }
    async fn search(&self, caller: Caller, q: String, limit: usize) -> Result<Vec<SearchHit>> {
        let who = match caller {
            Caller::Account(a) => a.username,
            Caller::App { name, user, .. } => format!("{name}:{}", user.unwrap_or_default()),
        };
        if q == "boom" {
            return Err(err(500, "The index is broken."));
        }
        Ok((0..limit.min(3))
            .map(|i| SearchHit {
                title: format!("{q} {i}"),
                subtitle: Some(who.clone()),
                url: format!("/things/{i}"),
                ..SearchHit::default()
            })
            .collect())
    }
    async fn sso_sign_in(&self, user: SsoUser, next: String) -> Result<Response> {
        Ok(Json(json!({ "issuer": user.issuer, "sub": user.subject, "username": user.username, "admin": user.admin, "next": next }))
            .into_response())
    }
}

/// A started app: its address and state.
struct Running {
    /// `http://127.0.0.1:<port>`
    base: String,
    /// the state
    app: App,
}

/// An HTTP client that never goes through a proxy (the apps are on this machine) and follows no redirects.
fn http() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap()
}

/// Start app `id` with `config` changes on a free port.
async fn start(id: &str, f: impl FnOnce(SuiteConfig) -> SuiteConfig) -> Running {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let core = TestHost::new();
    let store = SqliteSuite::new(core.db.clone()).unwrap();
    let config = f(SuiteConfig::new(id, &id.to_uppercase()).icon("M0 0h24").version("1.0.0").base_url(&base));
    let app = App { core, suite: Suite::with_http(config, http()), store };
    let router = router::<App>().with_state(app.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Running { base, app }
}

/// A request as test user `user` (0: nobody).
fn req(c: &reqwest::Client, m: reqwest::Method, url: String, user: i64) -> reqwest::RequestBuilder {
    let r = c.request(m, url);
    if user > 0 {
        r.header("x-test-user", user.to_string())
    } else {
        r
    }
}

/// The status and JSON body of a request.
async fn send(r: reqwest::RequestBuilder) -> (u16, Value) {
    let resp = r.send().await.unwrap();
    let s = resp.status().as_u16();
    (s, resp.json().await.unwrap_or(Value::Null))
}

/// A key made in `app` by its admin for an app at `redirect`.
async fn make_key(c: &reqwest::Client, app: &Running, name: &str, redirect: Option<&str>) -> String {
    let (s, v) = send(
        req(c, reqwest::Method::POST, format!("{}/api/kit/suite/keys", app.base), 1)
            .json(&json!({ "name": name, "redirect_url": redirect })),
    )
    .await;
    assert_eq!(s, 200, "{v}");
    v["key"].as_str().unwrap().to_string()
}

/// The source app (`one`, an account source, searchable) and the linked app (`two`) with `two` linked to `one`
/// as its account source.
async fn linked() -> (reqwest::Client, Running, Running) {
    let c = http();
    let one = start("one", |c| {
        c.section("things", "Things", "M1 1", "/things").section("more", "More", "", "/more").searchable().account_source(true)
    })
    .await;
    let two = start("two", |c| c.section("stuff", "Stuff", "M2 2", "/stuff").deep_link("item", "/stuff/{id}").searchable()).await;
    let key = make_key(&c, &one, "Two", Some(&two.base)).await;
    let (s, v) = send(
        req(&c, reqwest::Method::POST, format!("{}/api/kit/suite/links", two.base), 1)
            .json(&json!({ "url": format!("{}/", one.base), "key": key, "account_source": true })),
    )
    .await;
    assert_eq!(s, 200, "{v}");
    assert_eq!(v["name"], "ONE");
    assert_eq!(v["account_source"], true);
    assert!(v.get("api_key").is_none());
    (c, one, two)
}

#[tokio::test]
async fn the_manifest_says_what_the_app_is() {
    let c = http();
    let one = start("one", |c| {
        c.section("things", "Things", "M1 1", "/things").deep_link("thing", "/things/{id}").searchable().account_source(true)
    })
    .await;
    let (s, m) = send(c.get(format!("{}/api/kit/manifest", one.base))).await;
    assert_eq!(s, 200);
    let m: Manifest = serde_json::from_value(m).unwrap();
    assert!(m.problems().is_empty());
    assert_eq!((m.id.as_str(), m.name.as_str(), m.version.as_str(), m.base_url.as_str()), ("one", "ONE", "1.0.0", one.base.as_str()));
    assert_eq!(m.sections[0].path, "/things");
    assert_eq!(m.deep_link("thing", &[("id", "4")]), Some(format!("{}/things/4", one.base)));
    assert_eq!(m.search.as_deref(), Some(SEARCH_PATH));
    assert_eq!(m.sso.unwrap().issuer, one.base);

    // without a fixed base URL, the request's (behind a proxy, its forwarded host and scheme)
    let s = Suite::new(SuiteConfig::new("x", "X"));
    let mut h = HeaderMap::new();
    h.insert("host", "inside:8080".parse().unwrap());
    assert_eq!(s.manifest(&h).base_url, "http://inside:8080");
    h.insert("x-forwarded-proto", "https".parse().unwrap());
    h.insert("x-forwarded-host", "app.test, inner".parse().unwrap());
    assert_eq!(s.base_url(&h), "https://app.test");
    assert!(s.manifest(&h).sso.is_none());
}

#[tokio::test]
async fn keys_are_shown_once_and_checked() {
    let c = http();
    let one = start("one", |c| c).await;
    let keys = format!("{}/api/kit/suite/keys", one.base);
    assert_eq!(send(req(&c, reqwest::Method::GET, keys.clone(), 2)).await.0, 403);
    assert_eq!(send(req(&c, reqwest::Method::POST, keys.clone(), 1).json(&json!({ "name": " " }))).await.0, 400);
    assert_eq!(send(req(&c, reqwest::Method::POST, keys.clone(), 1).json(&json!({ "name": "x", "redirect_url": "nope" }))).await.0, 400);
    let key = make_key(&c, &one, "Other", None).await;
    let (_, list) = send(req(&c, reqwest::Method::GET, keys.clone(), 1)).await;
    assert_eq!(list["keys"].as_array().unwrap().len(), 1);
    assert!(list["keys"][0].get("key").is_none() && list["keys"][0].get("secret_hash").is_none());
    let id = list["keys"][0]["id"].as_str().unwrap().to_string();

    let ping = format!("{}/api/kit/suite/ping", one.base);
    let (s, v) = send(c.get(&ping).bearer_auth(&key)).await;
    assert_eq!((s, v["id"].as_str()), (200, Some("one")));
    assert_eq!(send(c.get(&ping)).await.0, 401);
    assert_eq!(send(c.get(&ping).bearer_auth(format!("{id}.wrong"))).await.0, 401);
    assert_eq!(send(c.get(&ping).bearer_auth("ks_nothere.x")).await.0, 401);
    let (_, list) = send(req(&c, reqwest::Method::GET, keys.clone(), 1)).await;
    assert!(list["keys"][0]["last_used_at"].as_f64().unwrap() > 0.0);

    assert_eq!(send(req(&c, reqwest::Method::DELETE, format!("{keys}/{id}"), 1)).await.0, 200);
    assert_eq!(send(req(&c, reqwest::Method::DELETE, format!("{keys}/{id}"), 1)).await.0, 404);
    assert_eq!(send(c.get(&ping).bearer_auth(&key)).await.0, 401);
}

#[tokio::test]
async fn links_check_the_other_app_first() {
    let c = http();
    let one = start("one", |c| c.section("things", "Things", "M1 1", "/things")).await;
    let two = start("two", |c| c).await;
    let links = format!("{}/api/kit/suite/links", two.base);
    let post = |b: Value| req(&c, reqwest::Method::POST, links.clone(), 1).json(&b);
    let key = make_key(&c, &one, "Two", None).await;

    let (s, v) = send(post(json!({ "url": "one.test", "key": key }))).await;
    assert_eq!((s, v["detail"].as_str().unwrap()), (400, "Type the other app's address, starting with http:// or https://."));
    assert_eq!(send(post(json!({ "url": one.base, "key": "secret" }))).await.0, 400);
    let (s, v) = send(post(json!({ "url": "http://127.0.0.1:9", "key": key }))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().contains("can't be reached"), "{v}");
    let (s, v) = send(post(json!({ "url": one.base, "key": format!("{}.bad", key.split('.').next().unwrap()) }))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().contains("did not accept the key"), "{v}");
    let self_key = make_key(&c, &two, "Me", None).await;
    let (s, v) = send(post(json!({ "url": two.base, "key": self_key }))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().contains("is this app"), "{v}");
    let (s, v) = send(post(json!({ "url": one.base, "key": key, "account_source": true }))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().contains("not an account source"), "{v}");
    assert_eq!(send(req(&c, reqwest::Method::POST, links.clone(), 2).json(&json!({ "url": one.base, "key": key }))).await.0, 403);

    let (s, v) = send(post(json!({ "url": one.base, "key": key }))).await;
    assert_eq!(s, 200, "{v}");
    let id = v["id"].as_i64().unwrap();
    assert_eq!(v["manifest"]["sections"][0]["id"], "things");
    assert_eq!(send(post(json!({ "url": one.base, "key": key }))).await.0, 409);

    let (_, all) = send(req(&c, reqwest::Method::GET, links.clone(), 1)).await;
    assert_eq!(all["links"].as_array().unwrap().len(), 1);
    assert!(all["links"][0].get("api_key").is_none());
    let (s, v) = send(req(&c, reqwest::Method::POST, format!("{links}/{id}/refresh"), 1)).await;
    assert_eq!((s, v["ok"].as_bool()), (200, Some(true)));
    let (s, _) = send(req(&c, reqwest::Method::PATCH, format!("{links}/{id}"), 1).json(&json!({ "account_source": true }))).await;
    assert_eq!(s, 400);

    // the other app revokes the key: the next check says so and keeps the last manifest
    let kid = key.split('.').next().unwrap();
    send(req(&c, reqwest::Method::DELETE, format!("{}/api/kit/suite/keys/{kid}", one.base), 1)).await;
    let (_, v) = send(req(&c, reqwest::Method::POST, format!("{links}/{id}/refresh"), 1)).await;
    assert_eq!(v["ok"], false);
    assert!(v["error"].as_str().unwrap().contains("did not accept the key"));
    assert_eq!(v["manifest"]["id"], "one");
    let new_key = make_key(&c, &one, "Two again", None).await;
    let (s, v) = send(req(&c, reqwest::Method::PATCH, format!("{links}/{id}"), 1).json(&json!({ "key": new_key }))).await;
    assert_eq!((s, v["ok"].as_bool()), (200, Some(true)), "{v}");

    assert_eq!(send(req(&c, reqwest::Method::DELETE, format!("{links}/{id}"), 1)).await.0, 200);
    assert_eq!(send(req(&c, reqwest::Method::DELETE, format!("{links}/{id}"), 1)).await.0, 404);
    assert_eq!(send(req(&c, reqwest::Method::POST, format!("{links}/{id}/refresh"), 1)).await.0, 404);
}

#[tokio::test]
async fn the_sidebar_shows_every_app_in_each_persons_order() {
    let (c, one, two) = linked().await;
    let url = format!("{}/api/kit/suite/sidebar", two.base);
    assert_eq!(send(req(&c, reqwest::Method::GET, url.clone(), 0)).await.0, 401);
    let (s, v) = send(req(&c, reqwest::Method::GET, url.clone(), 2)).await;
    assert_eq!(s, 200);
    let keys = |v: &Value| v["items"].as_array().unwrap().iter().map(|i| i["key"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(keys(&v), ["two/stuff", "one/things", "one/more"]);
    assert_eq!(v["items"][1]["url"], format!("{}/things", one.base));
    assert_eq!(v["items"][2]["icon"], "M0 0h24", "a section without an icon shows its app's");
    assert_eq!(v["items"][0]["local"], true);
    assert_eq!(v["apps"].as_array().unwrap().len(), 2);

    let (s, v) =
        send(req(&c, reqwest::Method::PUT, url.clone(), 2).json(&json!({ "order": ["one/more", "two/stuff"], "hidden": ["one/things"] })))
            .await;
    assert_eq!(s, 200);
    assert_eq!(keys(&v), ["one/more", "two/stuff", "one/things"]);
    assert_eq!(v["items"][2]["hidden"], true);
    // kept per person, next to their other preferences
    assert_eq!(keys(&send(req(&c, reqwest::Method::GET, url.clone(), 2)).await.1)[0], "one/more");
    assert_eq!(keys(&send(req(&c, reqwest::Method::GET, url.clone(), 3)).await.1)[0], "two/stuff");
    two.app.core.prefs.save_prefs(2, &json!({ "theme": "dark", "suite_sidebar": { "order": ["one/more"] } })).await.unwrap();
    send(req(&c, reqwest::Method::PUT, url.clone(), 2).json(&json!({}))).await;
    assert_eq!(two.app.core.prefs.prefs(2).await.unwrap(), json!({ "theme": "dark", "suite_sidebar": { "order": [], "hidden": [] } }));
    let long = vec!["x"; 1001];
    assert_eq!(send(req(&c, reqwest::Method::PUT, url, 2).json(&json!({ "order": long }))).await.0, 400);
}

#[tokio::test]
async fn one_search_asks_every_app() {
    let (c, one, two) = linked().await;
    let url = |q: &str| format!("{}/api/kit/suite/search?{q}", two.base);
    assert_eq!(send(req(&c, reqwest::Method::GET, url("q=%20"), 2)).await.0, 400);
    assert_eq!(send(req(&c, reqwest::Method::GET, url(&format!("q={}", "a".repeat(201))), 2)).await.0, 400);
    let (s, v) = send(req(&c, reqwest::Method::GET, url("q=cat&limit=2"), 2)).await;
    assert_eq!(s, 200, "{v}");
    let groups = v["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!((groups[0]["app"].as_str(), groups[0]["local"].as_bool()), (Some("two"), Some(true)));
    assert_eq!(groups[0]["results"][0]["url"], format!("{}/things/0", two.base));
    assert_eq!(groups[0]["results"][0]["subtitle"], "ana");
    assert_eq!(groups[1]["app"], "one");
    assert_eq!(groups[1]["results"].as_array().unwrap().len(), 2);
    assert_eq!(groups[1]["results"][1]["url"], format!("{}/things/1", one.base));
    // the other app is asked with its key, for the person by name
    assert_eq!(groups[1]["results"][0]["subtitle"], "Two:ana");

    let (_, v) = send(req(&c, reqwest::Method::GET, url("q=cat&scope=others"), 2)).await;
    assert_eq!(v["groups"].as_array().unwrap().iter().map(|g| g["app"].as_str().unwrap()).collect::<Vec<_>>(), ["one"]);
    let (_, v) = send(req(&c, reqwest::Method::GET, url("q=cat&scope=self"), 2)).await;
    assert_eq!(v["groups"].as_array().unwrap().len(), 1);
    let (_, v) = send(req(&c, reqwest::Method::GET, url("q=cat&apps=one"), 2)).await;
    assert_eq!(v["groups"][0]["app"], "one");
    let (_, v) = send(req(&c, reqwest::Method::GET, url("q=boom"), 2)).await;
    assert_eq!(v["groups"][0]["error"], "The index is broken.");
    assert!(v["groups"][1]["error"].as_str().unwrap().contains("The index is broken."));

    // an app's own search takes its people and keys only
    let own = format!("{}/api/kit/search?q=x", one.base);
    assert_eq!(send(c.get(&own)).await.0, 401);
    assert_eq!(send(c.get(&own).bearer_auth("ks_aaaaaaaaaaaa.nope")).await.0, 401);
    assert_eq!(send(req(&c, reqwest::Method::GET, own, 2)).await.1["results"][0]["subtitle"], "ana");
    let plain = start("plain", |c| c).await;
    assert_eq!(send(req(&c, reqwest::Method::GET, format!("{}/api/kit/search?q=x", plain.base), 2)).await.0, 404);

    // an app that is down is one group's error, not the whole search's
    let mut l = two.app.store.links().await.unwrap().remove(0);
    if let Some(m) = l.manifest.as_mut() {
        m.base_url = "http://127.0.0.1:9".into();
    }
    two.app.store.update_link(&l).await.unwrap();
    let (s, v) = send(req(&c, reqwest::Method::GET, url("q=cat"), 2)).await;
    assert_eq!(s, 200);
    assert!(v["groups"][1]["error"].as_str().unwrap().contains("can't be reached"), "{v}");
}

/// Follow a sign-in from the linked app to the account source and back, as `user` at the source; the
/// callback's answer.
async fn sign_in(c: &reqwest::Client, two: &Running, user: i64) -> reqwest::Response {
    let r = c.get(format!("{}/api/kit/sso/start?next=/stuff/4", two.base)).send().await.unwrap();
    assert_eq!(r.status(), StatusCode::SEE_OTHER);
    let to_source = r.headers()["location"].to_str().unwrap().to_string();
    let r = req(c, reqwest::Method::GET, to_source, user).send().await.unwrap();
    let back = r.headers()["location"].to_str().unwrap().to_string();
    c.get(back).send().await.unwrap()
}

#[tokio::test]
async fn people_sign_in_through_the_account_source() {
    let (c, one, two) = linked().await;
    let (_, info) = send(c.get(format!("{}/api/kit/sso", two.base))).await;
    assert_eq!(info["enabled"], true);
    assert_eq!(info["source"]["name"], "ONE");
    let (_, none) = send(c.get(format!("{}/api/kit/sso", one.base))).await;
    assert_eq!(none["enabled"], false);
    let (_, d) = send(c.get(format!("{}/.well-known/openid-configuration", one.base))).await;
    assert_eq!(d["issuer"], one.base);
    assert_eq!(send(c.get(format!("{}/.well-known/openid-configuration", two.base))).await.0, 404);

    let r = sign_in(&c, &two, 4).await;
    assert_eq!(r.status(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v, json!({ "issuer": one.base, "sub": "4", "username": "boss", "admin": true, "next": "/stuff/4" }));

    // not signed in at the source: its sign-in page first, then back to the authorization
    let r = c.get(format!("{}/api/kit/sso/start", two.base)).send().await.unwrap();
    let to_source = r.headers()["location"].to_str().unwrap().to_string();
    let r = c.get(&to_source).send().await.unwrap();
    let login = r.headers()["location"].to_str().unwrap().to_string();
    assert!(login.starts_with("/?next=%2Fapi%2Fkit%2Foidc%2Fauthorize%3F"), "{login}");

    // a state that was never sent, or used twice, goes back to the sign-in page with the reason
    let r = c.get(format!("{}/api/kit/sso/callback?code=x&state=nope", two.base)).send().await.unwrap();
    assert!(r.headers()["location"].to_str().unwrap().contains("sso_error=The%20sign-in%20took%20too%20long"));
}

#[tokio::test]
async fn the_provider_checks_every_step() {
    let (c, one, two) = linked().await;
    let kid = two.app.store.links().await.unwrap()[0].api_key.split('.').next().unwrap().to_string();
    let auth = |q: &str| req(&c, reqwest::Method::GET, format!("{}/api/kit/oidc/authorize?{q}", one.base), 2);
    let cb = format!("{}/api/kit/sso/callback", two.base);
    let (s, v) = send(auth(&format!("response_type=code&client_id=ks_unknown&redirect_uri={}", kit_rules::suite::encode(&cb)))).await;
    assert_eq!((s, v["detail"].as_str()), (400, Some("The app asking to sign you in is not linked here.")));
    let (s, v) = send(auth(&format!("response_type=code&client_id={kid}&redirect_uri=https%3A%2F%2Fevil.test%2Fcb"))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().starts_with("Sign-in may only return to"));
    let r =
        auth(&format!("response_type=token&client_id={kid}&redirect_uri={}&state=s1", kit_rules::suite::encode(&cb))).send().await.unwrap();
    assert!(r.headers()["location"].to_str().unwrap().ends_with("?error=unsupported_response_type&state=s1"));

    // a code: once, for its own client and redirect only
    let r = auth(&format!("response_type=code&client_id={kid}&redirect_uri={}&state=s2&nonce=n2", kit_rules::suite::encode(&cb)))
        .send()
        .await
        .unwrap();
    let loc = r.headers()["location"].to_str().unwrap().to_string();
    let code = loc.split("code=").nth(1).unwrap().split('&').next().unwrap().to_string();
    assert!(loc.ends_with("&state=s2"));
    let key = two.app.store.links().await.unwrap()[0].api_key.clone();
    let secret = key.split_once('.').unwrap().1.to_string();
    let token = format!("{}/api/kit/oidc/token", one.base);
    let form = |code: &str, redirect: &str, secret: &str| {
        c.post(&token).form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect),
            ("client_id", &kid),
            ("client_secret", secret),
        ])
    };
    let (s, v) = send(form(&code, &cb, "wrong")).await;
    assert_eq!((s, v["error"].as_str()), (401, Some("invalid_client")));
    let (s, v) = send(form(&code, "https://b.test/other", &secret)).await;
    assert_eq!((s, v["error"].as_str()), (400, Some("invalid_grant")));
    // the code was spent by the wrong redirect: a new one, sent the Basic way
    let r =
        auth(&format!("response_type=code&client_id={kid}&redirect_uri={}&nonce=n3", kit_rules::suite::encode(&cb))).send().await.unwrap();
    let code = r.headers()["location"].to_str().unwrap().split("code=").nth(1).unwrap().to_string();
    let (s, v) = send(c.post(&token).basic_auth(&kid, Some(&secret)).form(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", &cb),
    ]))
    .await;
    assert_eq!(s, 200, "{v}");
    let claims = crypto::verify(v["id_token"].as_str().unwrap(), &secret).unwrap();
    assert_eq!((claims["sub"].as_str(), claims["aud"].as_str(), claims["nonce"].as_str()), (Some("2"), Some(kid.as_str()), Some("n3")));
    assert_eq!(claims["iss"], one.base);
    assert_eq!(claims["preferred_username"], "ana");
    let (s, u) = send(c.get(format!("{}/api/kit/oidc/userinfo", one.base)).bearer_auth(v["access_token"].as_str().unwrap())).await;
    assert_eq!((s, u["sub"].as_str()), (200, Some("2")));
    assert_eq!(send(c.get(format!("{}/api/kit/oidc/userinfo", one.base)).bearer_auth("nope")).await.0, 401);
    let (s, v) = send(c.post(&token).basic_auth(&kid, Some(&secret)).form(&[
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", &cb),
    ]))
    .await;
    assert_eq!((s, v["error"].as_str()), (400, Some("invalid_grant")), "a code works once");
    let (s, v) = send(c.post(&token).form(&[("grant_type", "password")])).await;
    assert_eq!((s, v["error"].as_str()), (400, Some("unsupported_grant_type")));

    // a key made without the other app's address can't sign anyone in
    let plain = make_key(&c, &one, "No address", None).await;
    let pid = plain.split('.').next().unwrap();
    let (s, v) = send(auth(&format!("response_type=code&client_id={pid}&redirect_uri={}", kit_rules::suite::encode(&cb)))).await;
    assert_eq!(s, 400);
    assert!(v["detail"].as_str().unwrap().contains("has no address"), "{v}");
}

#[tokio::test]
async fn sign_in_is_refused_when_the_link_or_the_app_says_no() {
    let (c, one, two) = linked().await;
    // a second app links to the same account source and signs people in through it too
    let plain = start("plain", |c| c).await;
    let key = make_key(&c, &one, "Plain", Some(&plain.base)).await;
    let (s, v) = send(
        req(&c, reqwest::Method::POST, format!("{}/api/kit/suite/links", plain.base), 1)
            .json(&json!({ "url": one.base, "key": key, "account_source": true })),
    )
    .await;
    assert_eq!(s, 200, "{v}");
    let r = sign_in(&c, &plain, 2).await;
    assert_eq!(r.json::<Value>().await.unwrap()["username"], "ana");

    // the account source is unlinked: start says so
    let id = two.app.store.links().await.unwrap()[0].id;
    send(req(&c, reqwest::Method::PATCH, format!("{}/api/kit/suite/links/{id}", two.base), 1).json(&json!({ "account_source": false })))
        .await;
    assert_eq!(send(c.get(format!("{}/api/kit/sso/start", two.base))).await.0, 404);
    assert_eq!(send(c.get(format!("{}/api/kit/sso", two.base))).await.1["enabled"], false);
    assert!(two.app.suite.account_source(&two.app.store).await.unwrap().is_none());

    // fetch_json asks the linked app with its key
    let l = two.app.store.links().await.unwrap().remove(0);
    let v = two.app.suite.fetch_json(&l, "/api/kit/suite/ping").await.unwrap();
    assert_eq!(v["id"], "one");
    let e = two.app.suite.fetch_json(&l, "/nothing").await.unwrap_err();
    assert_eq!(e.status, StatusCode::BAD_GATEWAY);
}

#[test]
fn next_paths_stay_in_the_app() {
    assert_eq!(local_path("/stuff?x=1"), "/stuff?x=1");
    for bad in ["//evil.test", "https://evil.test", "/\\evil.test", "stuff", "/a\r\nb"] {
        assert_eq!(local_path(bad), "/");
    }
}
