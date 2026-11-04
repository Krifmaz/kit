//! Bug reports and app crash reports, from Encore's `routes/bugreports.rs` and `routes/client_crash.rs`.
//!
//! A user writes what went wrong, attaches screenshots or a log, and the admin answers in a thread; the admin
//! can export the lot as a zip (or one plain text file) to hand to the developer. The phone app files its crash
//! log as a report of kind `crash` ("Send crash log"), and uploads the crash reports it wrote itself to
//! `/api/diagnostics/client-crash`.
//!
//! ```text
//! POST   /api/bug-reports                      multipart kind, title, body, page, app_version, device, file… → {ok, id}
//! GET    /api/bug-reports?status=&kind=&q=&mine=1 → {reports: [{id, kind, title, status, username, user_id, created_at,
//!                                              updated_at, message_count, file_count, unread}]}
//! GET    /api/bug-reports/unread               → {count}
//! GET    /api/bug-reports/export?status=&kind=&ids=  (admin) → application/zip (reports.md, reports.json, files/…)
//!        …&format=txt → text/plain: the same reports as one plain text file (files listed by name)
//! GET    /api/bug-reports/{id}                 → {report: {…, files: […]}, messages: [{…, files: […]}]}
//! POST   /api/bug-reports/{id}/messages        multipart body, file… → {ok, id}
//! POST   /api/bug-reports/{id}/status          {status} (admin) → {ok}
//! DELETE /api/bug-reports/{id}                 (admin) → {ok}
//! GET    /api/bug-reports/{id}/files/{fid}     → the file
//!
//! POST   /api/diagnostics/client-crash  {kind, device, device_id, app_version, os, time, summary, text, key} → {ok, id, duplicate}
//! GET    /api/diagnostics/client-crash  (admin) → {crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}
//! DELETE /api/diagnostics/client-crash  (admin) → {ok}
//! ```
//!
//! A non-admin only sees their own reports; someone else's answers 404, not 403. The files live under
//! `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever built from a database row.
//!
//! An app adds the routes with `.merge(kit_server_bugreports::router())` (needs [`BugReportsHost`]) and
//! `.merge(kit_server_bugreports::crash::router())` (needs only [`kit_server_core::Host`]).

mod export;
mod files;
mod reports;
mod time;

pub mod crash;

pub use export::export;
pub use files::{check_file, safe_name};
pub use reports::{create, detail, file, list, remove, reply, set_status, unread, ListQ, StatusBody};

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use kit_server_core::{Account, Host, Result};
use serde_json::Value;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::sync::Semaphore;

/// Bytes per attached file.
pub const MAX_FILE: usize = 8 * 1024 * 1024;
/// Files per report or per message.
pub const MAX_FILES: usize = 6;
/// The whole upload: the files plus the text.
pub const UPLOAD_LIMIT: usize = MAX_FILES * MAX_FILE + 2 * 1024 * 1024;
/// Characters of a report's title.
pub const MAX_TITLE: usize = 200;
/// Characters of a report's or a reply's text.
pub const MAX_BODY: usize = 20_000;
/// Characters of an attached file's name.
pub const MAX_NAME: usize = 120;
/// What an image may decode to: 128 MiB holds a 5K screenshot (5120×2880 RGBA8 is 56 MB, RGBA16 twice that), and a
/// 400 KB PNG claiming 11000×11000 is refused instead of taking half a gigabyte.
pub const MAX_PIXEL_BYTES: u64 = 128 * 1024 * 1024;
/// The most pixels an image may have a side.
pub const MAX_SIDE: u32 = 8192;
/// Reports a user may file in an hour (admins are not held back).
pub const REPORTS_PER_HOUR: usize = 20;
/// Replies a user may send in an hour (admins are not held back).
pub const REPLIES_PER_HOUR: usize = 60;
/// Bytes of files one user's reports may hold in all (admins are not held back).
pub const MAX_USER_BYTES: i64 = 500 * 1024 * 1024;
/// Files an export may hold: the zip is built in memory.
pub const MAX_EXPORT_BYTES: i64 = 512 * 1024 * 1024;

/// `crash`: a crash log the phone app sends (This device → App crashes), the Crash logs section of the list.
pub const KINDS: [&str; 4] = ["bug", "not_bug", "howto", "crash"];
/// A report's statuses.
pub const STATUSES: [&str; 5] = ["new", "looking", "fixed", "wontfix", "answered"];

/// What the bug reports need from the app besides [`Host`]: where files go, the app's name and version for the
/// export, its rate limiter, and how it tells people about new reports and replies.
///
/// Encore's values, as an example: `data_dir` is its config folder, `app_name` "Encore", `app_version` its
/// `VERSION`, `limited` its `RateLimiter`, `log` its activity log, `notify` its Discord webhook, mail through
/// its SMTP settings with `base_url` from its config, and `enrich` its covers-and-translations pass.
pub trait BugReportsHost: Host {
    /// The folder the reports' files are kept under (in `reports/`).
    fn data_dir(&self) -> PathBuf;

    /// The app's name: the export's heading ("Encore bug reports (…)") and file name (`encore-bug-reports-<date>.zip`,
    /// see [`export_stem`]); the reply mail's subject ("Encore: a reply to your report …").
    fn app_name(&self) -> String;

    /// The app's version, in the export's heading and in `reports.json`.
    fn app_version(&self) -> String;

    /// True when `key` already had `limit` events in the last `window_secs` seconds; records one otherwise.
    /// The keys are `bugreport:<user id>` and `bugreply:<user id>`; it is never asked for an admin.
    fn limited(&self, key: &str, limit: usize, window_secs: f64) -> bool;

