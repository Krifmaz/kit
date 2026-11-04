//! In-app updates (contract: `contracts/update-feed.md`): the app's GitHub releases as an update feed, and the
//! server's own update routes. From Encore's `encore_system::update`, `routes/update.rs`, Android's
//! `UpdateRules.kt` and the desktop app's `updater.rs`.
//!
//! - [`feed`]: what an app updater (phone, desktop) reads from the release list: the newest full release,
//!   its file for the device ([`feed::Platform`]), the APK's version from its file name, release downloads
//!   only, SHA256SUMS parsing and checking, notes cut to [`NOTES_MAX`] characters.
//! - [`server`]: the server's own update: the newest release (stable or hotfix [`server::Channel`]),
//!   [`server::is_newer`] and the shared part of the update answer.
//! - [`Restart`]: "Restart server", planned so the answer gets out first; exit code [`EXIT_RESTART`].
//! - [`router`]: `GET /api/system/update[?force=1]`, `POST /api/system/update {tag?}` → 202 and
//!   `POST /api/system/restart` → 202 (admins), over the app's [`Updates`] (Encore's Docker self-update is
//!   Encore's own).
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

/// The tag prefix of the hotfix builds CI publishes on every push (`hotfix-0.36.0.17`).
pub const HOTFIX_PREFIX: &str = "hotfix-";

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
    /// The update card (`GET /api/system/update`): installed and newest version, notes, whether Update now
    /// works; `force` (Check now) asks again instead of the kept answer.
    fn update_status(&self, force: bool) -> impl Future<Output = Result<Value>> + Send;

    /// Update now to `tag` (none: the newest release), asked for by `who`; the 202 answer's body, or the
    /// error the route answers (Encore: 409 "The Docker socket is not mounted").
    fn update_now(&self, who: &Account, tag: Option<String>) -> impl Future<Output = Result<Value>> + Send;

    /// Restart server, asked for by `who`: plan the restart (a [`Restart`] with [`RESTART_DELAY`]) so the
    /// 202 answer gets out first.
    fn restart(&self, who: &Account);
}

/// The update routes: `GET`/`POST /api/system/update` and `POST /api/system/restart` (admins).
pub fn router<S: Updates>() -> Router<S> {
    Router::new()
        .route("/api/system/update", get(get_update::<S>).post(post_update::<S>))
        .route("/api/system/restart", post(post_restart::<S>))
}

/// `GET /api/system/update` query.
#[derive(Deserialize)]
pub struct UpdateQ {
    /// "1", "true" or "yes" checks again instead of the kept answer
    force: Option<String>,
}

/// `POST /api/system/update` body.
#[derive(Deserialize, Default)]
pub struct UpdateBody {
    /// the release to update to (default: the newest)
    tag: Option<String>,
}

/// `GET /api/system/update`: the running version and the newest release (admins).
pub async fn get_update<S: Updates>(State(st): State<S>, Admin(_): Admin, Query(q): Query<UpdateQ>) -> Result<Json<Value>> {
    let force = matches!(q.force.as_deref(), Some("1" | "true" | "yes"));
    Ok(Json(st.update_status(force).await?))
}

/// `POST /api/system/update`: update the server to a release (admins) → 202.
pub async fn post_update<S: Updates>(
    State(st): State<S>,
    Admin(who): Admin,
    body: Option<Json<UpdateBody>>,
) -> Result<(StatusCode, Json<Value>)> {
    let v = st.update_now(&who, body.and_then(|b| b.0.tag)).await?;
    Ok((StatusCode::ACCEPTED, Json(v)))
}

/// `POST /api/system/restart`: 202 at once, the restart a moment later (admins).
pub async fn post_restart<S: Updates>(State(st): State<S>, Admin(who): Admin) -> Result<(StatusCode, Json<Value>)> {
    st.restart(&who);
    Ok((StatusCode::ACCEPTED, Json(json!({ "ok": true, "message": "Restarting" }))))
}

#[cfg(test)]
mod tests;
