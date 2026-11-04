//! The What's new routes, for an app state that implements [`ChangelogHost`]:
//!
//! - `GET /api/changelog[?limit=N]` (signed in): every release this server includes, newest first, with
//!   the account's popup switch and the version it last saw.
//! - `GET /api/changelog/new` (signed in): the popup — `show` when it is on and something is new.
//! - `POST /api/changelog/seen` (signed in): the popup was shown (or closed); this version is seen.

use crate::notes::{base, parse, unseen, Release};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kit_server_core::{p, version, Account, Host, Result, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};

/// An app's notes and version, made once at start.
#[derive(Debug, Clone)]
pub struct Changelog {
    /// this server's version (a hotfix build `0.36.0.17` shares its release's notes)
    pub version: String,
    /// every release with notes, newest first
    pub releases: Vec<Release>,
}

impl Changelog {
    /// The changelog of `app` at `version` from its embedded notes ([`crate::build::embed`]'s `NOTES`).
    pub fn new(app: &str, version: &str, notes: &[(&str, &str)]) -> Changelog {
        let mut releases: Vec<Release> = notes.iter().map(|(ver, md)| parse(app, ver, md)).collect();
        releases.sort_by(|a, b| version::compare(&b.version, &a.version));
        Changelog { version: version.to_string(), releases }
    }

    /// The releases this server includes (none written ahead for a later version), newest first.
    pub fn included(&self) -> Vec<&Release> {
        let cur = base(&self.version);
        self.releases.iter().filter(|r| version::compare(&r.version, &cur) != std::cmp::Ordering::Greater).collect()
    }
}

/// What the routes need from the app: its changelog, and where an account's preferences live.
pub trait ChangelogHost: Host {
    /// The app's changelog.
    fn changelog(&self) -> &Changelog;

    /// The account's preferences (`whats_new_seen`, `whats_new_popup`): by default the JSON text in the
    /// `prefs` column of its row, `{}` when there is none.
    fn prefs(&self, account: &Account) -> Value {
        account.row.get("prefs").and_then(Value::as_str).and_then(|s| serde_json::from_str(s).ok()).unwrap_or(json!({}))
    }

    /// Keep the account's preferences: by default `UPDATE users SET prefs=? WHERE id=?` (Encore's table).
    fn save_prefs(&self, account: &Account, prefs: &Value) -> Result<()> {
        self.db().execute("UPDATE users SET prefs=? WHERE id=?", p![prefs.to_string(), account.id])?;
        Ok(())
    }
}

/// The What's new routes.
pub fn router<S: ChangelogHost>() -> Router<S> {
    Router::new()
        .route("/api/changelog", get(all::<S>))
        .route("/api/changelog/new", get(new::<S>))
        .route("/api/changelog/seen", post(seen::<S>))
}

/// The popup is on unless the person switched it off.
fn popup_on(prefs: &Value) -> bool {
    prefs.get("whats_new_popup").and_then(Value::as_bool).unwrap_or(true)
}

/// `GET /api/changelog` query.
#[derive(Deserialize)]
struct AllQuery {
    /// how many releases (default: all)
    #[serde(default)]
    limit: Option<usize>,
}

/// Settings → What's new: every release this server includes, newest first.
async fn all<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn, Query(q): Query<AllQuery>) -> Json<Value> {
    let c = st.changelog();
    let mut list = c.included();
    if let Some(n) = q.limit {
        list.truncate(n);
    }
    let p = st.prefs(&u);
    Json(json!({
        "version": c.version,
        "popup": popup_on(&p),
        "seen": p.get("whats_new_seen").cloned().unwrap_or(Value::Null),
        "releases": list,
    }))
}

/// The popup: `show` when it is on and something is new since this account last saw it.
async fn new<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Json<Value> {
    let c = st.changelog();
    let p = st.prefs(&u);
    let on = popup_on(&p);
    let list = unseen(&c.releases, &c.version, p.get("whats_new_seen").and_then(Value::as_str));
    let list: Vec<Value> = list
        .into_iter()
        .map(|r| json!({ "version": r.version, "title": r.title, "summary": r.summary, "highlights": r.highlights }))
        .collect();
    Json(json!({ "version": c.version, "popup": on, "show": on && !list.is_empty(), "releases": list }))
}

