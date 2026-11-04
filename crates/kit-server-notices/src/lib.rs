//! Plain-language banners (contract: `contracts/notices.md`): a server tells signed-in admins, in one plain
//! sentence, that something is wrong and what to check, with a Check again action. Encore's storage notice
//! (the `downloads` part of `GET /api/requests/storage`, `POST /api/requests/storage/check`), generalised.
//!
//! An app registers its notices ([`NoticeCheck`]) in [`Notices`] and hands them to the kit through
//! [`NoticesHost`]. Each notice has an id, a [`Severity`], how it reads now (`current`, cheap: no probing) and
//! how it is checked again (`check`, on demand and, with [`NoticeCheck::every`], on a timer started by
//! [`spawn_rechecks`]). [`NoticeCheck::kept`] makes one from a single probe and keeps its last result.
//!
//! The kit's default routes ([`router`]): `GET /api/notices` (signed in) → `{"notices": [<notice>…]}` and
//! `POST /api/notices/check` (admin) → the same after every notice was checked again. An app that serves a
//! notice at its own paths or inside its own answer (Encore) calls [`Notices::current`] / [`Notices::check`]
//! and gets the notice's object exactly as the check made it, or merges [`router_at`].
//!
//! ```ignore
//! impl kit_server_notices::NoticesHost for App {
//!     fn notices(&self) -> Notices<App> {
//!         Notices::new().register(NoticeCheck::kept("disk", Severity::Warn, |app: App| async move { probe_disk(&app).await })
//!             .every(Duration::from_secs(60)))
//!     }
//! }
//! let app = Router::new().merge(kit_server_notices::router::<App>());
//! kit_server_notices::spawn_rechecks(state.clone());
//! ```

mod notice;
mod registry;

pub use notice::{Notice, Severity, Toast, Toasts};
pub use registry::{spawn_rechecks, NoticeCheck, Notices, CHECK_WAIT};

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use kit_server_core::{Admin, Host, Result, SignedIn};
use serde_json::{json, Value};

/// What the app gives the notice routes: its notices.
pub trait NoticesHost: Host {
    /// The app's notices. Built when asked for (cheap: a few closures), or kept and cloned; a notice made
    /// with [`NoticeCheck::kept`] keeps its last result in itself, so keep and clone that registry.
    fn notices(&self) -> Notices<Self>;
}

/// The kit's default notice routes: `GET /api/notices` (signed in) and `POST /api/notices/check` (admins).
pub fn router<S: NoticesHost>() -> Router<S> {
    Router::new().route("/api/notices", get(list::<S>)).route("/api/notices/check", post(check_all::<S>))
}

/// One notice served alone at the app's own paths: `GET <get_path>` (signed in) answers the notice's object
/// as the check makes it, and `POST <check_path>` (admins) checks it again first. An unknown `id` answers 404
/// "Not found".
pub fn router_at<S: NoticesHost>(get_path: &str, check_path: &str, id: &str) -> Router<S> {
    let (a, b) = (id.to_string(), id.to_string());
    Router::new()
        .route(
            get_path,
            get(move |State(st): State<S>, SignedIn(who): SignedIn| {
                let id = a.clone();
                async move { st.notices().current(&st, &id, who.admin).await.map(|n| Json(n.into_json())).ok_or_else(not_found) }
            }),
        )
        .route(
            check_path,
            post(move |State(st): State<S>, Admin(_): Admin| {
                let id = b.clone();
                async move { st.notices().check(&st, &id, true).await.map(|n| Json(n.into_json())).ok_or_else(not_found) }
            }),
        )
}

/// The 404 of a notice id the app does not have.
fn not_found() -> kit_server_core::Error {
    kit_server_core::err(404, "Not found")
}

/// `GET /api/notices`: every notice as it reads now, each with its `id` and `severity`.
pub async fn list<S: NoticesHost>(State(st): State<S>, SignedIn(who): SignedIn) -> Result<Json<Value>> {
    let notices = st.notices();
    let out = notices.current_all(&st, who.admin).await;
    Ok(Json(json!({ "notices": out })))
}

/// `POST /api/notices/check`: every notice checked again now (admins), answered as `GET /api/notices`.
pub async fn check_all<S: NoticesHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let notices = st.notices();
    let out = notices.check_all(&st).await;
    Ok(Json(json!({ "notices": out })))
}

#[cfg(test)]
mod tests;
