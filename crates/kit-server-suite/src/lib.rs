//! Suite (contract: `contracts/suite.md`): apps that link to each other, share one sidebar, one sign-in and
//! one search.
//!
//! - **Manifest.** Every app serves `GET /api/kit/manifest` (public): its id, name, icon, version, base URL,
//!   sections with icons, deep-link patterns, its search endpoint and whether it is an account source. All of it
//!   comes from the app's [`SuiteConfig`]; nothing is built in.
//! - **Links.** An admin makes a key in one app (`/api/kit/suite/keys`, shown once) and pastes it with that
//!   app's address into another (`/api/kit/suite/links`). The linking app reads the other's manifest and checks
//!   the key (`/api/kit/suite/ping`). Links and keys live behind [`SuiteStore`] ([`SqliteSuite`], or
//!   `PgSuite` with feature `postgres`) in the kit's `kit_suite_links` and `kit_suite_keys` tables.
//! - **Sidebar.** `GET /api/kit/suite/sidebar` gives every section of this app and the linked apps in the
//!   person's order, hidden ones marked; `PUT` keeps their order and hidden entries in their preferences.
//! - **Single sign-on.** One app is the account source ([`SuiteConfig::account_source`]): an OIDC provider
//!   (`/.well-known/openid-configuration`, `/api/kit/oidc/*`, authorization code flow, HS256 ID tokens signed
//!   with the key's secret). A linked app marks that link as its account source and signs people in through it
//!   (`/api/kit/sso/start` → … → `/api/kit/sso/callback` → [`SuiteHost::sso_sign_in`], where the app makes its
//!   own session).
//! - **Search.** An app that answers [`SuiteHost::search`] is searchable at `GET /api/kit/search` by its own
//!   signed-in people and by apps holding one of its keys. `GET /api/kit/suite/search?q=` asks this app and
//!   every linked app at once and answers one group per app; `scope=others` gives the cross-links ("related in
//!   …").
//!
//! ```ignore
//! let suite = Suite::new(SuiteConfig::new("demo", "Demo").section("things", "Things", ICON, "/things").searchable());
//! impl SuiteHost for App {
//!     fn suite(&self) -> &Suite { &self.suite }
//!     fn suite_store(&self) -> &dyn SuiteStore { &self.links }
//!     fn sidebar_prefs(&self) -> &dyn PrefsStore { &self.prefs }
//! }
//! let app = Router::new().merge(kit_server_suite::router::<App>());
//! ```

mod crypto;
mod oidc;
pub mod remote;
mod routes;
mod sso;
pub mod store;

pub use kit_rules::suite::{Manifest, SearchGroup, SearchHit, Section, SidebarItem, SidebarPrefs, SsoInfo};
#[cfg(feature = "postgres")]
pub use store::PgSuite;
pub use store::{Key, Link, SqliteSuite, SuiteStore};

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum::response::Response;
use axum::Router;
use kit_server_core::{err, Account, Host, PrefsStore, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::sync::{Arc, Mutex};

/// Where the kit serves an app's own search for the suite.
pub const SEARCH_PATH: &str = "/api/kit/search";
/// How long a sign-in may take between leaving for the account source and coming back (seconds).
pub const SSO_WAIT_SECS: f64 = 600.0;
/// How long an authorization code lasts (seconds).
pub const CODE_SECS: f64 = 60.0;
/// How long an access token lasts (seconds).
pub const ACCESS_SECS: f64 = 600.0;
/// How long an ID token lasts (seconds).
pub const ID_TOKEN_SECS: u64 = 300;

/// What the app says about itself in its manifest, and how the suite routes behave.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SuiteConfig {
    /// The app's id (`a-z`, `0-9`, `-`).
    pub id: String,
    /// The app's name, as people see it.
    pub name: String,
    /// The app's icon: SVG path data in a 24×24 box.
    pub icon: String,
    /// The app's version.
    pub version: String,
    /// Where people reach the app (`https://host[/prefix]`); None: from each request's `Host` (and
    /// `X-Forwarded-Proto` / `X-Forwarded-Host` behind a proxy).
    pub base_url: Option<String>,
    /// The app's sections, in its own order.
    pub sections: Vec<Section>,
    /// Deep-link patterns by name (`/things/{id}`).
    pub deep_links: BTreeMap<String, String>,
    /// The search endpoint the manifest names; [`SuiteConfig::searchable`] sets the kit's ([`SEARCH_PATH`]).
    pub search: Option<String>,
    /// The app is an account source: other apps sign people in through it.
    pub account_source: bool,
    /// Where a person who is not signed in is sent to sign in, `{next}` filled with where they were going
    /// (default `/?next={next}`).
    pub login_path: String,
}

