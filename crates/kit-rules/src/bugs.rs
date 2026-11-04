//! Bug reports: the kinds and statuses, the New report sheet's words and checks, and what is sent along.
//!
//! The contract is `contracts/bug-reports.md`. The sheet's words are [`BugWords`]; the file limits are
//! [`FileLimits::BUG_REPORT`] unless the app passes its own.

use crate::uploads::FileLimits;

/// Every kind, as the server stores it and as the user reads it (`crash` is a crash log a native app sends).
pub const KINDS: [(&str, &str); 4] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…"), ("crash", "Crash log")];
/// The kinds a new report can pick (a crash log comes from the app).
pub const PICK_KINDS: [(&str, &str); 3] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…")];
/// The five statuses: key, label, badge class.
pub const STATUSES: [(&str, &str, &str); 5] = [
    ("new", "New", "warn"),
    ("looking", "Looking into it", "info"),
    ("fixed", "Fixed", "ok"),
    ("wontfix", "Won't fix", ""),
    ("answered", "Answered", "accent"),
];

/// The sheet's title.
pub const NEW_REPORT: &str = "New report";
/// The title field's label.
pub const TITLE: &str = "Title";
/// The title field's placeholder.
pub const TITLE_PLACEHOLDER: &str = "A short name for it";
/// The description field's label.
pub const DESCRIPTION: &str = "Description";
/// The description field's placeholder.
pub const DESCRIPTION_PLACEHOLDER: &str = "What happened, what you expected, how to repeat it";
/// The button that opens the file chooser.
pub const ADD_IMAGES: &str = "Add images";
/// The file input's `accept`.
pub const ACCEPT: &str = "image/*,.txt,.log";
/// What follows [`FileLimits::summary`] in the line next to Add images.
pub const FILES_HINT: &str = "Images, .txt and .log files (pick or drop them here).";
/// The toast when Send is pressed without a title.
pub const NEED_TITLE: &str = "Give the report a title";
/// The toast when Send is pressed without a description.
pub const NEED_TEXT: &str = "Say what happened";
/// The toast once the report went.
pub const SENT: &str = "Report sent. Thanks.";
/// The send button.
pub const SEND: &str = "Send";
/// The send button while the report goes.
pub const SENDING: &str = "Sending…";
/// A thumbnail's remove button is labelled "Remove <name>".
pub const REMOVE: &str = "Remove";
/// The title's limit in characters (`maxlength`).
pub const TITLE_MAX: usize = 200;
/// The description's limit in characters (`maxlength`).
pub const TEXT_MAX: usize = 20_000;
/// The browser's user agent is cut to this many characters.
pub const DEVICE_UA_CHARS: usize = 200;

/// A kind's words ("Bug" for one this client does not know).
pub fn kind_label(kind: &str) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).map(|(_, l)| *l).unwrap_or("Bug")
}

/// A kind's badge class (`err` for a bug, `info` for a question, `warn` for a crash log).
pub fn kind_class(kind: &str) -> &'static str {
    match kind {
        "bug" => "err",
        "howto" => "info",
        "crash" => "warn",
        _ => "",
    }
}

/// A status's words ("New" for one this client does not know).
pub fn status_label(status: &str) -> &'static str {
    STATUSES.iter().find(|(k, _, _)| *k == status).map(|(_, l, _)| *l).unwrap_or("New")
}

/// A status's badge class.
pub fn status_class(status: &str) -> &'static str {
    STATUSES.iter().find(|(k, _, _)| *k == status).map(|(_, _, c)| *c).unwrap_or("")
}

/// The line next to Add images: "Up to 6 files, 8 MB each. Images, .txt and .log files (pick or drop them
/// here).".
pub fn files_line(limits: &FileLimits) -> String {
    format!("{} {FILES_HINT}", limits.summary())
}

/// The sheet's last line: what goes with the report ("Sent along: the page you came from (#/projects), the
/// app version (1.4.0) and your browser."). An empty `page` is left out; an empty `version` reads "unknown".
pub fn sent_along(page: &str, version: &str) -> String {
    BugWords::DEFAULT.sent_along(page, version)
}

/// The sheet's last line with the page, `{page}` filled in ([`sent_along`]).
pub const SENT_ALONG: &str = "Sent along: the page you came from ({page}), the app version ({version}) and your browser.";
/// The sheet's last line without a page.
pub const SENT_ALONG_NO_PAGE: &str = "Sent along: the app version ({version}) and your browser.";
/// The version when the app does not know its own.
pub const UNKNOWN_VERSION: &str = "unknown";

/// The New report sheet's words. [`BugWords::DEFAULT`] holds the kit's; an app passes its own to change any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BugWords {
    /// The sheet's title ([`NEW_REPORT`]).
    pub new_report: &'static str,
    /// The kinds a new report can pick, key and words ([`PICK_KINDS`]).
    pub kinds: [(&'static str, &'static str); 3],
    /// The title field's label ([`TITLE`]).
    pub title: &'static str,
    /// The title field's placeholder ([`TITLE_PLACEHOLDER`]).
    pub title_placeholder: &'static str,
    /// The description field's label ([`DESCRIPTION`]).
    pub description: &'static str,
    /// The description field's placeholder ([`DESCRIPTION_PLACEHOLDER`]).
    pub description_placeholder: &'static str,
    /// The button that opens the file chooser ([`ADD_IMAGES`]).
    pub add_images: &'static str,
    /// What follows the limits next to it ([`FILES_HINT`]).
    pub files_hint: &'static str,
    /// The toast when Send is pressed without a title ([`NEED_TITLE`]).
    pub need_title: &'static str,
    /// The toast when Send is pressed without a description ([`NEED_TEXT`]).
    pub need_text: &'static str,
    /// The toast once the report went ([`SENT`]).
    pub sent: &'static str,
    /// The send button ([`SEND`]).
    pub send: &'static str,
    /// The send button while the report goes ([`SENDING`]).
    pub sending: &'static str,
    /// A thumbnail's remove button, before the file's name ([`REMOVE`]).
    pub remove: &'static str,
    /// The last line with the page, with `{page}` and `{version}` ([`SENT_ALONG`]).
    pub sent_along: &'static str,
    /// The last line without a page, with `{version}` ([`SENT_ALONG_NO_PAGE`]).
    pub sent_along_no_page: &'static str,
    /// The version when it is unknown ([`UNKNOWN_VERSION`]).
    pub unknown_version: &'static str,
}

