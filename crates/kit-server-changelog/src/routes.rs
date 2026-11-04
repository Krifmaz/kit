//! The What's new routes, for an app state that implements [`ChangelogHost`], at [`ChangelogPaths`] (the
//! defaults below):
//!
//! - `GET /api/changelog[?limit=N]` (signed in): every release this server includes, newest first, with
//!   the account's popup switch and the version it last saw.
//! - `GET /api/changelog/new` (signed in): the popup — `show` when it is on and something is new.
//! - `POST /api/changelog/seen` (signed in): the popup was shown (or closed); this version is seen.

use crate::notes::{base, parse, unseen, Release};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kit_server_core::{version, Host, PrefsStore, Result, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

/// An app's notes and version, made once at start: from notes built into the server ([`Changelog::new`]) or
/// from a folder of `<MAJOR.MINOR.PATCH>.md` read at start ([`Changelog::from_dir`]).
#[derive(Debug, Clone)]
pub struct Changelog {
    /// this server's version (a hotfix build `1.4.0.17` shares its release's notes)
    pub version: String,
    /// every release with notes, newest first
    pub releases: Vec<Release>,
}

impl Changelog {
    /// The changelog of `app` (the name in each title, "Demo 1.4.0") at `version` from (version, Markdown)
    /// pairs: [`crate::build::embed`]'s `NOTES`, or any list the app makes.
    pub fn new(app: &str, version: &str, notes: &[(&str, &str)]) -> Changelog {
        let mut releases: Vec<Release> = notes.iter().map(|(ver, md)| parse(app, ver, md)).collect();
        releases.sort_by(|a, b| version::compare(&b.version, &a.version));
        Changelog { version: version.to_string(), releases }
    }

    /// The changelog of `app` at `version` from the `<MAJOR.MINOR.PATCH>.md` files in `dir`, read now (other
    /// files are left out, as [`crate::build::notes_in`] says). An error in words when the folder or a file
    /// can't be read.
    pub fn from_dir(app: &str, version: &str, dir: &Path) -> std::result::Result<Changelog, String> {
        let files = crate::build::notes_in(dir).ok_or_else(|| format!("The release notes folder {} can't be read", dir.display()))?;
        let mut notes = Vec::with_capacity(files.len());
        for (v, path) in files {
            let md = std::fs::read_to_string(&path).map_err(|e| format!("The release notes {} can't be read: {e}", path.display()))?;
            notes.push((v, md));
        }
        let pairs: Vec<(&str, &str)> = notes.iter().map(|(v, md)| (v.as_str(), md.as_str())).collect();
        Ok(Changelog::new(app, version, &pairs))
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

    /// Where each account's preferences are kept (`whats_new_seen`, `whats_new_popup`): the kit's
    /// `SqlitePrefs` or `pg::PgPrefs`, or the app's own [`PrefsStore`].
    fn prefs(&self) -> &dyn PrefsStore;
}

/// Where the What's new routes are served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelogPaths {
    /// `GET`: every release (default `/api/changelog`)
    pub all: String,
    /// `GET`: the popup (default `/api/changelog/new`)
    pub new: String,
    /// `POST`: the popup was seen (default `/api/changelog/seen`)
    pub seen: String,
}

impl Default for ChangelogPaths {
    fn default() -> Self {
        ChangelogPaths { all: "/api/changelog".into(), new: "/api/changelog/new".into(), seen: "/api/changelog/seen".into() }
    }
}

/// The What's new routes at their default paths ([`ChangelogPaths::default`]).
pub fn router<S: ChangelogHost>() -> Router<S> {
    router_with(ChangelogPaths::default())
}

/// The What's new routes at `paths`.
pub fn router_with<S: ChangelogHost>(paths: ChangelogPaths) -> Router<S> {
    Router::new().route(&paths.all, get(all::<S>)).route(&paths.new, get(new::<S>)).route(&paths.seen, post(seen::<S>))
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

/// The full What's new list: every release this server includes, newest first.
async fn all<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn, Query(q): Query<AllQuery>) -> Result<Json<Value>> {
    let c = st.changelog();
    let mut list = c.included();
    if let Some(n) = q.limit {
        list.truncate(n);
    }
    let p = st.prefs().prefs(u.id).await?;
    Ok(Json(json!({
        "version": c.version,
        "popup": popup_on(&p),
        "seen": p.get("whats_new_seen").cloned().unwrap_or(Value::Null),
        "releases": list,
    })))
}

/// The popup: `show` when it is on and something is new since this account last saw it.
async fn new<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Result<Json<Value>> {
    let c = st.changelog();
    let p = st.prefs().prefs(u.id).await?;
    let on = popup_on(&p);
    let list = unseen(&c.releases, &c.version, p.get("whats_new_seen").and_then(Value::as_str));
    let list: Vec<Value> = list
        .into_iter()
        .map(|r| json!({ "version": r.version, "title": r.title, "summary": r.summary, "highlights": r.highlights }))
        .collect();
    Ok(Json(json!({ "version": c.version, "popup": on, "show": on && !list.is_empty(), "releases": list })))
}

/// The popup was shown (or closed): this version is seen.
async fn seen<S: ChangelogHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Result<Json<Value>> {
    let version = st.changelog().version.clone();
    let mut pr = st.prefs().prefs(u.id).await?;
    if let Some(o) = pr.as_object_mut() {
        o.insert("whats_new_seen".into(), json!(version));
    }
    st.prefs().save_prefs(u.id, &pr).await?;
    Ok(Json(json!({ "ok": true, "seen": version })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use kit_server_core::testing::TestHost;
    use std::sync::{Arc, OnceLock};
    use tower::ServiceExt;

    /// The test app with notes for 1.2.1, 1.3.0, 1.4.0 and 1.5.0 (written ahead), running 1.4.0.17.
    #[derive(Clone)]
    struct App(TestHost, Arc<Changelog>);

    impl Host for App {
        async fn signed_in(&self, parts: &mut axum::http::request::Parts) -> Result<kit_server_core::Account> {
            self.0.signed_in(parts).await
        }
    }

    impl ChangelogHost for App {
        fn changelog(&self) -> &Changelog {
            &self.1
        }
        fn prefs(&self) -> &dyn PrefsStore {
            &self.0.prefs
        }
    }

    fn app() -> (App, Router) {
        static NOTES: OnceLock<Vec<(String, String)>> = OnceLock::new();
        let notes = NOTES.get_or_init(|| {
            ["1.2.1", "1.5.0", "1.3.0", "1.4.0"]
                .iter()
                .map(|v| (v.to_string(), format!("## Demo {v}\n\nAbout {v}.\n\n- **New {v}.** Text.\n")))
                .collect()
        });
        let pairs: Vec<(&str, &str)> = notes.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
        let st = App(TestHost::new(), Arc::new(Changelog::new("Demo", "1.4.0.17", &pairs)));
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
        // never seen: the newest included release only (1.5.0 is not out yet)
        let (s, v) = call(&r, "GET", "/api/changelog/new", 2).await;
        assert_eq!(s, 200);
        assert_eq!(v["version"], "1.4.0.17");
        assert_eq!((v["popup"].as_bool(), v["show"].as_bool()), (Some(true), Some(true)));
        assert_eq!(v["releases"].as_array().unwrap().len(), 1);
        assert_eq!(
            v["releases"][0],
            json!({ "version": "1.4.0", "title": "Demo 1.4.0", "summary": "About 1.4.0.", "highlights": ["New 1.4.0"] })
        );
        // seen: kept in the account's prefs; the next hotfix shows nothing again
        let (_, v) = call(&r, "POST", "/api/changelog/seen", 2).await;
        assert_eq!(v, json!({ "ok": true, "seen": "1.4.0.17" }));
        assert_eq!(st.0.prefs.prefs(2).await.unwrap(), json!({ "whats_new_seen": "1.4.0.17" }));
        let (_, v) = call(&r, "GET", "/api/changelog/new", 2).await;
        assert_eq!((v["show"].as_bool(), v["releases"].as_array().unwrap().len()), (Some(false), 0));
    }

    #[tokio::test]
    async fn the_switch_turns_the_popup_off_but_not_the_list() {
        let (st, r) = app();
        st.0.prefs.save_prefs(1, &json!({ "whats_new_popup": false, "whats_new_seen": "1.2.1" })).await.unwrap();
        let (_, v) = call(&r, "GET", "/api/changelog/new", 1).await;
        assert_eq!((v["popup"].as_bool(), v["show"].as_bool()), (Some(false), Some(false)));
        assert_eq!(
            v["releases"].as_array().unwrap().iter().map(|x| x["version"].as_str().unwrap()).collect::<Vec<_>>(),
            vec!["1.4.0", "1.3.0"]
        );
        let (_, v) = call(&r, "GET", "/api/changelog", 1).await;
        assert_eq!((v["popup"].as_bool(), v["seen"].as_str()), (Some(false), Some("1.2.1")));
        let list = v["releases"].as_array().unwrap();
        assert_eq!(list.iter().map(|x| x["version"].as_str().unwrap()).collect::<Vec<_>>(), vec!["1.4.0", "1.3.0", "1.2.1"]);
        assert_eq!(list[0]["blocks"][1], json!({ "k": "li", "lead": "New 1.4.0.", "text": "Text." }));
        let (_, v) = call(&r, "GET", "/api/changelog?limit=1", 1).await;
        assert_eq!(v["releases"].as_array().unwrap().len(), 1);
        assert_eq!(v["seen"], json!("1.2.1"));
        let (_, v) = call(&r, "GET", "/api/changelog", 2).await;
        assert_eq!(v["seen"], Value::Null);
    }

    #[tokio::test]
    async fn notes_from_a_folder_at_the_apps_paths() {
        let dir = std::env::temp_dir().join(format!("kit-changelog-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.0.0.md"), "## Demo 1.0.0\n\nFirst.\n").unwrap();
        std::fs::write(dir.join("1.1.0.md"), "## Demo 1.1.0 – Faster\n\nSecond.\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not notes").unwrap();
        let c = Changelog::from_dir("Demo", "1.1.0.3", &dir).unwrap();
        assert_eq!(c.releases.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(), vec!["Faster", "Demo 1.0.0"]);
        let missing = Changelog::from_dir("Demo", "1.0.0", &dir.join("missing")).unwrap_err();
        assert!(missing.starts_with("The release notes folder ") && missing.ends_with(" can't be read"), "{missing}");
        let _ = std::fs::remove_dir_all(&dir);

        let st = App(TestHost::new(), Arc::new(c));
        let paths = ChangelogPaths { all: "/notes".into(), new: "/notes/new".into(), seen: "/notes/seen".into() };
        let r = router_with(paths).with_state(st);
        let (s, v) = call(&r, "GET", "/notes/new", 2).await;
        assert_eq!((s, v["releases"][0]["title"].clone()), (200, json!("Faster")));
        assert_eq!(call(&r, "POST", "/notes/seen", 2).await.1, json!({ "ok": true, "seen": "1.1.0.3" }));
        assert_eq!(call(&r, "GET", "/notes", 2).await.1["seen"], json!("1.1.0.3"));
        let req = Request::builder().uri("/api/changelog").header("x-test-user", "2").body(Body::empty()).unwrap();
        assert_eq!(r.clone().oneshot(req).await.unwrap().status(), 404);
    }

    #[tokio::test]
    async fn signed_out_is_401() {
        let (_, r) = app();
        let (s, v) = call(&r, "GET", "/api/changelog/new", 0).await;
        assert_eq!((s, v), (401, json!({ "detail": "Not authenticated" })));
        assert_eq!(call(&r, "POST", "/api/changelog/seen", 0).await.0, 401);
    }
}