impl SuiteConfig {
    /// The config of app `id` called `name`, with no sections yet.
    pub fn new(id: &str, name: &str) -> SuiteConfig {
        SuiteConfig { id: id.into(), name: name.into(), login_path: "/?next={next}".into(), ..SuiteConfig::default() }
    }

    /// With the app's icon (SVG path data).
    pub fn icon(mut self, icon: &str) -> SuiteConfig {
        self.icon = icon.into();
        self
    }

    /// With the app's version.
    pub fn version(mut self, version: &str) -> SuiteConfig {
        self.version = version.into();
        self
    }

    /// With a fixed base URL (else from each request).
    pub fn base_url(mut self, url: &str) -> SuiteConfig {
        self.base_url = kit_rules::suite::normalize_base(url);
        self
    }

    /// With one more section.
    pub fn section(mut self, id: &str, title: &str, icon: &str, path: &str) -> SuiteConfig {
        self.sections.push(Section { id: id.into(), title: title.into(), icon: icon.into(), path: path.into() });
        self
    }

    /// With one more deep-link pattern.
    pub fn deep_link(mut self, name: &str, pattern: &str) -> SuiteConfig {
        self.deep_links.insert(name.into(), pattern.into());
        self
    }

    /// Searchable through the kit's endpoint ([`SEARCH_PATH`], answered by [`SuiteHost::search`]).
    pub fn searchable(mut self) -> SuiteConfig {
        self.search = Some(SEARCH_PATH.into());
        self
    }

    /// An account source: serves OIDC sign-in to the apps it made keys for.
    pub fn account_source(mut self, on: bool) -> SuiteConfig {
        self.account_source = on;
        self
    }

    /// With the app's sign-in page (`{next}` is where the person was going).
    pub fn login_path(mut self, path: &str) -> SuiteConfig {
        self.login_path = path.into();
        self
    }
}

/// Who calls an app's search: one of its own signed-in people, or a linked app with one of its keys.
#[derive(Debug, Clone, PartialEq)]
pub enum Caller {
    /// A person signed in to this app.
    Account(Account),
    /// Another app, with a key this app made.
    App {
        /// The key's id.
        key: String,
        /// The key's name.
        name: String,
        /// The person the other app searches for (their user name there), a hint this app may use or ignore.
        user: Option<String>,
    },
}

/// A person the account source vouched for, handed to [`SuiteHost::sso_sign_in`].
#[derive(Debug, Clone, PartialEq)]
pub struct SsoUser {
    /// The account source's issuer (its base URL).
    pub issuer: String,
    /// Their id there (stable; the key to match an account here on, with `issuer`).
    pub subject: String,
    /// Their user name there.
    pub username: String,
    /// Their display name there.
    pub name: String,
    /// Their e-mail address there, when it has one.
    pub email: Option<String>,
    /// They are an admin there.
    pub admin: bool,
    /// They own the account source.
    pub owner: bool,
    /// Every claim of the ID token, for anything else the app reads.
    pub claims: Value,
}

/// What the suite routes need from the app.
pub trait SuiteHost: Host {
    /// The app's suite.
    fn suite(&self) -> &Suite;

    /// Where links and keys are kept.
    fn suite_store(&self) -> &dyn SuiteStore;

    /// Where each person's sidebar choices are kept (under [`kit_rules::suite::PREFS_KEY`]).
    fn sidebar_prefs(&self) -> &dyn PrefsStore;

    /// The app's own search for `q`, at most `limit` hits, each `url` a path in this app or a full URL. Only
    /// what `caller` may see. None by default.
    fn search(&self, caller: Caller, q: String, limit: usize) -> impl Future<Output = Result<Vec<SearchHit>>> + Send {
        let _ = (caller, q, limit);
        async { Ok(vec![]) }
    }

    /// Sign in the person the account source vouched for: find (or make) the account for `user.issuer` +
    /// `user.subject`, start the app's own session and answer a redirect to `next` (a path in this app) with
    /// its cookie. 501 by default.
    fn sso_sign_in(&self, user: SsoUser, next: String) -> impl Future<Output = Result<Response>> + Send {
        let _ = (user, next);
        async { Err(err(501, "This app can't sign people in through another app yet.")) }
    }

