//! Suite: this app among the other apps on the kit (kit-server-suite). Its manifest names its sections (the web
//! app's pages, as `/#/<page>` addresses, so another app opens them directly), it answers searches with its pages,
//! and it signs in people another app vouches for when that app is linked here as the account source.
//!
//! The settings `public_url` (the address people reach the app at; empty: each request's), `suite_id` (its id
//! among linked apps; empty: the crate's name, so two copies of one app need one each) and `account_source`
//! (other apps may sign people in with their account here) are read at start.

use crate::AppState;
use axum::response::Response;
use kit_server_core::{PrefsStore, Result};
use kit_server_settings::Store;
use kit_server_suite::{Caller, SearchHit, SsoUser, Suite, SuiteConfig, SuiteHost, SuiteStore};

/// The app's icon in the suite: SVG path data in a 24×24 box (four squares). Replace it with the app's own.
pub const ICON: &str = "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z";

/// The app's pages as suite sections: (id, title, icon, path).
pub const SECTIONS: [(&str, &str, &str, &str); 2] = [
    ("home", "Home", "M3 11l9-8 9 8M5 10v10h14V10", "/#/"),
    ("bug-reports", "Bug reports", "M8 8a4 4 0 1 1 8 0v6a4 4 0 1 1-8 0zM3 13h5M16 13h5M4 7l4 3M20 7l-4 3", "/#/bug-reports"),
];

/// What the app says about itself in its manifest, from its settings.
pub fn config(settings: &Store) -> SuiteConfig {
    let id = settings.str("suite_id");
    let id = if id.trim().is_empty() { crate::NAME.to_string() } else { id.trim().to_string() };
    let mut c = SuiteConfig::new(&id, &settings.str("app_name"))
        .icon(ICON)
        .version(crate::VERSION)
        .deep_link("page", "/#/{page}")
        .searchable()
        .account_source(settings.bool("account_source"));
    for (id, title, icon, path) in SECTIONS {
        c = c.section(id, title, icon, path);
    }
    let url = settings.str("public_url");
    if !url.trim().is_empty() {
        c = c.base_url(&url);
    }
    c
}

/// The pages whose title holds `q` (any case), at most `limit`.
pub fn search_pages(q: &str, limit: usize) -> Vec<SearchHit> {
    let q = q.to_lowercase();
    SECTIONS
        .iter()
        .filter(|(_, title, _, _)| title.to_lowercase().contains(&q))
        .take(limit)
        .map(|(id, title, icon, path)| SearchHit {
            title: title.to_string(),
            subtitle: Some("Page".into()),
            url: path.to_string(),
            section: Some(id.to_string()),
            icon: Some(icon.to_string()),
        })
        .collect()
}

impl SuiteHost for AppState {
    fn suite(&self) -> &Suite {
        &self.suite
    }

    fn suite_store(&self) -> &dyn SuiteStore {
        &self.links
    }

    fn sidebar_prefs(&self) -> &dyn PrefsStore {
        &self.prefs
    }

    async fn search(&self, _caller: Caller, q: String, limit: usize) -> Result<Vec<SearchHit>> {
        Ok(search_pages(&q, limit))
    }

    async fn sso_sign_in(&self, user: SsoUser, next: String) -> Result<Response> {
        crate::auth::sso_sign_in(self, user, next)
    }
}