impl BugWords {
    /// The kit's words.
    pub const DEFAULT: BugWords = BugWords {
        new_report: NEW_REPORT,
        kinds: PICK_KINDS,
        title: TITLE,
        title_placeholder: TITLE_PLACEHOLDER,
        description: DESCRIPTION,
        description_placeholder: DESCRIPTION_PLACEHOLDER,
        add_images: ADD_IMAGES,
        files_hint: FILES_HINT,
        need_title: NEED_TITLE,
        need_text: NEED_TEXT,
        sent: SENT,
        send: SEND,
        sending: SENDING,
        remove: REMOVE,
        sent_along: SENT_ALONG,
        sent_along_no_page: SENT_ALONG_NO_PAGE,
        unknown_version: UNKNOWN_VERSION,
    };

    /// The sheet's last line for `page` (left out when empty) and `version` (unknown when empty).
    pub fn sent_along(&self, page: &str, version: &str) -> String {
        let version = if version.is_empty() { self.unknown_version } else { version };
        let t = if page.is_empty() { self.sent_along_no_page } else { self.sent_along };
        crate::app::fill(t, &[("page", page), ("version", version)])
    }

    /// The line next to Add images under `limits`.
    pub fn files_line(&self, limits: &FileLimits) -> String {
        format!("{} {}", limits.summary(), self.files_hint)
    }

    /// What Send says is missing (title first, then the description), or None when the report can go.
    pub fn missing(&self, title: &str, text: &str) -> Option<&'static str> {
        if title.trim().is_empty() {
            Some(self.need_title)
        } else if text.trim().is_empty() {
            Some(self.need_text)
        } else {
            None
        }
    }
}

impl Default for BugWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The `device` field: the user agent (at most [`DEVICE_UA_CHARS`] characters) and the screen size, when it
/// is known ("Mozilla/5.0 … · 1920x1080").
pub fn device(user_agent: &str, screen: Option<(i32, i32)>) -> String {
    let ua: String = user_agent.chars().take(DEVICE_UA_CHARS).collect();
    match screen {
        Some((w, h)) => format!("{ua} · {w}x{h}"),
        None => ua,
    }
}

/// What Send says is missing (title first, then the description), or None when the report can go. Both are
/// trimmed.
pub fn missing(title: &str, text: &str) -> Option<&'static str> {
    if title.trim().is_empty() {
        Some(NEED_TITLE)
    } else if text.trim().is_empty() {
        Some(NEED_TEXT)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_statuses_read_as_the_server_writes_them() {
        assert_eq!(kind_label("not_bug"), "Not a bug");
        assert_eq!(kind_label("crash"), "Crash log");
        assert_eq!(kind_label("something new"), "Bug");
        assert_eq!((kind_class("bug"), kind_class("howto"), kind_class("crash"), kind_class("not_bug")), ("err", "info", "warn", ""));
        assert_eq!(status_label("looking"), "Looking into it");
        assert_eq!(status_label("?"), "New");
        assert_eq!((status_class("answered"), status_class("wontfix")), ("accent", ""));
        assert_eq!(PICK_KINDS.map(|k| k.0), ["bug", "not_bug", "howto"]);
    }

    #[test]
    fn the_sheet_says_what_goes_along() {
        assert_eq!(files_line(&FileLimits::BUG_REPORT), "Up to 6 files, 8 MB each. Images, .txt and .log files (pick or drop them here).");
        assert_eq!(
            sent_along("#/projects", "1.4.0"),
            "Sent along: the page you came from (#/projects), the app version (1.4.0) and your browser."
        );
        assert_eq!(sent_along("", ""), "Sent along: the app version (unknown) and your browser.");
        assert_eq!(device(&"x".repeat(300), Some((1920, 1080))), format!("{} · 1920x1080", "x".repeat(200)));
        assert_eq!(device("UA", None), "UA");
    }

    #[test]
    fn send_needs_a_title_and_a_description() {
        assert_eq!(missing(" ", "x"), Some(NEED_TITLE));
        assert_eq!(missing("t", " \n"), Some(NEED_TEXT));
        assert_eq!(missing("t", "x"), None);
    }

    #[test]
    fn an_app_may_word_the_sheet_its_own_way() {
        let mine = BugWords { sent_along: "Mit: {page} · {version}", need_title: "Titel fehlt", ..BugWords::DEFAULT };
        assert_eq!(mine.sent_along("#/x", ""), "Mit: #/x · unknown");
        assert_eq!(mine.missing("", "x"), Some("Titel fehlt"));
        assert_eq!(BugWords::DEFAULT.files_line(&FileLimits::BUG_REPORT), files_line(&FileLimits::BUG_REPORT));
        assert_eq!(BugWords::default(), BugWords::DEFAULT);
    }
}
