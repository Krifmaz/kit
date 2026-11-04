//! In-app updates (contract: `contracts/update-feed.md`): the app's GitHub releases as an update feed, and the
//! server's own update routes.
//!
//! - [`feed`]: what an app updater (phone, desktop) reads from the release list, over the app's
//!   [`feed::Feed`] (repository, tag prefixes, each platform's file name): the newest full release, its file
//!   for the device, release downloads only, SHA256SUMS parsing and checking, notes cut to [`NOTES_MAX`]
//!   characters.
//! - [`server`]: the server's own update: the newest release (stable or hotfix [`server::Channel`]),
//!   [`server::is_newer`] and the shared part of the update answer.
//! - [`Restart`]: "Restart server", planned so the answer gets out first; exit code [`EXIT_RESTART`].
//! - [`router`]: `GET /api/update[?force=1]`, `POST /api/update {tag?}` → 202 and `POST /api/restart` → 202
//!   (admins), over the app's [`Updates`]; [`router_with`] serves them at the app's own [`UpdatePaths`]. How
//!   the server replaces itself is the app's.
//!
//! ```ignore
//! impl kit_server_updates::Updates for App {
//!     async fn update_status(&self, force: bool) -> Result<Value> { Ok(my_update_card(self, force).await) }
//!     async fn update_now(&self, who: &Account, tag: Option<String>) -> Result<Value> { Err(err(409, "Update now is not available here")) }
//!     fn restart(&self, who: &Account) { self.restart.schedule(kit_server_updates::RESTART_DELAY); }
//! }
//! let app = Router::new().merge(kit_server_updates::router::<App>());
//! ```

pub mod feed;
mod restart;
pub mod server;

pub use restart::{Restart, EXIT_RESTART, RESTART_DELAY};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use kit_server_core::{Account, Admin, Host, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::future::Future;

/// The longest release notes kept (characters; longer ones are cut).
pub const NOTES_MAX: usize = 4000;

/// A plain `MAJOR.MINOR.PATCH` (a release build); anything else (`0.0.1-dev`, `dev.12`) is a build without a
/// release number.
pub fn is_release_version(v: &str) -> bool {
    let p: Vec<&str> = v.trim().split('.').collect();
    p.len() == 3 && p.iter().all(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()))
}

/// The app updaters' rule: `latest` (a release number) is newer than the running `current`, by number; a
/// build without a release number counts every release as newer.
pub fn newer_release(latest: &str, current: &str) -> bool {
    if !is_release_version(latest) {
        return false;
    }
    if !is_release_version(current) {
        return true;
    }
    kit_server_core::version::compare(latest, current) == std::cmp::Ordering::Greater
}

/// What the app does for the server's update routes.
pub trait Updates: Host {
    /// The update card (`GET /api/update`): installed and newest version, notes, whether Update now
    /// works; `force` (Check now) asks again instead of the kept answer.
    fn update_status(&self, force: bool) -> impl Future<Output = Result<Value>> + Send;

    /// Update now to `tag` (none: the newest release), asked for by `who`; the 202 answer's body, or the
    /// error the route answers (such as a 409 when this install cannot update itself).
    fn update_now(&self, who: &Account, tag: Option<String>) -> impl Future<Output = Result<Value>> + Send;

    /// Restart server, asked for by `who`: plan the restart (a [`Restart`] with [`RESTART_DELAY`]) so the
    /// 202 answer gets out first.
    fn restart(&self, who: &Account);
}

/// Where the update routes are served. [`Default`] gives the kit's paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePaths {
    /// the update card (`GET`) and Update now (`POST`); default `/api/update`
    pub update: String,
    /// Restart server (`POST`); default `/api/restart`
    pub restart: String,
}

impl Default for UpdatePaths {
    fn default() -> Self {
        UpdatePaths { update: "/api/update".into(), restart: "/api/restart".into() }
    }
}

/// The update routes at the kit's paths: `GET`/`POST /api/update` and `POST /api/restart` (admins).
pub fn router<S: Updates>() -> Router<S> {
    router_with(UpdatePaths::default())
}

/// The update routes at the app's own `paths` (admins).
pub fn router_with<S: Updates>(paths: UpdatePaths) -> Router<S> {
    Router::new().route(&paths.update, get(get_update::<S>).post(post_update::<S>)).route(&paths.restart, post(post_restart::<S>))
}

/// `GET /api/update` query.
#[derive(Deserialize)]
pub struct UpdateQ {
    /// "1", "true" or "yes" checks again instead of the kept answer
    force: Option<String>,
}

/// `POST /api/update` body.
#[derive(Deserialize, Default)]
pub struct UpdateBody {
    /// the release to update to (default: the newest)
    tag: Option<String>,
}

/// `GET /api/update`: the running version and the newest release (admins).
pub async fn get_update<S: Updates>(State(st): State<S>, Admin(_): Admin, Query(q): Query<UpdateQ>) -> Result<Json<Value>> {
    let force = matches!(q.force.as_deref(), Some("1" | "true" | "yes"));
    Ok(Json(st.update_status(force).await?))
}

/// `POST /api/update`: update the server to a release (admins) → 202.
pub async fn post_update<S: Updates>(
    State(st): State<S>,
    Admin(who): Admin,
    body: Option<Json<UpdateBody>>,
) -> Result<(StatusCode, Json<Value>)> {
    let v = st.update_now(&who, body.and_then(|b| b.0.tag)).await?;
    Ok((StatusCode::ACCEPTED, Json(v)))
}

/// `POST /api/restart`: 202 at once, the restart a moment later (admins).
pub async fn post_restart<S: Updates>(State(st): State<S>, Admin(who): Admin) -> Result<(StatusCode, Json<Value>)> {
    st.restart(&who);
    Ok((StatusCode::ACCEPTED, Json(json!({ "ok": true, "message": "Restarting" }))))
}

#[cfg(test)]
mod tests;
