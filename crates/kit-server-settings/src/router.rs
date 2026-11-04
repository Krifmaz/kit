//! The settings API: `GET /api/admin/settings` (every value, secrets masked, with `_meta`), `PUT
//! /api/admin/settings` (save), `GET /api/admin/settings/schema` (the pages, cards and fields) and `GET
//! /api/admin/settings/secret/{key}` (one saved secret, for the owner). The app says where its values are and
//! what else happens around a save through [`SettingsHost`].

use crate::schema;
use crate::values::{is_masked, Values};
use axum::extract::{FromRequestParts, Path, State};
use axum::http::request::Parts;
use axum::routing::get;
use axum::{Json, Router};
use kit_server_core::{err, Account, Admin, Host, Result, RowExt};
use serde_json::{json, Map, Value};
use std::future::Future;

/// What the settings routes need from the app, on top of [`Host`]. Only [`SettingsHost::settings`] has no
/// default; Encore's values are given as the example of each.
pub trait SettingsHost: Host {
    /// Where the values are: the app's [`crate::Store`] (or its own [`Values`]; Encore: its `Config`).
    fn settings(&self) -> &dyn Values;

    /// The signed-in owner, who alone may see a saved secret; 403 "Only the server owner can do this" for
    /// any other account (Encore's words; the default takes the account whose `role` is `owner`).
    fn owner(&self, parts: &mut Parts) -> impl Future<Output = Result<Account>> + Send {
        async move {
            let a = self.signed_in(parts).await?;
            if a.row.s("role") != "owner" {
                return Err(err(403, "Only the server owner can do this"));
            }
            Ok(a)
        }
    }

    /// `GET /api/admin/settings` and the answer of a save: the values' [`Values::dump`] (Encore adds what its
    /// plugins credit to themselves).
    fn dump(&self) -> Value {
        self.settings().dump()
    }

    /// Keys a save never changes, whatever it is sent (Encore: `api_key`, which has its own route).
    fn kept(&self) -> &'static [&'static str] {
        &[]
    }

    /// Why a saved secret may not be shown, if it may not (Encore: the WireGuard config, "The WireGuard config
    /// can't be shown – paste it again to replace it"); answered as 403.
    fn unrevealed(&self, _key: &str) -> Option<&'static str> {
        None
    }

    /// Before a save, with the values that will be saved: an error refuses the whole save (Encore: 400 for a
    /// `signup_mode` that is not invite, open or closed); the warnings come back as `_warnings` in the answer
    /// (Encore: a download folder it cannot write to).
    fn before_save(&self, _values: &Map<String, Value>) -> Result<Vec<Value>> {
        Ok(vec![])
    }

    /// After a save, with the saved keys (Encore: its plugins follow their auto-update switches, Soulseek
    /// picks up a new login at once).
    fn after_save(&self, _keys: &[String]) {}

    /// A line for the app's activity log ("admin updated settings: app_name", "admin viewed the saved
    /// smtp_password"); the default logs it with `tracing` (Encore: its activity table).
    fn log(&self, message: &str) {
        tracing::info!("{message}");
    }
}

/// The signed-in owner ([`SettingsHost::owner`]).
#[derive(Debug, Clone)]
pub struct Owner(pub Account);

impl<S: SettingsHost> FromRequestParts<S> for Owner {
    type Rejection = kit_server_core::Error;
    async fn from_request_parts(parts: &mut Parts, st: &S) -> Result<Self> {
        st.owner(parts).await.map(Owner)
    }
}

/// The settings routes, for an app to merge: `.merge(kit_server_settings::router())`.
pub fn router<S: SettingsHost>() -> Router<S> {
    Router::new()
        .route("/api/admin/settings", get(get_settings::<S>).put(put_settings::<S>))
        .route("/api/admin/settings/schema", get(settings_schema::<S>))
        .route("/api/admin/settings/secret/{key}", get(reveal_secret::<S>))
}

/// GET /api/admin/settings: every setting, secrets masked.
async fn get_settings<S: SettingsHost>(State(st): State<S>, Admin(_): Admin) -> Json<Value> {
    Json(st.dump())
}

/// The pages, cards and fields of Settings → Server (contracts/settings-schema.md); the values are in
/// GET /api/admin/settings.
async fn settings_schema<S: SettingsHost>(State(st): State<S>, Admin(_): Admin) -> Json<Value> {
    let values = st.settings();
    Json(schema::answer(&values.table(), |k| values.env_locked(k)))
}

/// PUT /api/admin/settings: change settings (unknown, masked, locked and kept values are left out).
async fn put_settings<S: SettingsHost>(
    State(st): State<S>,
    Admin(me): Admin,
    Json(values): Json<Map<String, Value>>,
) -> Result<Json<Value>> {
    let cfg = st.settings();
    let mut clean = Map::new();
    for (k, v) in values {
        if !cfg.known(&k) || is_masked(&v) || st.kept().contains(&k.as_str()) || cfg.env_locked(&k) {
            continue;
        }
        clean.insert(k, v);
    }
    let warnings = st.before_save(&clean)?;
    let mut keys: Vec<&String> = clean.keys().collect();
    keys.sort();
    st.log(&format!("{} updated settings: {}", me.username, keys.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    let saved: Vec<String> = clean.keys().cloned().collect();
    cfg.set_many(&Value::Object(clean));
    st.after_save(&saved);
    let mut out = st.dump();
    if !warnings.is_empty() {
        out["_warnings"] = json!(warnings);
    }
    Ok(Json(out))
}

/// One saved secret in plain text, for the eye/copy buttons in Settings. Owner only, and logged.
async fn reveal_secret<S: SettingsHost>(State(st): State<S>, Owner(me): Owner, Path(key): Path<String>) -> Result<Json<Value>> {
    if !st.settings().known(&key) {
        return Err(err(404, "Unknown setting"));
    }
    if let Some(why) = st.unrevealed(&key) {
        return Err(err(403, why));
    }
    st.log(&format!("{} viewed the saved {key}", me.username));
    let v = st.settings().get(&key);
    Ok(Json(json!({ "key": key, "value": v.as_str().map(String::from).unwrap_or_else(|| v.to_string()) })))
}
