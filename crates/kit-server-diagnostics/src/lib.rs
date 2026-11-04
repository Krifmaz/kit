//! One page that checks every connection, folder and tool, and says what's wrong and how to fix it
//! (contract: `contracts/diagnostics.md`). From Encore's `GET /api/system/diagnostics`
//! (`server/crates/api/src/routes/stats.rs`) and `POST /api/system/test/{service}` (`routes/system.rs`).
//!
//! The app fills a [`Registry`]: groups of [`Check`]s (a group gives none, one or several checks, so a check
//! that only applies when a setting is filled in simply gives none) and the Test buttons of its settings cards.
//! It hands the registry to the kit through [`Diagnostics`], and merges [`router`]:
//!
//! - `GET /api/system/diagnostics` (admin): every group runs at once; the answer is `{"checks": [...]}` in the
//!   order the groups were added, or whatever [`Diagnostics::answer`] makes of the checks (Encore adds its
//!   version, settings, machine and download folders).
//! - `POST /api/system/test/{service}` (admin): that one test; `{ok, message, …}`, `{ok: false, message}`
//!   when the test failed on the way, 404 "Unknown service" when there is no test of that name.
//!
//! ```ignore
//! impl kit_server_diagnostics::Diagnostics for App {
//!     fn diagnostics(&self) -> Registry<App> {
//!         Registry::new()
//!             .group("library", |app: App| async move {
//!                 Ok(vec![Check::new("Music library", Kind::Path, true, "OK – 12 entries").request("/music")])
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

    /// The `GET /api/system/diagnostics` answer for these `checks` (each already as JSON, in order). The kit's
    /// own answer is `{"checks": [...]}`; an app adds sections of its own here (Encore: `version`, `server`,
    /// `uid`, `settings`, `machine`, `download_folders`, `time`).
    fn answer(&self, checks: Vec<Value>) -> impl Future<Output = Result<Value>> + Send {
        async move { Ok(json!({ "checks": checks })) }
    }
}

/// The diagnostics routes: `GET /api/system/diagnostics` and `POST /api/system/test/{service}` (admins).
pub fn router<S: Diagnostics>() -> Router<S> {
    Router::new().route("/api/system/diagnostics", get(diagnostics::<S>)).route("/api/system/test/{service}", post(test::<S>))
}

/// `GET /api/system/diagnostics`: every check at once (admins).
pub async fn diagnostics<S: Diagnostics>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let checks = st.diagnostics().run(&st).await?;
    let checks = checks.into_iter().map(Check::into_json).collect();
    Ok(Json(st.answer(checks).await?))
}

/// `POST /api/system/test/{service}`: one settings card's Test button (admins).
pub async fn test<S: Diagnostics>(State(st): State<S>, Admin(who): Admin, Path(service): Path<String>) -> Result<Json<Value>> {
    run_test(&st, &service, who).await.map(Json)
}

/// The test `service` as `who`, as the route answers it: `{ok, message, …}`; 404 "Unknown service".
pub async fn run_test<S: Diagnostics>(st: &S, service: &str, who: Account) -> Result<Value> {
    match st.diagnostics().run_test(st, service, who).await {
        Some(Ok(v)) => Ok(v),
        Some(Err(why)) => Ok(test_failed(why)),
        None => Err(err(404, "Unknown service")),
    }
}

#[cfg(test)]
mod tests;
