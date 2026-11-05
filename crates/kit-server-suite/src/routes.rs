//! The manifest, key check, app search, links, keys, sidebar and suite search routes.

use crate::store::{Key, Link};
use crate::{crypto, remote, Caller, SuiteCaller, SuiteHost};
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use kit_rules::suite::{normalize_base, sidebar, Manifest, SearchGroup, SidebarPrefs, PREFS_KEY};
use kit_server_core::{err, Admin, Result, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};

/// The most entries a person's sidebar choices may name.
const MAX_PREF_KEYS: usize = 1000;
/// The longest search text.
const MAX_QUERY: usize = 200;

/// The routes of this module.
pub(crate) fn router<S: SuiteHost>() -> Router<S> {
    Router::new()
        .route(remote::MANIFEST_PATH, get(manifest::<S>))
        .route(remote::PING_PATH, get(ping::<S>))
        .route(crate::SEARCH_PATH, get(app_search::<S>))
        .route("/api/kit/suite/links", get(links::<S>).post(add_link::<S>))
        .route("/api/kit/suite/links/{id}", delete(remove_link::<S>).patch(patch_link::<S>))
        .route("/api/kit/suite/links/{id}/refresh", post(refresh_link::<S>))
        .route("/api/kit/suite/keys", get(keys::<S>).post(add_key::<S>))
        .route("/api/kit/suite/keys/{id}", delete(remove_key::<S>))
        .route("/api/kit/suite/sidebar", get(get_sidebar::<S>).put(put_sidebar::<S>))
        .route("/api/kit/suite/search", get(suite_search::<S>))
}

/// `GET /api/kit/manifest` (public): what this app says about itself.
async fn manifest<S: SuiteHost>(State(st): State<S>, headers: HeaderMap) -> Json<Manifest> {
    Json(st.suite().manifest(&headers))
}

/// `GET /api/kit/suite/ping` (a key this app made): `{id, name}` of this app, so the linking app knows the key
/// works.
async fn ping<S: SuiteHost>(State(st): State<S>, headers: HeaderMap) -> Result<Json<Value>> {
    let b = crate::bearer(&headers).ok_or_else(|| err(401, "No key was sent. Send it as Authorization: Bearer <key>."))?;
    let key = st.suite().check_key(st.suite_store(), &b).await?;
    let c = st.suite().config();
    Ok(Json(json!({ "id": c.id, "name": c.name, "key": key.id })))
}

/// The query of a search.
#[derive(Debug, Deserialize)]
struct SearchQuery {
    /// what to look for
    #[serde(default)]
    q: String,
    /// the most hits per app
    limit: Option<usize>,
    /// `all` (default), `self` or `others`
    scope: Option<String>,
    /// only these app ids, comma-separated
    apps: Option<String>,
}

/// The search text, trimmed: 400 when empty or too long.
fn query_text(q: &str) -> Result<String> {
    let q = q.trim();
    if q.is_empty() {
        return Err(err(400, "Type something to search for."));
    }
    if q.chars().count() > MAX_QUERY {
        return Err(err(400, format!("Search for at most {MAX_QUERY} characters.")));
    }
    Ok(q.to_string())
}

/// `GET /api/kit/search?q=&limit=` (signed in, or a linked app with a key): this app's own search,
/// `{"results": [<hit>…]}`. 404 when the app is not searchable.
async fn app_search<S: SuiteHost>(
    State(st): State<S>,
    SuiteCaller(caller): SuiteCaller,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    if st.suite().config().search.is_none() {
        return Err(err(404, "This app has no search for other apps."));
    }
    let text = query_text(&q.q)?;
    let limit = q.limit.unwrap_or(10).clamp(1, 50);
    let mut hits = st.search(caller, text, limit).await?;
    hits.truncate(limit);
    Ok(Json(json!({ "results": hits })))
}

/// `GET /api/kit/suite/links` (admin): every link, without keys.
async fn links<S: SuiteHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let all = st.suite_store().links().await?;
    Ok(Json(json!({ "links": all.iter().map(Link::public).collect::<Vec<_>>() })))
}

