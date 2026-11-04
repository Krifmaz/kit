//! Bug reports and app crash reports.
//!
//! A user writes what went wrong, attaches screenshots or a log, and the admin answers in a thread; the admin
//! can export the lot as a zip (or one plain text file) to hand to the developer. An app can file a crash log as
//! a report of kind `crash`, and upload the crash reports it wrote itself to the [`crash`] routes.
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
//! POST   /api/crash-reports  {kind, device, device_id, app_version, os, time, summary, text, key} → {ok, id, duplicate}
//! GET    /api/crash-reports  (admin) → {crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}
//! DELETE /api/crash-reports  (admin) → {ok}
//! ```
//!
//! The paths are the defaults: [`BugReportPaths`] and [`crash::CrashPaths`] move them. The limits ([`Limits`]) and
//! the words people see ([`Words`]: kinds, statuses, the export's title and file name, notifications, the reply
//! mail) are the app's too, with plain English defaults.
//!
//! A non-admin only sees their own reports; someone else's answers 404, not 403. The files live under
//! `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever built from a database row.
//!
//! An app adds the routes with `.merge(kit_server_bugreports::router())` (needs [`BugReportsHost`]) or
//! `router_with(BugReportsConfig { … })`, and `.merge(kit_server_bugreports::crash::router())` (needs
//! [`crash::CrashHost`]).
//!
//! What the routes keep is behind two traits ([`store`]): [`BugReportStore`] and [`CrashStore`]. The kit has them
//! in SQLite ([`SqliteReports`], feature `sqlite`, on by default) and Postgres (`PgReports`, feature
//! `postgres`), in tables of their own (`kit_bug_reports`, `kit_bug_report_messages`, `kit_bug_report_files`,
//! `kit_client_crashes`); the app's own tables are never read.

mod export;
mod files;
#[cfg(feature = "postgres")]
mod pg;
mod reports;
#[cfg(feature = "sqlite")]
mod sqlite;
mod time;

pub mod crash;
pub mod store;

pub use files::{check_file, safe_name};
#[cfg(feature = "postgres")]
pub use pg::PgReports;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteReports;
pub use store::{
    BugReportStore, Crash, CrashStore, FileRow, ListFilter, Message, NewCrash, NewFile, NewMessage, NewReport, Report, ReportSummary, Side,
};

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Extension, Router};
use kit_server_core::{Host, Result};
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock};
use tokio::sync::Semaphore;

/// Characters of an attached file's name (the name is cut to fit, never refused).
pub const MAX_NAME: usize = 120;

/// A report's statuses. Their order and meaning are fixed (an admin's reply makes a `new` report `answered`, the
/// reporter's reply makes an `answered` one `new` again); their words are [`Words::statuses`].
pub const STATUSES: [&str; 5] = ["new", "looking", "fixed", "wontfix", "answered"];

/// How much a report, a user and an export may hold. The defaults suit a small server.
#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    /// bytes per attached file (default 8 MiB)
    pub max_file: usize,
    /// files per report or per reply (default 6)
    pub max_files: usize,
    /// characters of a report's title (default 200)
    pub max_title: usize,
    /// characters of a report's or a reply's text (default 20 000)
    pub max_body: usize,
    /// what an image may decode to (default 128 MiB: a 5K screenshot fits, and a small PNG claiming 11000×11000
    /// is refused instead of taking half a gigabyte)
    pub max_pixel_bytes: u64,
    /// the most pixels an image may have a side (default 8192)
    pub max_side: u32,
    /// reports a user may file in an hour (default 20; admins are not held back)
    pub reports_per_hour: usize,
    /// replies a user may send in an hour (default 60; admins are not held back)
    pub replies_per_hour: usize,
    /// bytes of files one user's reports may hold in all (default 500 MiB; admins are not held back)
    pub max_user_bytes: i64,
    /// bytes of files an export may hold: the zip is built in memory (default 512 MiB)
    pub max_export_bytes: i64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_file: 8 << 20,
            max_files: 6,
            max_title: 200,
            max_body: 20_000,
            max_pixel_bytes: 128 << 20,
            max_side: 8192,
            reports_per_hour: 20,
            replies_per_hour: 60,
            max_user_bytes: 500 << 20,
            max_export_bytes: 512 << 20,
        }
    }
}

impl Limits {
    /// The whole upload a request may carry: the files plus 2 MiB for the text.
    pub fn upload(&self) -> usize {
        self.max_files.saturating_mul(self.max_file).saturating_add(2 << 20)
    }
}