/// The popup was shown (or closed): this version is seen.
async fn seen<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Result<Json<Value>> {
    let version = st.changelog().version.clone();
    let mut pr = st.prefs(&u);
    if let Some(o) = pr.as_object_mut() {
        o.insert("whats_new_seen".into(), json!(version));
    }
    st.save_prefs(&u, &pr)?;
    Ok(Json(json!({ "ok": true, "seen": version })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use kit_server_core::testing::TestHost;
    use kit_server_core::Db;
    use std::sync::{Arc, OnceLock};
    use tower::ServiceExt;

    /// The test app with notes for 0.35.2, 0.36.0, 0.36.1 and 0.37.0 (written ahead), running 0.36.1.17.
    #[derive(Clone)]
    struct App(TestHost, Arc<Changelog>);

    impl Host for App {
        fn db(&self) -> &dyn Db {
            self.0.db()
        }
        async fn signed_in(&self, parts: &mut axum::http::request::Parts) -> Result<Account> {
            self.0.signed_in(parts).await
        }
    }

    impl ChangelogHost for App {
        fn changelog(&self) -> &Changelog {
            &self.1
        }
    }

    fn app() -> (App, Router) {
        static NOTES: OnceLock<Vec<(String, String)>> = OnceLock::new();
        let notes = NOTES.get_or_init(|| {
            ["0.35.2", "0.37.0", "0.36.0", "0.36.1"]
                .iter()
                .map(|v| (v.to_string(), format!("## Encore {v}\n\nAbout {v}.\n\n- **New {v}.** Text.\n")))
                .collect()
        });
        let pairs: Vec<(&str, &str)> = notes.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let st = App(TestHost::new(), Arc::new(Changelog::new("Encore", "0.36.1.17", &pairs)));
        (st.clone(), router().with_state(st))
    }

    async fn call(r: &Router, method: &str, uri: &str, user: i64) -> (u16, Value) {
        let req = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string()).body(Body::empty()).unwrap();
        let res = r.clone().oneshot(req).await.unwrap();
        let status = res.status().as_u16();
        let b = axum::body::to_bytes(res.into_body(), 1 << 20).await.unwrap();
        (status, serde_json::from_slice(&b).unwrap())
    }

    #[tokio::test]
    async fn the_popup_shows_once_per_release() {
        let (st, r) = app();
        // never seen: the newest included release only (0.37.0 is not out yet)
        let (s, v) = call(&r, "GET", "/api/changelog/new", 2).await;
        assert_eq!(s, 200);
        assert_eq!(v["version"], "0.36.1.17");
        assert_eq!((v["popup"].as_bool(), v["show"].as_bool()), (Some(true), Some(true)));
        assert_eq!(v["releases"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["releases"][0],
            json!({ "version": "0.36.1", "title": "Encore 0.36.1", "summary": "About 0.36.1.", "highlights": ["New 0.36.1"] })
        );
        // seen: kept in the account's prefs; the next hotfix shows nothing again
        let (_, v) = call(&r, "POST", "/api/changelog/seen", 2).await;
        assert_eq!(v, json!({ "ok": true, "seen": "0.36.1.17" }));
        let prefs = st.0.db.one("SELECT prefs FROM users WHERE id=2", p![]).unwrap().unwrap();
        assert_eq!(prefs["prefs"], "{\"whats_new_seen\":\"0.36.1.17\"}");
        let (_, v) = call(&r, "GET", "/api/changelog/new", 2).await;
        assert_eq!((v["show"].as_bool(), v["releases"].as_array().unwrap().len()), (Some(false), 0));
    }

    #[tokio::test]
    async fn the_switch_turns_the_popup_off_but_not_the_list() {
        let (st, r) = app();
        st.0.db
            .execute("UPDATE users SET prefs=? WHERE id=1", p![json!({ "whats_new_popup": false, "whats_new_seen": "0.35.2" }).to_string()])
            .unwrap();
        let (_, v) = call(&r, "GET", "/api/changelog/new", 1).await;
        assert_eq!((v["popup"].as_bool(), v["show"].as_bool()), (Some(false), Some(false)));
        assert_eq!(
            v["releases"].as_array().unwrap().iter().map(|x| x["version"].as_str().unwrap()).collect::<Vec<_>>(),
            vec!["0.36.1", "0.36.0"]
        );
        let (_, v) = call(&r, "GET", "/api/changelog", 1).await;
        assert_eq!((v["popup"].as_bool(), v["seen"].as_str()), (Some(false), Some("0.35.2")));
        let list = v["releases"].as_array().unwrap();
        assert_eq!(list.iter().map(|x| x["version"].as_str().unwrap()).collect::<Vec<_>>(), vec!["0.36.1", "0.36.0", "0.35.2"]);
        assert_eq!(list[0]["blocks"][1], json!({ "k": "li", "lead": "New 0.36.1.", "text": "Text." }));
        let (_, v) = call(&r, "GET", "/api/changelog?limit=1", 1).await;
        assert_eq!(v["releases"].as_array().unwrap().len(), 1);
        assert_eq!(v["seen"], json!("0.35.2"));
        let (_, v) = call(&r, "GET", "/api/changelog", 2).await;
        assert_eq!(v["seen"], Value::Null);
    }

    #[tokio::test]
    async fn signed_out_is_401() {
        let (_, r) = app();
        let (s, v) = call(&r, "GET", "/api/changelog/new", 0).await;
        assert_eq!((s, v), (401, json!({ "detail": "Not authenticated" })));
        assert_eq!(call(&r, "POST", "/api/changelog/seen", 0).await.0, 401);
    }
}
