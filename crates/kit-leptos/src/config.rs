//! The app the components work for: its id, name, version and logo, the server paths they call, the app's
//! routes and every word they show.
//!
//! An app sets its [`AppConfig`] once at start-up ([`set`]); every component reads it ([`get`]). Nothing has to
//! be set: the defaults are a neutral app ("App", id `app`) on the kit's default paths (`/api/health`,
//! `/api/changelog/*`, `/api/notices`, `/api/bug-reports`, `/api/diagnostics`, `/api/settings/*`) with the
//! kit's words.
//!
//! ```
//! use kit_leptos::config::{self, AppConfig};
//! use kit_rules::app::App;
//!
//! let mut cfg = AppConfig::new(App::named("Kit Demo").with_version("1.4.0"));
//! cfg.paths.diagnostics = "/checks".into();
//! cfg.words.update.ready = "{app} has a new version";
//! config::set(cfg);
//! assert_eq!(config::get().app.key("update.tried"), "kit-demo.update.tried");
//! ```

pub use crate::schema_form::FormWords;
use kit_rules::app::App;
use kit_rules::bugs::BugWords;
use kit_rules::changelog::ChangelogWords;
use kit_rules::diagnostics::DiagWords;
use kit_rules::dialogs::DialogWords;
use kit_rules::notices::CheckWords;
use kit_rules::offline::mode::OfflineWords;
use kit_rules::palette::PaletteWords;
use kit_rules::toasts::{ToastPolicy, ToastWords};
use kit_rules::update::UpdateWords;
use std::cell::RefCell;
use std::rc::Rc;

/// The server paths the components call. Every path but [`Paths::api`] and [`Paths::worker`] is relative to
/// [`Paths::api`]; `{name}` and `{key}` are filled in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    /// The prefix of every API path (`/api`).
    pub api: String,
    /// The health check the self-update asks (`/health`).
    pub health: String,
    /// Every release with its notes (`/changelog`).
    pub changelog: String,
    /// The releases this account has not seen (`/changelog/new`).
    pub changelog_new: String,
    /// Marks them seen (`/changelog/seen`).
    pub changelog_seen: String,
    /// Where the What's new switch is saved with `PUT` (`/auth/prefs`).
    pub popup: String,
    /// The field of that body that holds the switch (`whats_new_popup`).
    pub popup_field: String,
    /// The notices (`/notices`).
    pub notices: String,
    /// Check again (`/notices/check`).
    pub notices_check: String,
    /// Where a new bug report is posted (`/bug-reports`).
    pub bug_reports: String,
    /// Where a crash log is posted (`/crash-reports`).
    pub crash_reports: String,
    /// The Diagnostics page's checks (`/diagnostics`).
    pub diagnostics: String,
    /// A settings card's Test button (`/diagnostics/{name}/test`).
    pub test: String,
    /// The settings values: `GET` and `PUT` (`/settings`).
    pub settings: String,
    /// The settings schema (`/settings/schema`).
    pub settings_schema: String,
    /// A saved secret, shown on request (`/settings/secret/{key}`).
    pub settings_secret: String,
    /// The sign-up routes' base (`/signup`): `GET` the policy, `POST` a sign-up, `/invite`, `/verify`, `/settings`,
    /// `/invites`, `/pending`.
    pub signup: String,
    /// The service worker's script, from the site's root (`/sw.js`).
    pub worker: String,
}

impl Default for Paths {
    fn default() -> Self {
        Paths {
            api: "/api".into(),
            health: "/health".into(),
            changelog: "/changelog".into(),
            changelog_new: "/changelog/new".into(),
            changelog_seen: "/changelog/seen".into(),
            popup: "/auth/prefs".into(),
            popup_field: "whats_new_popup".into(),
            notices: "/notices".into(),
            notices_check: "/notices/check".into(),
            bug_reports: "/bug-reports".into(),
            crash_reports: "/crash-reports".into(),
            diagnostics: "/diagnostics".into(),
            test: "/diagnostics/{name}/test".into(),
            settings: "/settings".into(),
            settings_schema: "/settings/schema".into(),
            settings_secret: "/settings/secret/{key}".into(),
            signup: "/signup".into(),
            worker: "/sw.js".into(),
        }
    }
}