/// The words of the bug reports that people see. `{app}`, `{title}`, `{who}`, `{body}`, `{link}` and `{id}` are
/// filled in where a text names them.
#[derive(Debug, Clone, PartialEq)]
pub struct Words {
    /// the kinds a report may have, (id, words), in the order an error lists them; the first is the kind of a
    /// report that names none. Default: `bug` "Bug", `not_bug` "Not a bug", `howto` "How do I…", `crash` "Crash log"
    pub kinds: Vec<(String, String)>,
    /// the words of each of [`STATUSES`], (id, words). Default: "New", "Looking into it", "Fixed", "Won't fix",
    /// "Answered"
    pub statuses: Vec<(String, String)>,
    /// the export's heading, `{app}` (default "{app} bug reports")
    pub export_title: String,
    /// the export's file name before `-<date>.zip`, `{app}` as the app's name in lower case with anything but
    /// letters and digits as `-` (default "{app}-bug-reports")
    pub export_file: String,
    /// the notification's title for a new report (default "New bug report")
    pub notify_new: String,
    /// the notification's title for a user's reply (default "Bug report reply")
    pub notify_reply: String,
    /// the reply mail's subject, `{app}`, `{title}` (default "{app}: a reply to your report “{title}”")
    pub mail_subject: String,
    /// the reply mail's text, `{who}`, `{title}`, `{body}`, `{link}`
    /// (default "{who} answered your report “{title}”:\n\n{body}\n\n{link}\n")
    pub mail_text: String,
    /// where the app shows a report, `{id}`, after the app's address (default "#/bug-reports/{id}")
    pub report_link: String,
}

impl Default for Words {
    fn default() -> Self {
        let pairs = |l: &[(&str, &str)]| l.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        Words {
            kinds: pairs(&[("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…"), ("crash", "Crash log")]),
            statuses: pairs(&[
                ("new", "New"),
                ("looking", "Looking into it"),
                ("fixed", "Fixed"),
                ("wontfix", "Won't fix"),
                ("answered", "Answered"),
            ]),
            export_title: "{app} bug reports".into(),
            export_file: "{app}-bug-reports".into(),
            notify_new: "New bug report".into(),
            notify_reply: "Bug report reply".into(),
            mail_subject: "{app}: a reply to your report “{title}”".into(),
            mail_text: "{who} answered your report “{title}”:\n\n{body}\n\n{link}\n".into(),
            report_link: "#/bug-reports/{id}".into(),
        }
    }
}

impl Words {
    /// Whether `kind` is one of [`Words::kinds`].
    pub fn has_kind(&self, kind: &str) -> bool {
        self.kinds.iter().any(|(k, _)| k == kind)
    }

    /// The kind of a report that names none: the first of [`Words::kinds`] ("bug" when there are none).
    pub fn default_kind(&self) -> &str {
        self.kinds.first().map(|(k, _)| k.as_str()).unwrap_or("bug")
    }

    /// The words of a kind; the first kind's for one the app doesn't know.
    pub fn kind_label<'a>(&'a self, kind: &'a str) -> &'a str {
        self.kinds.iter().find(|(k, _)| k == kind).or(self.kinds.first()).map(|(_, l)| l.as_str()).unwrap_or(kind)
    }

    /// The words of a status; "new"'s for one the app doesn't know.
    pub fn status_label<'a>(&'a self, status: &'a str) -> &'a str {
        let find = |s: &str| self.statuses.iter().find(|(k, _)| k == s).map(|(_, l)| l.as_str());
        find(status).or_else(|| find("new")).unwrap_or(status)
    }

    /// The export's file name without its extension: [`Words::export_file`] with the app's name in lower case,
    /// anything but letters and digits as `-`, then `-<date>` ("Demo" → `demo-bug-reports-2026-11-04`).
    pub fn export_stem(&self, app_name: &str, date: &str) -> String {
        let app: String = app_name.trim().chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
        format!("{}-{date}", fill(&self.export_file, &[("app", &app)]))
    }
}

/// `text` with each `{name}` of `values` filled in, in one pass: a value that itself holds `{…}` stays as it is.
pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let hit = after.find('}').and_then(|end| values.iter().find(|(k, _)| *k == &after[..end]).map(|(_, v)| (end, *v)));
        match hit {
            Some((end, v)) => {
                out.push_str(v);
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// "A, B or C".
pub(crate) fn or_list<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let items: Vec<&str> = items.collect();
    match items.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
        Some((last, _)) => last.to_string(),
        None => String::new(),
    }
}