    /// The claims an account source puts in a signed-in person's ID token next to `sub`: by default
    /// `preferred_username`, `name`, `kit_admin` and `kit_owner`. An app adds `email` and anything else here.
    fn oidc_claims(&self, account: Account) -> impl Future<Output = Result<Value>> + Send {
        async move {
            Ok(json!({
                "preferred_username": account.username,
                "name": account.username,
                "kit_admin": account.admin,
                "kit_owner": account.owner,
            }))
        }
    }
}

/// A sign-in on its way to the account source (this app is the relying party).
#[derive(Debug, Clone)]
struct Pending {
    /// the nonce the ID token must carry
    nonce: String,
    /// where to go after
    next: String,
    /// the link it goes through
    link: i64,
    /// when it started
    at: f64,
}

/// An authorization code (this app is the account source).
#[derive(Debug, Clone)]
struct Grant {
    /// the key it was issued to
    client_id: String,
    /// where it was sent
    redirect_uri: String,
    /// who signed in
    account: Account,
    /// the relying party's nonce
    nonce: Option<String>,
    /// when it was issued
    at: f64,
}

/// What the suite keeps in memory only: sign-ins in flight, codes and access tokens. A restart forgets them,
/// so a sign-in under way starts again.
#[derive(Debug, Default)]
struct Memory {
    /// state → pending sign-in
    pending: HashMap<String, Pending>,
    /// code → grant
    codes: HashMap<String, Grant>,
    /// access token → (account, client id, until)
    access: HashMap<String, (Account, String, f64)>,
}

impl Memory {
    /// Forget everything that has run out.
    fn prune(&mut self, now: f64) {
        self.pending.retain(|_, p| now - p.at < SSO_WAIT_SECS);
        self.codes.retain(|_, g| now - g.at < CODE_SECS);
        self.access.retain(|_, a| a.2 > now);
    }
}

/// An app's suite: its config, the HTTP client it asks linked apps with, and what it keeps in memory. Made
/// once at start; clones share everything.
#[derive(Clone)]
pub struct Suite {
    /// what the app says about itself
    config: Arc<SuiteConfig>,
    /// the client linked apps are asked with
    http: reqwest::Client,
    /// sign-ins in flight, codes, access tokens
    memory: Arc<Mutex<Memory>>,
}

impl std::fmt::Debug for Suite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Suite").field("config", &self.config).finish_non_exhaustive()
    }
}

impl Suite {
    /// The suite of the app `config` describes.
    pub fn new(config: SuiteConfig) -> Suite {
        Suite::with_http(config, remote::client())
    }

    /// The same, asking linked apps with the app's own HTTP client.
    pub fn with_http(config: SuiteConfig, http: reqwest::Client) -> Suite {
        Suite { config: Arc::new(config), http, memory: Arc::default() }
    }

    /// What the app says about itself.
    pub fn config(&self) -> &SuiteConfig {
        &self.config
    }

    /// The HTTP client linked apps are asked with.
    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// Where people reach the app: the configured base URL, else the one this request came to.
    pub fn base_url(&self, headers: &HeaderMap) -> String {
        if let Some(b) = &self.config.base_url {
            return b.clone();
        }
        let h = |k: &str| headers.get(k).and_then(|v| v.to_str().ok()).map(|v| v.split(',').next().unwrap_or("").trim().to_string());
        let proto = h("x-forwarded-proto").filter(|p| p == "https" || p == "http").unwrap_or_else(|| "http".into());
        let host = h("x-forwarded-host").filter(|v| !v.is_empty()).or_else(|| h("host")).unwrap_or_else(|| "localhost".into());
        format!("{proto}://{host}")
    }

    /// The app's manifest as served to a request with `headers`.
    pub fn manifest(&self, headers: &HeaderMap) -> Manifest {
        let c = &self.config;
        let base = self.base_url(headers);
        Manifest {
            kit_suite: kit_rules::suite::MANIFEST_VERSION,
            id: c.id.clone(),
            name: c.name.clone(),
            icon: c.icon.clone(),
            version: c.version.clone(),
            sections: c.sections.clone(),
            deep_links: c.deep_links.clone(),
            search: c.search.clone(),
            sso: c.account_source.then(|| SsoInfo { issuer: base.clone() }),
            base_url: base,
        }
    }

    /// The link people sign in through, if one is marked as the account source.
    pub async fn account_source(&self, store: &dyn SuiteStore) -> Result<Option<Link>> {
        Ok(store.links().await?.into_iter().find(|l| l.account_source))
    }

    /// GET `path` of the linked app with the link's key: its JSON answer; 502 with the reason otherwise.
    pub async fn fetch_json(&self, link: &Link, path: &str) -> Result<Value> {
        remote::get(&self.http, &link.base_url, path, &link.api_key).await.map_err(|e| err(502, e))
    }