/// The body of a new link.
#[derive(Debug, Deserialize)]
struct NewLink {
    /// the other app's address
    #[serde(default)]
    url: String,
    /// the key the other app made for this one
    #[serde(default)]
    key: String,
    /// people sign in here through it
    #[serde(default)]
    account_source: bool,
}

/// 400 unless `m` offers sign-in.
fn must_offer_sso(m: &Manifest) -> Result<()> {
    if m.sso.is_none() {
        return Err(err(400, format!("{} is not an account source. Turn on sign-in for other apps there first.", m.name)));
    }
    Ok(())
}

/// `POST /api/kit/suite/links` (admin) `{url, key, account_source}`: read the other app's manifest, check the
/// key, keep the link; the link. 400 with the reason when the app can't be linked.
async fn add_link<S: SuiteHost>(State(st): State<S>, Admin(_): Admin, Json(b): Json<NewLink>) -> Result<Json<Value>> {
    let base = normalize_base(&b.url).ok_or_else(|| err(400, "Type the other app's address, starting with http:// or https://."))?;
    let key = b.key.trim().to_string();
    if crypto::split_key(&key).is_none() {
        return Err(err(400, "Paste the key the other app made for this one (it starts with ks_)."));
    }
    let store = st.suite_store();
    let existing = store.links().await?;
    if existing.iter().any(|l| l.base_url == base) {
        return Err(err(409, format!("{base} is linked already.")));
    }
    let m = st.suite().check(&base, &key).await.map_err(|e| err(400, e))?;
    if let Some(l) = existing.iter().find(|l| l.manifest.as_ref().is_some_and(|x| x.id == m.id)) {
        return Err(err(409, format!("{} is linked already, at {}.", m.name, l.base_url)));
    }
    if b.account_source {
        must_offer_sso(&m)?;
    }
    let now = kit_server_core::db::now();
    let mut link = Link {
        id: 0,
        base_url: base,
        api_key: key,
        account_source: false,
        manifest: Some(m),
        error: None,
        checked_at: now,
        created_at: now,
    };
    link.id = store.add_link(&link).await?;
    if b.account_source {
        store.set_account_source(Some(link.id)).await?;
        link.account_source = true;
    }
    Ok(Json(link.public()))
}

/// Link `id`, or 404.
async fn link_or_404<S: SuiteHost>(st: &S, id: i64) -> Result<Link> {
    st.suite_store().link(id).await?.ok_or_else(|| err(404, "That link is gone."))
}

/// The body of a link change.
#[derive(Debug, Deserialize)]
struct LinkPatch {
    /// make it (or stop it being) the account source
    account_source: Option<bool>,
    /// a new key from the other app
    key: Option<String>,
}

/// `PATCH /api/kit/suite/links/{id}` (admin) `{account_source?, key?}`: make the link the account source (or
/// not), or give it a new key (checked first); the link.
async fn patch_link<S: SuiteHost>(
    State(st): State<S>,
    Admin(_): Admin,
    Path(id): Path<i64>,
    Json(b): Json<LinkPatch>,
) -> Result<Json<Value>> {
    let store = st.suite_store();
    let mut link = link_or_404(&st, id).await?;
    if let Some(key) = b.key.map(|k| k.trim().to_string()) {
        if crypto::split_key(&key).is_none() {
            return Err(err(400, "Paste the key the other app made for this one (it starts with ks_)."));
        }
        let m = st.suite().check(&link.base_url, &key).await.map_err(|e| err(400, e))?;
        link.api_key = key;
        link.manifest = Some(m);
        link.error = None;
        link.checked_at = kit_server_core::db::now();
        store.update_link(&link).await?;
    }
    match b.account_source {
        Some(true) => {
            let m = link.manifest.clone().unwrap_or_default();
            must_offer_sso(&Manifest { name: link.name(), ..m })?;
            store.set_account_source(Some(id)).await?;
        }
        Some(false) if link.account_source => store.set_account_source(None).await?,
        _ => {}
    }
    Ok(Json(link_or_404(&st, id).await?.public()))
}