/// Where the bug report routes are served: `base`, and `base/unread`, `base/export`, `base/{id}`,
/// `base/{id}/messages`, `base/{id}/status`, `base/{id}/files/{fid}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BugReportPaths {
    /// the routes' common start (default `/api/bug-reports`); also the start of each file's `url`
    pub base: String,
}

impl Default for BugReportPaths {
    fn default() -> Self {
        BugReportPaths { base: "/api/bug-reports".into() }
    }
}

/// Everything about the bug reports an app may change: where they are served, their limits and their words.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BugReportsConfig {
    /// the routes' paths
    pub paths: BugReportPaths,
    /// the limits
    pub limits: Limits,
    /// the words people see
    pub words: Words,
}

/// What the bug reports need from the app besides [`Host`]: where they are kept, where files go, the app's name
/// and version for the export, its rate limiter, and how it tells people about new reports and replies.
pub trait BugReportsHost: Host {
    /// Where the reports, their replies and the records of their files are kept: a `SqliteReports` or
    /// `PgReports` the app made once at start, or the app's own [`BugReportStore`].
    fn reports(&self) -> &dyn BugReportStore;

    /// The folder the reports' files are kept under (in `reports/`).
    fn data_dir(&self) -> PathBuf;

    /// The app's name: `{app}` in the export's heading and file name and in the reply mail's subject.
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

    /// Tells the admins something happened ([`Words::notify_new`], [`Words::notify_reply`]) however the app
    /// does (a chat webhook, a push message …). The future is spawned, so a slow notification never holds the
    /// request. Default: nothing.
    fn notify(&self, title: String, message: String) -> impl Future<Output = ()> + Send + 'static {
        let _ = (title, message);
        async {}
    }

    /// Whether the app can send mail; when it can, a reporter hears of an admin's reply by mail. Default: no.
    fn mail_enabled(&self) -> bool {
        false
    }

    /// The mail address of account `user_id` ("" for none): the app knows its accounts, the kit has no table of
    /// them. Default: "" (no reply mail is sent).
    fn mail_address(&self, user_id: i64) -> Result<String> {
        let _ = user_id;
        Ok(String::new())
    }

    /// The address the app is reached at (`https://app.example.com`), put before [`Words::report_link`] in a
    /// reply mail; "" when unknown (the link is then the report link alone). Default: "".
    fn base_url(&self) -> String {
        String::new()
    }

    /// Sends a plain text mail; the error in words when it fails (logged as "Bug report reply mail failed: …").
    /// The future is spawned. Default: mail isn't set up.
    fn send_mail(&self, to: String, subject: String, text: String) -> impl Future<Output = Result<(), String>> + Send + 'static {
        let _ = (to, subject, text);
        async { Err("mail isn't set up".to_string()) }
    }

    /// How many uploads are checked (their images decoded) at once: each may hold [`Limits::max_pixel_bytes`],
    /// so two at a time is what a small server can take. Default: two for the whole process.
    fn decodes(&self) -> Arc<Semaphore> {
        static DECODES: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(2)));
        DECODES.clone()
    }
}

/// The configuration the handlers are given.
pub(crate) type Cfg = Extension<Arc<BugReportsConfig>>;

/// The bug report routes with the default paths, limits and words.
pub fn router<S: BugReportsHost>() -> Router<S> {
    router_with(BugReportsConfig::default())
}

/// The bug report routes with the app's paths, limits and words.
pub fn router_with<S: BugReportsHost>(config: BugReportsConfig) -> Router<S> {
    use reports::{create, detail, file, list, remove, reply, set_status, unread};
    let base = config.paths.base.trim_end_matches('/').to_string();
    let upload = DefaultBodyLimit::max(config.limits.upload());
    Router::new()
        .route(&base, get(list::<S>).post(create::<S>).layer(upload))
        .route(&format!("{base}/unread"), get(unread::<S>))
        .route(&format!("{base}/export"), get(export::export::<S>))
        .route(&format!("{base}/{{id}}"), get(detail::<S>).delete(remove::<S>))
        .route(&format!("{base}/{{id}}/messages"), post(reply::<S>).layer(upload))
        .route(&format!("{base}/{{id}}/status"), post(set_status::<S>))
        .route(&format!("{base}/{{id}}/files/{{fid}}"), get(file::<S>))
        .layer(Extension(Arc::new(config)))
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

/// Bytes as whole MB for a sentence, rounded up (8 MiB → 8).
pub(crate) fn mb(bytes: i64) -> i64 {
    (bytes + (1 << 20) - 1) >> 20
}

/// Where the reports' files are kept.
fn reports_dir(st: &impl BugReportsHost) -> PathBuf {
    st.data_dir().join("reports")
}

#[cfg(test)]
mod tests;