    /// Read the linked app's manifest again and check its key; the link as kept afterwards (with the reason in
    /// `error` when the app could not be read).
    pub async fn refresh(&self, store: &dyn SuiteStore, mut link: Link) -> Result<Link> {
        match self.check(&link.base_url, &link.api_key).await {
            Ok(m) => {
                link.manifest = Some(m);
                link.error = None;
            }
            Err(e) => link.error = Some(e),
        }
        link.checked_at = kit_server_core::db::now();
        store.update_link(&link).await?;
        Ok(link)
    }

    /// The manifest of the app at `base`, after checking it takes `key` and is not this app.
    async fn check(&self, base: &str, key: &str) -> std::result::Result<Manifest, String> {
        let m = remote::manifest(&self.http, base).await?;
        if m.id == self.config.id {
            return Err(format!("{base} is this app (both are \u{201c}{}\u{201d}). Link a different app.", m.id));
        }
        remote::ping(&self.http, base, key).await?;
        Ok(m)
    }

    /// The memory, even after a panic in another thread held it.
    fn mem(&self) -> std::sync::MutexGuard<'_, Memory> {
        let mut m = self.memory.lock().unwrap_or_else(|p| p.into_inner());
        m.prune(kit_server_core::db::now());
        m
    }

    /// The key `bearer` names, when this app made it and it is right; it is marked used.
    pub async fn check_key(&self, store: &dyn SuiteStore, bearer: &str) -> Result<Key> {
        let bad = || err(401, "The key is not one this app made, or it was revoked.");
        let (id, secret) = crypto::split_key(bearer).ok_or_else(bad)?;
        let key = store.key(id).await?.ok_or_else(bad)?;
        if !crypto::same(&crypto::hash(secret), &key.secret_hash) {
            return Err(bad());
        }
        let now = kit_server_core::db::now();
        if now - key.last_used_at > 60.0 {
            store.touch_key(&key.id, now).await?;
        }
        Ok(key)
    }
}

/// Refresh every link's manifest every `every` (the sidebar shows what the linked apps say now), from a task
/// of its own.
pub fn spawn_refresh<S: SuiteHost>(host: S, every: std::time::Duration) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(every);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let Ok(links) = host.suite_store().links().await else { continue };
            for l in links {
                if let Err(e) = host.suite().refresh(host.suite_store(), l).await {
                    tracing::warn!("suite: a link can't be refreshed: {e}");
                }
            }
        }
    });
}

/// The `Authorization: Bearer` token of a request.
pub(crate) fn bearer(headers: &HeaderMap) -> Option<String> {
    let v = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?.trim();
    let (scheme, token) = v.split_once(' ')?;
    (scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty()).then(|| token.trim().to_string())
}

/// A caller of the app's search: a linked app with one of this app's keys, or a signed-in person.
#[derive(Debug, Clone)]
pub struct SuiteCaller(pub Caller);

impl<S: SuiteHost> FromRequestParts<S> for SuiteCaller {
    type Rejection = kit_server_core::Error;
    async fn from_request_parts(parts: &mut Parts, st: &S) -> Result<Self> {
        if let Some(b) = bearer(&parts.headers).filter(|b| crypto::split_key(b).is_some()) {
            let key = st.suite().check_key(st.suite_store(), &b).await?;
            let user = parts.headers.get(remote::USER_HEADER).and_then(|v| v.to_str().ok()).map(str::to_string).filter(|u| !u.is_empty());
            return Ok(SuiteCaller(Caller::App { key: key.id, name: key.name, user }));
        }
        st.signed_in(parts).await.map(|a| SuiteCaller(Caller::Account(a)))
    }
}

/// Every suite route: the manifest, key check, search, links, keys, sidebar, suite search, sign-on (both
/// sides). The paths are fixed (other apps find them by them); an app under a prefix says so in its base URL.
pub fn router<S: SuiteHost>() -> Router<S> {
    routes::router::<S>().merge(oidc::router::<S>()).merge(sso::router::<S>())
}

/// A path in this app to go to after signing in: `next` when it is one, `/` otherwise.
pub(crate) fn local_path(next: &str) -> String {
    let ok = next.starts_with('/') && !next.starts_with("//") && !next.starts_with("/\\") && !next.contains(['\r', '\n']);
    if ok {
        next.to_string()
    } else {
        "/".into()
    }
}

#[cfg(test)]
mod tests;