/// `POST /api/kit/suite/links/{id}/refresh` (admin): read the other app's manifest again; the link (its
/// `error` says why when it failed).
async fn refresh_link<S: SuiteHost>(State(st): State<S>, Admin(_): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    let link = link_or_404(&st, id).await?;
    Ok(Json(st.suite().refresh(st.suite_store(), link).await?.public()))
}

/// `DELETE /api/kit/suite/links/{id}` (admin): remove the link.
async fn remove_link<S: SuiteHost>(State(st): State<S>, Admin(_): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    if !st.suite_store().remove_link(id).await? {
        return Err(err(404, "That link is gone."));
    }
    Ok(Json(json!({ "ok": true })))
}

/// `GET /api/kit/suite/keys` (admin): every key this app made, without secrets.
async fn keys<S: SuiteHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let all = st.suite_store().keys().await?;
    Ok(Json(json!({ "keys": all.iter().map(Key::public).collect::<Vec<_>>() })))
}

/// The body of a new key.
#[derive(Debug, Deserialize)]
struct NewKey {
    /// what to call it
    #[serde(default)]
    name: String,
    /// the other app's base URL, for sign-in through this app
    redirect_url: Option<String>,
}

/// `POST /api/kit/suite/keys` (admin) `{name, redirect_url?}`: a new key; the answer has the whole key in
/// `key`, the only time it is shown.
async fn add_key<S: SuiteHost>(State(st): State<S>, Admin(_): Admin, Json(b): Json<NewKey>) -> Result<Json<Value>> {
    let name = b.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(err(400, "Give the key a name of 1 to 100 characters (the other app's name, say)."));
    }
    let redirect_url = match b.redirect_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) => Some(normalize_base(u).ok_or_else(|| err(400, "The other app's address must start with http:// or https://."))?),
        None => None,
    };
    let (id, whole, secret_hash) = crypto::new_key();
    let key = Key { id, name: name.into(), secret_hash, redirect_url, created_at: kit_server_core::db::now(), last_used_at: 0.0 };
    st.suite_store().add_key(&key).await?;
    let mut out = key.public();
    out["key"] = json!(whole);
    Ok(Json(out))
}

/// `DELETE /api/kit/suite/keys/{id}` (admin): revoke the key.
async fn remove_key<S: SuiteHost>(State(st): State<S>, Admin(_): Admin, Path(id): Path<String>) -> Result<Json<Value>> {
    if !st.suite_store().remove_key(&id).await? {
        return Err(err(404, "That key is gone."));
    }
    Ok(Json(json!({ "ok": true })))
}

/// The person's sidebar choices from their preferences.
async fn prefs_of<S: SuiteHost>(st: &S, account: i64) -> Result<SidebarPrefs> {
    let all = st.sidebar_prefs().prefs(account).await?;
    Ok(all.get(PREFS_KEY).cloned().and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default())
}

/// The sidebar answer: every entry in the person's order, and the apps they come from.
async fn sidebar_answer<S: SuiteHost>(st: &S, headers: &HeaderMap, prefs: &SidebarPrefs) -> Result<Value> {
    let me = st.suite().manifest(headers);
    let links = st.suite_store().links().await?;
    let mut apps = vec![json!({ "id": me.id, "name": me.name, "icon": me.icon, "base_url": me.base_url, "local": true, "error": null })];
    let mut manifests = vec![me.clone()];
    for l in &links {
        if let Some(m) = &l.manifest {
            apps.push(json!({ "id": m.id, "name": m.name, "icon": m.icon, "base_url": m.base_url, "local": false, "error": l.error }));
            manifests.push(m.clone());
        }
    }
    Ok(json!({ "items": sidebar(&manifests, &me.id, prefs), "apps": apps }))
}