impl Paths {
    /// `path` with `{name}` (a test) or `{key}` (a secret) filled in.
    pub fn fill(path: &str, value: &str) -> String {
        path.replace("{name}", value).replace("{key}", value)
    }

    /// A path from the site's root: [`Paths::api`] then `path` (`/api/health`).
    pub fn full(&self, path: &str) -> String {
        format!("{}{path}", self.api)
    }
}

/// The app's own pages the components link to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Routes {
    /// The changelog page, where the popup's Full changelog goes (`/changelog`).
    pub changelog: String,
}

impl Default for Routes {
    fn default() -> Self {
        Routes { changelog: "/changelog".into() }
    }
}

/// Every word the components show, per component. Each one defaults to the kit's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Toasts and copies.
    pub toasts: ToastWords,
    /// Dialogs and the confirm dialog.
    pub dialogs: DialogWords,
    /// What's new and the changelog page.
    pub changelog: ChangelogWords,
    /// The update toast.
    pub update: UpdateWords,
    /// The offline and connection banners.
    pub offline: OfflineWords,
    /// The New report sheet.
    pub bugs: BugWords,
    /// The Diagnostics page.
    pub diag: DiagWords,
    /// Check again's toast for a notice the app gives no words of its own for.
    pub notices: CheckWords,
    /// The settings form.
    pub form: FormWords,
    /// The command palette.
    pub palette: PaletteWords,
}

/// One app's configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct AppConfig {
    /// Its id (storage keys, events, the service worker's names), name and version.
    pub app: App,
    /// Its logo, an image URL, if it has one (the demo's head draws it).
    pub logo: Option<String>,
    /// The stem of its script's name (`<id>-web`: trunk names it `<stem>-<hash>.js`).
    pub script: String,
    /// The server paths.
    pub paths: Paths,
    /// The app's own pages.
    pub routes: Routes,
    /// The words.
    pub words: Words,
    /// How toasts stack and go.
    pub toast_policy: ToastPolicy,
}

impl AppConfig {
    /// `app` on the kit's paths, routes and words, with the web's toasts.
    pub fn new(app: App) -> Self {
        let script = format!("{}-web", app.id);
        AppConfig {
            app,
            logo: None,
            script,
            paths: Paths::default(),
            routes: Routes::default(),
            words: Words::default(),
            toast_policy: ToastPolicy::web(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig::new(App::default())
    }
}

thread_local! {
    static CONFIG: RefCell<Rc<AppConfig>> = RefCell::new(Rc::new(AppConfig::default()));
}

/// Uses `cfg` from here on (call once at start-up, before the components start).
pub fn set(cfg: AppConfig) {
    CONFIG.with(|c| *c.borrow_mut() = Rc::new(cfg));
    crate::toasts::configure();
}

/// The configuration in use.
pub fn get() -> Rc<AppConfig> {
    CONFIG.with(|c| c.borrow().clone())
}

/// The app's name as people read it.
pub fn app_name() -> String {
    get().app.name.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_neutral() {
        let c = AppConfig::default();
        assert_eq!((c.app.id.as_str(), c.app.name.as_str(), c.script.as_str()), ("app", "App", "app-web"));
        assert_eq!(c.paths.full(&c.paths.health), "/api/health");
        assert_eq!(Paths::fill(&c.paths.test, "mail"), "/diagnostics/mail/test");
        assert_eq!(Paths::fill(&c.paths.settings_secret, "token"), "/settings/secret/token");
        assert_eq!(c.routes.changelog, "/changelog");
        assert_eq!(c.words.form, FormWords::DEFAULT);
    }

    #[test]
    fn an_app_sets_its_own() {
        let mut c = AppConfig::new(App::named("Kit Demo").with_version("1.4.0"));
        c.words.update.ready = "{app} has news";
        set(c);
        let got = get();
        assert_eq!(got.script, "kit-demo-web");
        assert_eq!(got.words.update.ready(&got.app.name), "Kit Demo has news");
        assert_eq!(app_name(), "Kit Demo");
        set(AppConfig::default());
    }
}
