//! Plain-language banners (contract: `contracts/notices.md`): a server tells signed-in admins, in one plain
//! sentence, that something is wrong and what to check, with a Check again action.
//!
//! An app registers its notices ([`NoticeCheck`]) in [`Notices`] and hands them to the kit through
//! [`NoticesHost`]. Each notice has an id, a [`Severity`], how it reads now (`current`, cheap: no probing) and
//! how it is checked again (`check`, on demand and, with [`NoticeCheck::every`], on a timer started by
//! [`spawn_rechecks`]). [`NoticeCheck::kept`] makes one from a single probe and keeps its last result.
//!
//! The routes ([`router`], or [`router_with`] at the app's own [`NoticePaths`]): `GET /api/notices` (signed
//! in) → `{"notices": [<notice>…]}` and `POST /api/notices/check` (admin) → the same after every notice was
//! checked again. An app that shows a notice inside an answer of its own calls [`Notices::current`] /
//! [`Notices::check`] and gets the notice's object exactly as the check made it.
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

/// Where the notice routes are served. [`Default`] gives the kit's paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticePaths {
    /// every notice as it reads now (`GET`; default `/api/notices`)
    pub list: String,
    /// every notice checked again (`POST`; default `/api/notices/check`)
    pub check: String,
}

impl Default for NoticePaths {
    fn default() -> Self {
        NoticePaths { list: "/api/notices".into(), check: "/api/notices/check".into() }
    }
}

/// The notice routes at the kit's paths: `GET /api/notices` (signed in) and `POST /api/notices/check`
/// (admins).
pub fn router<S: NoticesHost>() -> Router<S> {
    router_with(NoticePaths::default())
}

/// The notice routes at the app's own `paths`.
pub fn router_with<S: NoticesHost>(paths: NoticePaths) -> Router<S> {
    Router::new().route(&paths.list, get(list::<S>)).route(&paths.check, post(check_all::<S>))
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