/// `GET /api/kit/suite/sidebar` (signed in): `{items, apps}`: every section of this app and the linked apps
/// in the person's order (hidden ones marked), and the apps.
async fn get_sidebar<S: SuiteHost>(State(st): State<S>, SignedIn(u): SignedIn, headers: HeaderMap) -> Result<Json<Value>> {
    let prefs = prefs_of(&st, u.id).await?;
    Ok(Json(sidebar_answer(&st, &headers, &prefs).await?))
}

/// `PUT /api/kit/suite/sidebar` (signed in) `{order, hidden}`: keep the person's sidebar choices; answered as
/// `GET`. An empty object puts the sidebar back in the apps' own order.
async fn put_sidebar<S: SuiteHost>(
    State(st): State<S>,
    SignedIn(u): SignedIn,
    headers: HeaderMap,
    Json(p): Json<SidebarPrefs>,
) -> Result<Json<Value>> {
    if p.order.len() > MAX_PREF_KEYS || p.hidden.len() > MAX_PREF_KEYS || p.order.iter().chain(&p.hidden).any(|k| k.len() > 200) {
        return Err(err(400, "The sidebar choices are too long."));
    }
    let mut all = st.sidebar_prefs().prefs(u.id).await?;
    if let Some(o) = all.as_object_mut() {
        o.insert(PREFS_KEY.into(), serde_json::to_value(&p).unwrap_or(Value::Null));
    }
    st.sidebar_prefs().save_prefs(u.id, &all).await?;
    Ok(Json(sidebar_answer(&st, &headers, &p).await?))
}

/// `GET /api/kit/suite/search?q=&scope=&apps=&limit=` (signed in): this app's search and every linked app's,
/// at once: `{"groups": [<group>…]}`, this app first, each linked app in link order, only apps that found
/// something or failed (with the reason in `error`). `scope=others` leaves this app out (cross-links),
/// `scope=self` the others.
async fn suite_search<S: SuiteHost>(
    State(st): State<S>,
    SignedIn(u): SignedIn,
    headers: HeaderMap,
    Query(q): Query<SearchQuery>,
) -> Result<Json<Value>> {
    let text = query_text(&q.q)?;
    let limit = q.limit.unwrap_or(10).clamp(1, 50);
    let scope = q.scope.as_deref().unwrap_or("all");
    let only: Option<Vec<String>> =
        q.apps.as_deref().map(|a| a.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect());
    let wanted = |id: &str| only.as_ref().is_none_or(|o| o.iter().any(|x| x == id));
    let me = st.suite().manifest(&headers);
    let mut groups: Vec<SearchGroup> = Vec::new();

    let local = async {
        if scope == "others" || !wanted(&me.id) {
            return None;
        }
        Some(match st.search(Caller::Account(u.clone()), text.clone(), limit).await {
            Ok(mut hits) => {
                hits.truncate(limit);
                SearchGroup::of(&me, true, hits)
            }
            Err(e) => SearchGroup { error: Some(e.detail), ..SearchGroup::of(&me, true, vec![]) },
        })
    };
    let links = st.suite_store().links().await?;
    let remote_ms: Vec<(Manifest, String)> = if scope == "self" {
        vec![]
    } else {
        links.into_iter().filter_map(|l| l.manifest.map(|m| (m, l.api_key))).filter(|(m, _)| m.search.is_some() && wanted(&m.id)).collect()
    };
    let http = st.suite().http();
    let remote = futures_util::future::join_all(remote_ms.iter().map(|(m, key)| {
        let text = text.clone();
        let user = u.username.clone();
        async move {
            match remote::search(http, m, key, &text, limit, Some(&user)).await {
                Ok(hits) => SearchGroup::of(m, false, hits),
                Err(e) => SearchGroup { error: Some(e), ..SearchGroup::of(m, false, vec![]) },
            }
        }
    }));
    let (local, remote) = futures_util::join!(local, remote);
    groups.extend(local);
    groups.extend(remote);
    groups.retain(|g| !g.results.is_empty() || g.error.is_some());
    Ok(Json(json!({ "query": text, "groups": groups })))
}
