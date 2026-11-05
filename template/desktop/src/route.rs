//! The app's pages and the paths that name them. The paths are the web app's (`#/settings?s=diagnostics`), so a
//! bug report sent from the desktop names its page the way the web's do.

use kit_iced::widgets::Icon;

/// A page of the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The first page: the server, and every kit feature to try.
    Home,
    /// The bug reports this account may see, and New report.
    BugReports,
    /// One bug report with its thread and the reply box.
    Report(i64),
    /// The server's settings form (admins), and This computer (the look and updates).
    Settings,
    /// The server's checks (admins).
    Diagnostics,
    /// Every release's notes.
    WhatsNew,
    /// One search across this app and every linked app.
    Search,
}

impl Page {
    /// The app's own pages in the shared sidebar until the server's sidebar (every linked app's sections, in the
    /// person's order) has come, or when it can't be had.
    pub const OWN: [Page; 2] = [Page::Home, Page::BugReports];

    /// The sidebar's pages under the shared sidebar, in order.
    pub const NAV: [Page; 4] = [Page::Search, Page::Settings, Page::Diagnostics, Page::WhatsNew];

    /// The page's name in the sidebar and its heading.
    pub fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::BugReports | Page::Report(_) => "Bug reports",
            Page::Settings => "Settings",
            Page::Diagnostics => "Diagnostics",
            Page::WhatsNew => "What's new",
            Page::Search => "Search",
        }
    }

    /// The sidebar's icon.
    pub fn icon(self) -> Icon {
        match self {
            Page::Home => Icon::Home,
            Page::BugReports | Page::Report(_) => Icon::Bug,
            Page::Settings => Icon::Settings,
            Page::Diagnostics => Icon::Activity,
            Page::WhatsNew => Icon::Spark,
            Page::Search => Icon::Search,
        }
    }

    /// Only admins open it (the server says no to anyone else).
    pub fn admin_only(self) -> bool {
        matches!(self, Page::Settings | Page::Diagnostics)
    }

    /// The sidebar entry this page lights up (a report lights up Bug reports).
    pub fn nav(self) -> Page {
        match self {
            Page::Report(_) => Page::BugReports,
            p => p,
        }
    }

    /// The web app's path for the page (`#/settings?s=whats-new`), sent along with a bug report.
    pub fn path(self) -> String {
        match self {
            Page::Home => "#/".into(),
            Page::BugReports => "#/bug-reports".into(),
            Page::Report(id) => format!("#/bug-reports/{id}"),
            Page::Settings => "#/settings".into(),
            Page::Diagnostics => "#/settings?s=diagnostics".into(),
            Page::WhatsNew => "#/settings?s=whats-new".into(),
            Page::Search => "#/search".into(),
        }
    }

    /// The page a web path names (with or without `#`, or as a suite section's `/#/…`); `None` for one the
    /// desktop app doesn't have.
    pub fn parse(path: &str) -> Option<Page> {
        let p = path.trim();
        let p = p.strip_prefix("/#").unwrap_or(p).trim_start_matches('#');
        let (p, query) = p.split_once('?').unwrap_or((p, ""));
        let s = query.split('&').filter_map(|kv| kv.split_once('=')).find(|(k, _)| *k == "s").map(|(_, v)| v).unwrap_or("");
        match (p.trim_end_matches('/'), s) {
            ("", _) => Some(Page::Home),
            ("/bug-reports", _) => Some(Page::BugReports),
            ("/search", _) => Some(Page::Search),
            ("/settings", "diagnostics") => Some(Page::Diagnostics),
            ("/settings", "whats-new") => Some(Page::WhatsNew),
            ("/settings", _) => Some(Page::Settings),
            (rest, _) => rest.strip_prefix("/bug-reports/").and_then(|id| id.parse().ok()).map(Page::Report),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_goes_there_and_back() {
        for p in Page::OWN.into_iter().chain(Page::NAV).chain([Page::Report(7)]) {
            assert_eq!(Page::parse(&p.path()), Some(p), "{}", p.path());
        }
    }

    #[test]
    fn web_paths_name_the_pages() {
        assert_eq!(Page::parse("#/"), Some(Page::Home));
        assert_eq!(Page::parse(""), Some(Page::Home));
        assert_eq!(Page::parse("#/settings?s=general"), Some(Page::Settings));
        assert_eq!(Page::parse("/bug-reports/12"), Some(Page::Report(12)));
        assert_eq!(Page::parse("#/bug-reports/x"), None);
        assert_eq!(Page::parse("#/library"), None);
        assert_eq!(Page::parse("/#/bug-reports"), Some(Page::BugReports), "a suite section's path");
        assert_eq!(Page::parse("/#/"), Some(Page::Home));
    }

    #[test]
    fn the_sidebar() {
        assert_eq!(Page::OWN.map(Page::title), ["Home", "Bug reports"]);
        assert_eq!(Page::NAV.map(Page::title), ["Search", "Settings", "Diagnostics", "What's new"]);
        assert!(Page::Settings.admin_only() && Page::Diagnostics.admin_only() && !Page::WhatsNew.admin_only());
        assert_eq!(Page::Report(3).nav(), Page::BugReports);
    }
}
