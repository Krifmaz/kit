//! Bug reports: the kinds and statuses, the New report sheet's words and checks, and what is sent along.
//!
//! From Encore's `web-rs/src/pages/bugreports.rs` (`KINDS`, `PICK_KINDS`, `STATUSES`, `kind_badge`,
//! `status_badge`, `device`, `new_report_modal`) and `contracts/bug-reports.md`. The file limits are
//! [`crate::uploads::FileLimits::BUG_REPORT`].

use crate::uploads::FileLimits;

/// Every kind, as the server stores it and as the user reads it (`crash` is a crash log the phone app sends).
/// From `bugreports.rs` `KINDS`.
pub const KINDS: [(&str, &str); 4] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…"), ("crash", "Crash log")];
/// The kinds a new report can pick (a crash log comes from the app). From `bugreports.rs` `PICK_KINDS`.
pub const PICK_KINDS: [(&str, &str); 3] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…")];
/// The five statuses: key, label, badge class. From `bugreports.rs` `STATUSES`.
pub const STATUSES: [(&str, &str, &str); 5] = [
    ("new", "New", "warn"),
    ("looking", "Looking into it", "info"),
    ("fixed", "Fixed", "ok"),
    ("wontfix", "Won't fix", ""),
    ("answered", "Answered", "accent"),
];

/// The sheet's title. From `bugreports.rs` `new_report_modal`.
pub const NEW_REPORT: &str = "New report";
/// The title field's label. From `bugreports.rs` `new_report_modal`.
pub const TITLE: &str = "Title";
/// The title field's placeholder. From `bugreports.rs` `new_report_modal`.
pub const TITLE_PLACEHOLDER: &str = "A short name for it";
/// The description field's label. From `bugreports.rs` `new_report_modal`.
pub const DESCRIPTION: &str = "Description";
/// The description field's placeholder. From `bugreports.rs` `new_report_modal`.
pub const DESCRIPTION_PLACEHOLDER: &str = "What happened, what you expected, how to repeat it";
/// The button that opens the file chooser. From `bugreports.rs` `new_report_modal`.
pub const ADD_IMAGES: &str = "Add images";
/// The file input's `accept`. From `bugreports.rs` `new_report_modal`.
pub const ACCEPT: &str = "image/*,.txt,.log";
/// What follows [`FileLimits::summary`] in the line next to Add images. From `bugreports.rs` `new_report_modal`.
pub const FILES_HINT: &str = "Images, .txt and .log files (pick or drop them here).";
/// The toast when Send is pressed without a title. From `bugreports.rs` `new_report_modal`.
pub const NEED_TITLE: &str = "Give the report a title";
/// The toast when Send is pressed without a description. From `bugreports.rs` `new_report_modal`.
pub const NEED_TEXT: &str = "Say what happened";
/// The toast once the report went. From `bugreports.rs` `new_report_modal`.
pub const SENT: &str = "Report sent. Thanks.";
/// The send button. From `bugreports.rs` `new_report_modal`.
pub const SEND: &str = "Send";
/// The send button while the report goes. From `bugreports.rs` `new_report_modal`.
pub const SENDING: &str = "Sending…";
/// A thumbnail's remove button is labelled "Remove <name>". From `bugreports.rs` `Picker::draw`.
pub const REMOVE: &str = "Remove";
/// The title's limit in characters (`maxlength`). From `bugreports.rs` and `contracts/bug-reports.md`.
pub const TITLE_MAX: usize = 200;
/// The description's limit in characters (`maxlength`). From `bugreports.rs` and `contracts/bug-reports.md`.
pub const TEXT_MAX: usize = 20_000;
/// The browser's user agent is cut to this many characters. From `bugreports.rs` `device`.
pub const DEVICE_UA_CHARS: usize = 200;

/// A kind's words ("Bug" for one this client does not know). From `bugreports.rs` `kind_label`.
pub fn kind_label(kind: &str) -> &'static str {
    KINDS.iter().find(|(k, _)| *k == kind).map(|(_, l)| *l).unwrap_or("Bug")
}

/// A kind's badge class (`err` for a bug, `info` for a question, `warn` for a crash log). From `bugreports.rs`
/// `kind_badge`.
pub fn kind_class(kind: &str) -> &'static str {
    match kind {
        "bug" => "err",
        "howto" => "info",
        "crash" => "warn",
        _ => "",
    }
}

/// A status's words ("New" for one this client does not know). From `bugreports.rs` `status_label`.
pub fn status_label(status: &str) -> &'static str {
    STATUSES.iter().find(|(k, _, _)| *k == status).map(|(_, l, _)| *l).unwrap_or("New")
}

/// A status's badge class. From `bugreports.rs` `status_badge`.
pub fn status_class(status: &str) -> &'static str {
    STATUSES.iter().find(|(k, _, _)| *k == status).map(|(_, _, c)| *c).unwrap_or("")
}

/// The line next to Add images: "Up to 6 files, 8 MB each. Images, .txt and .log files (pick or drop them
/// here).". From `bugreports.rs` `new_report_modal`.
pub fn files_line(limits: &FileLimits) -> String {
    format!("{} {FILES_HINT}", limits.summary())
}

/// The sheet's last line: what goes with the report ("Sent along: the page you came from (#/library), the app
/// version (0.36.4) and your browser."). An empty `page` is left out; an empty `version` reads "unknown".
/// From `bugreports.rs` `new_report_modal`.
pub fn sent_along(page: &str, version: &str) -> String {
    let page = if page.is_empty() { String::new() } else { format!("the page you came from ({page}), ") };
    let version = if version.is_empty() { "unknown" } else { version };
    format!("Sent along: {page}the app version ({version}) and your browser.")
}

/// The `device` field: the user agent (at most [`DEVICE_UA_CHARS`] characters) and the screen size, when it
/// is known ("Mozilla/5.0 … · 1920x1080"). From `bugreports.rs` `device`.
pub fn device(user_agent: &str, screen: Option<(i32, i32)>) -> String {
    let ua: String = user_agent.chars().take(DEVICE_UA_CHARS).collect();
    match screen {
        Some((w, h)) => format!("{ua} · {w}x{h}"),
        None => ua,
    }
}

/// What Send says is missing (title first, then the description), or None when the report can go. Both are
/// trimmed. From `bugreports.rs` `new_report_modal`.
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
    fn kinds_and_statuses_read_as_encore_writes_them() {
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
            sent_along("#/library", "0.36.4"),
            "Sent along: the page you came from (#/library), the app version (0.36.4) and your browser."
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
}