    /// A line for the app's activity log (`info` or `warning`): a report filed, a status set, a report deleted, a
    /// reply mail that failed. The default writes it to the tracing log.
    fn log(&self, level: &str, message: &str) {
        tracing::info!("{level}: {message}");
    }

    /// Tells the admins something happened ("New bug report", "Bug report reply"), as the app does (Encore: its
    /// Discord webhook). The future is spawned, so a slow notification never holds the request. Default: nothing.
    fn notify(&self, title: String, message: String) -> impl Future<Output = ()> + Send + 'static {
        let _ = (title, message);
        async {}
    }

    /// Whether the app can send mail; when it can, a reporter hears of an admin's reply by mail. Default: no.
    fn mail_enabled(&self) -> bool {
        false
    }

    /// The mail address of account `user_id` ("" for none). Default: `SELECT email FROM users WHERE id=?`.
    fn mail_address(&self, user_id: i64) -> Result<String> {
        use kit_server_core::RowExt;
        Ok(self.db().one("SELECT email FROM users WHERE id=?", kit_server_core::p![user_id])?.map(|x| x.s("email")).unwrap_or_default())
    }

    /// The address the app is reached at (`https://music.example.com`), for the link in a reply mail; "" when
    /// unknown (the link is then `#/bug-reports/<id>`). Default: "".
    fn base_url(&self) -> String {
        String::new()
    }

    /// Sends a plain text mail; the error in words when it fails (logged as "Bug report reply mail failed: …").
    /// The future is spawned. Default: mail isn't set up.
    fn send_mail(&self, to: String, subject: String, text: String) -> impl Future<Output = Result<(), String>> + Send + 'static {
        let _ = (to, subject, text);
        async { Err("mail isn't set up".to_string()) }
    }

    /// What the app adds to a list or a report before it goes out, for the account that asked (Encore: titles
    /// translated for a listener who reads translations). Default: nothing.
    fn enrich(&self, viewer: &Account, answer: &mut Value) -> impl Future<Output = ()> + Send {
        let _ = (viewer, answer);
        async {}
    }

    /// How many uploads are checked (their images decoded) at once: each may hold [`MAX_PIXEL_BYTES`], so two
    /// at a time is what a small server can take. Default: two for the whole process.
    fn decodes(&self) -> Arc<Semaphore> {
        static DECODES: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(2)));
        DECODES.clone()
    }
}

/// The bug report routes.
pub fn router<S: BugReportsHost>() -> Router<S> {
    Router::new()
        .route("/api/bug-reports", get(list::<S>).post(create::<S>).layer(DefaultBodyLimit::max(UPLOAD_LIMIT)))
        .route("/api/bug-reports/unread", get(unread::<S>))
        .route("/api/bug-reports/export", get(export::<S>))
        .route("/api/bug-reports/{id}", get(detail::<S>).delete(remove::<S>))
        .route("/api/bug-reports/{id}/messages", post(reply::<S>).layer(DefaultBodyLimit::max(UPLOAD_LIMIT)))
        .route("/api/bug-reports/{id}/status", post(set_status::<S>))
        .route("/api/bug-reports/{id}/files/{fid}", get(file::<S>))
}

/// The export's file name without its extension: the app's name in lower case, anything but letters and digits
/// as `-`, then `-bug-reports-<date>` (Encore: `encore-bug-reports-2026-11-04`).
pub fn export_stem(app_name: &str, date: &str) -> String {
    let app: String = app_name.trim().chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("{app}-bug-reports-{date}")
}

/// What the side bar and the export call a kind.
pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "not_bug" => "Not a bug",
        "howto" => "How do I…",
        "crash" => "Crash log",
        _ => "Bug",
    }
}

/// What the list and the export call a status.
pub fn status_label(status: &str) -> &'static str {
    match status {
        "looking" => "Looking into it",
        "fixed" => "Fixed",
        "wontfix" => "Won't fix",
        "answered" => "Answered",
        _ => "New",
    }
}

/// `s` on one line: control characters (newlines among them) become spaces and runs of white space one space, so a
/// title can't add lines to the export's Markdown, the log or a notification.
pub fn line(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The first `max` characters of `s`, trimmed.
fn cut(s: &str, max: usize) -> String {
    s.trim().chars().take(max).collect()
}

/// Create the bug report tables when they aren't there (Encore's tables, unchanged).
fn ensure_table(st: &impl Host) -> Result<()> {
    // created on first use: an older server simply never reads these
    st.db().execute_batch(
        "CREATE TABLE IF NOT EXISTS bug_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            username TEXT,
            kind TEXT,
            title TEXT,
            body TEXT,
            page TEXT,
            app_version TEXT,
            device TEXT,
            status TEXT DEFAULT 'new',
            created_at REAL,
            updated_at REAL,
            last_reply_by TEXT DEFAULT '',
            user_seen_at REAL DEFAULT 0,
            admin_seen_at REAL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS bug_report_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            report_id INTEGER,
            user_id INTEGER,
            username TEXT,
            is_admin INTEGER,
            body TEXT,
            created_at REAL
        );
        CREATE TABLE IF NOT EXISTS bug_report_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            report_id INTEGER,
            message_id INTEGER,
            name TEXT,
            mime TEXT,
            size INTEGER,
            path TEXT,
            created_at REAL
        );
        CREATE INDEX IF NOT EXISTS idx_bug_report_messages_report ON bug_report_messages(report_id);
        CREATE INDEX IF NOT EXISTS idx_bug_report_files_report ON bug_report_files(report_id);",
    )
}

/// Where the reports' files are kept.
fn reports_dir(st: &impl BugReportsHost) -> PathBuf {
    st.data_dir().join("reports")
}

#[cfg(test)]
mod tests;
