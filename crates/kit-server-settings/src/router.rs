//! The settings API, on paths an app may change ([`SettingsPaths`]): `GET /api/settings` (every value, secrets
//! masked, with `_meta`), `PUT /api/settings` (save), `GET /api/settings/schema` (the pages, cards and fields)
//! and `GET /api/settings/secret/{key}` (one saved secret, for the owner). All of them are for admins. The app
//! says where its values are and what else happens around a save through [`SettingsHost`].

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
/// default; the others are where an app adds its own behaviour.
pub trait SettingsHost: Host {
    /// Where the values are: the app's [`crate::Store`] (or its own [`Values`]).
    fn settings(&self) -> &dyn Values;

    /// The signed-in owner, who alone may see a saved secret; 403 "Only the server owner can do this" for
    /// any other account (the default takes the account whose `role` is `owner`).
    fn owner(&self, parts: &mut Parts) -> impl Future<Output = Result<Account>> + Send {
        async move {
            let a = self.signed_in(parts).await?;
            if a.row.s("role") != "owner" {
                return Err(err(403, "Only the server owner can do this"));
            }
            Ok(a)
        }
    }

    /// `GET /api/settings` and the answer of a save: the values' [`Values::dump`] (an app may add to it).
    fn dump(&self) -> Value {
        self.settings().dump()
    }

    /// Keys a save never changes, whatever it is sent (for example an API key that has its own route).
    fn kept(&self) -> &'static [&'static str] {
        &[]
    }

    /// Why a saved secret may not be shown, if it may not (for example "This key can't be shown – paste it
    /// again to replace it"); answered as 403.
    fn unrevealed(&self, _key: &str) -> Option<&'static str> {
        None
    }

    /// Before a save, with the values that will be saved: an error refuses the whole save (for example 400
    /// for a value the app does not accept); the warnings (`{key, message}`) come back as `_warnings` in the
    /// answer (for example a folder the server cannot write to).
    fn before_save(&self, _values: &Map<String, Value>) -> Result<Vec<Value>> {
        Ok(vec![])
    }

    /// After a save, with the saved keys (for example to pick up a new login at once).
    fn after_save(&self, _keys: &[String]) {}

    /// A line for the app's activity log ("admin updated settings: app_name", "admin viewed the saved
    /// smtp_password"); the default logs it with `tracing`.
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

/// Where the settings routes are. [`Default`] gives `/api/settings`, `/api/settings/schema` and
/// `/api/settings/secret/{key}`; an app that wants others sets them (`secret` must end in `{key}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPaths {
    /// `GET` every value, `PUT` a save
    pub values: String,
    /// `GET` the schema answer
    pub schema: String,
    /// `GET` one saved secret; ends in `{key}`
    pub secret: String,
}

impl Default for SettingsPaths {
    fn default() -> Self {
        SettingsPaths { values: "/api/settings".into(), schema: "/api/settings/schema".into(), secret: "/api/settings/secret/{key}".into() }
    }
}

/// The settings routes on [`SettingsPaths::default`], for an app to merge: `.merge(kit_server_settings::router())`.
pub fn router<S: SettingsHost>() -> Router<S> {
    router_with(SettingsPaths::default())
}

/// The settings routes on `paths`.
///
/// # Panics
///
/// When `paths.secret` does not end in `{key}`, or two paths are the same (axum refuses them).
pub fn router_with<S: SettingsHost>(paths: SettingsPaths) -> Router<S> {
    assert!(paths.secret.ends_with("{key}"), "SettingsPaths::secret must end in {{key}}: {}", paths.secret);
    Router::new()
        .route(&paths.values, get(get_settings::<S>).put(put_settings::<S>))
        .route(&paths.schema, get(settings_schema::<S>))
        .route(&paths.secret, get(reveal_secret::<S>))
}

/// GET /api/settings: every setting, secrets masked.
async fn get_settings<S: SettingsHost>(State(st): State<S>, Admin(_): Admin) -> Json<Value> {
    Json(st.dump())
}

/// The pages, cards and fields of the Settings screen (contracts/settings-schema.md); the values are in
/// GET /api/settings.
async fn settings_schema<S: SettingsHost>(State(st): State<S>, Admin(_): Admin) -> Json<Value> {
    let values = st.settings();
    Json(schema::answer(&values.table(), |k| values.env_locked(k)))
}

/// PUT /api/settings: change settings (unknown, masked, locked and kept values are left out).
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
