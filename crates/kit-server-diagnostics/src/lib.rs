//! One page that checks every connection, folder and tool an app depends on, and says what's wrong and how to
//! fix it (contract: `contracts/diagnostics.md`).
//!
//! The app fills a [`Registry`]: groups of [`Check`]s (a group gives none, one or several checks, so a check
//! that only applies when a setting is filled in simply gives none) and the Test buttons of its settings cards.
//! It hands the registry to the kit through [`Diagnostics`], and merges [`router`] (the default paths) or
//! [`router_with`] (paths of its own, [`DiagnosticsPaths`]):
//!
//! - `GET /api/diagnostics` (admin): every group runs at once; the answer is `{"checks": [...]}` in the order
//!   the groups were added, or whatever [`Diagnostics::answer`] makes of the checks (an app may add sections of
//!   its own, such as its version).
//! - `POST /api/diagnostics/{id}/test` (admin): that one test; `{ok, message, …}`, `{ok: false, message}`
//!   when the test failed on the way, 404 "Unknown check" when there is no test of that id.
//!
//! ```ignore
//! impl kit_server_diagnostics::Diagnostics for App {
//!     fn diagnostics(&self) -> Registry<App> {
//!         Registry::new()
//!             .group("folders", |app: App| async move {
//!                 Ok(vec![Check::new("Data folder", Kind::Path, true, "OK – 12 entries").request("/data")])
//!             })
//!             .test("email", |app: App, who: Account| async move { Ok(json!({ "ok": true, "message": "Sent" })) })
//!     }
//! }
//! let app = Router::new().merge(kit_server_diagnostics::router::<App>());
//! ```

mod check;
mod registry;

pub use check::{Check, Kind, Status};
pub use registry::{test_failed, Registry, TestResult};

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use kit_server_core::{err, Account, Admin, Host, Result};
use serde_json::{json, Value};
use std::future::Future;

/// What the app gives the diagnostics routes: its checks and tests, and (optionally) the rest of the
/// diagnostics answer.
pub trait Diagnostics: Host {
    /// The app's checks and tests. Built when a request comes (cheap: a few closures), or kept and cloned.
    fn diagnostics(&self) -> Registry<Self>;

    /// The diagnostics answer for these `checks` (each already as JSON, in order). The kit's own answer is
    /// `{"checks": [...]}`; an app adds sections of its own here (its version, the time, …).
    fn answer(&self, checks: Vec<Value>) -> impl Future<Output = Result<Value>> + Send {
        async move { Ok(json!({ "checks": checks })) }
    }
}

/// Where the diagnostics routes are served. [`Default`] gives the kit's paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsPaths {
    /// every check at once (`GET`; default `/api/diagnostics`)
    pub list: String,
    /// one test (`POST`; default `/api/diagnostics/{id}/test`); it must hold exactly one path parameter, the
    /// test's id
    pub test: String,
}

impl Default for DiagnosticsPaths {
    fn default() -> Self {
        DiagnosticsPaths { list: "/api/diagnostics".into(), test: "/api/diagnostics/{id}/test".into() }
    }
}

/// The diagnostics routes at the kit's paths: `GET /api/diagnostics` and `POST /api/diagnostics/{id}/test`
/// (admins).
pub fn router<S: Diagnostics>() -> Router<S> {
    router_with(DiagnosticsPaths::default())
}

/// The diagnostics routes at the app's own `paths` (admins).
pub fn router_with<S: Diagnostics>(paths: DiagnosticsPaths) -> Router<S> {
    Router::new().route(&paths.list, get(diagnostics::<S>)).route(&paths.test, post(test::<S>))
}

/// `GET /api/diagnostics`: every check at once (admins).
pub async fn diagnostics<S: Diagnostics>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let checks = st.diagnostics().run(&st).await?;
    let checks = checks.into_iter().map(Check::into_json).collect();
    Ok(Json(st.answer(checks).await?))
}

/// `POST /api/diagnostics/{id}/test`: one settings card's Test button (admins).
pub async fn test<S: Diagnostics>(State(st): State<S>, Admin(who): Admin, Path(id): Path<String>) -> Result<Json<Value>> {
    run_test(&st, &id, who).await.map(Json)
}

/// The test `id` as `who`, as the route answers it: `{ok, message, …}`; 404 "Unknown check".
pub async fn run_test<S: Diagnostics>(st: &S, id: &str, who: Account) -> Result<Value> {
    match st.diagnostics().run_test(st, id, who).await {
        Some(Ok(v)) => Ok(v),
        Some(Err(why)) => Ok(test_failed(why)),
        None => Err(err(404, "Unknown check")),
    }
}

#[cfg(test)]
mod tests;
