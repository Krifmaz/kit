//! Where the bug reports and crash reports are kept: the [`BugReportStore`] and [`CrashStore`] traits, the rows
//! they hand back, and the migrations of their `kit_*` tables.
//!
//! The kit has two ready implementations: `SqliteReports` (feature `sqlite`, the
//! default) and `PgReports` (feature `postgres`). Each implements both traits on the four
//! tables `kit_bug_reports`, `kit_bug_report_messages`, `kit_bug_report_files` and `kit_client_crashes`. An app
//! with another database implements the traits itself; `reports_contract` and `crash_contract` (feature
//! `testing`) say what an implementation must do.

use kit_server_core::{async_trait, Migrations, Result, Row, RowExt};
use serde_json::{json, Value};

/// A bug report as it is kept.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// the report's id
    pub id: i64,
    /// the account that filed it
    pub user_id: i64,
    /// that account's name when it filed it
    pub username: String,
    /// one of the app's kinds ([`crate::Words::kinds`])
    pub kind: String,
    /// the title, on one line
    pub title: String,
    /// what happened
    pub body: String,
    /// where in the app it happened
    pub page: String,
    /// the app's version the reporter ran
    pub app_version: String,
    /// the reporter's device
    pub device: String,
    /// one of [`crate::STATUSES`]
    pub status: String,
    /// when it was filed (seconds since 1970)
    pub created_at: f64,
    /// when it last changed: a reply or a status
    pub updated_at: f64,
    /// who wrote the last reply: `admin`, `user`, or "" before any reply
    pub last_reply_by: String,
    /// when the reporter last looked at it
    pub user_seen_at: f64,
    /// when an admin last looked at it
    pub admin_seen_at: f64,
}

impl Report {
    /// A report from a row of `kit_bug_reports` (SQLite and Postgres alike).
    pub(crate) fn from_row(r: &Row) -> Report {
        Report {
            id: r.i("id"),
            user_id: r.i("user_id"),
            username: r.s("username"),
            kind: r.s("kind"),
            title: r.s("title"),
            body: r.s("body"),
            page: r.s("page"),
            app_version: r.s("app_version"),
            device: r.s("device"),
            status: r.s("status"),
            created_at: r.f("created_at"),
            updated_at: r.f("updated_at"),
            last_reply_by: r.s("last_reply_by"),
            user_seen_at: r.f("user_seen_at"),
            admin_seen_at: r.f("admin_seen_at"),
        }
    }
}

/// A report to keep. It starts `new`, with no reply, `updated_at` and `user_seen_at` at `created_at`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewReport {
    /// the account that files it
    pub user_id: i64,
    /// that account's name
    pub username: String,
    /// its kind
    pub kind: String,
    /// its title
    pub title: String,
    /// what happened
    pub body: String,
    /// where in the app
    pub page: String,
    /// the app's version
    pub app_version: String,
    /// the device
    pub device: String,
    /// when it is filed
    pub created_at: f64,
    /// when an admin has seen it: 0 for news, `created_at` for an admin's own report
    pub admin_seen_at: f64,
}

/// A report in a list: the report with how many replies and files it has and whether the viewer has seen all of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReportSummary {
    /// the report
    pub report: Report,
    /// its replies
    pub message_count: i64,
    /// its files, the replies' among them
    pub file_count: i64,
    /// something in it the viewer hasn't seen (see [`Side`])
    pub unread: bool,
}

/// A reply in a report's thread.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Message {
    /// the reply's id
    pub id: i64,
    /// the report it belongs to
    pub report_id: i64,
    /// who wrote it
    pub user_id: i64,
    /// that account's name
    pub username: String,
    /// whether an admin wrote it
    pub is_admin: bool,
    /// its text (may be empty when it carries files)
    pub body: String,
    /// when it was written
    pub created_at: f64,
}

impl Message {
    /// A reply from a row of `kit_bug_report_messages`.
    pub(crate) fn from_row(r: &Row) -> Message {
        Message {
            id: r.i("id"),
            report_id: r.i("report_id"),
            user_id: r.i("user_id"),
            username: r.s("username"),
            is_admin: r.b("is_admin"),
            body: r.s("body"),
            created_at: r.f("created_at"),
        }
    }
}

/// A reply to keep.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewMessage {
    /// the report
    pub report_id: i64,
    /// who writes it
    pub user_id: i64,
    /// that account's name
    pub username: String,
    /// whether an admin writes it
    pub is_admin: bool,
    /// its text
    pub body: String,
    /// when
    pub created_at: f64,
}

/// An attached file as it is kept. The bytes are on disk at `<data_dir>/reports/<path>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileRow {
    /// the file's id
    pub id: i64,
    /// the report it belongs to
    pub report_id: i64,
    /// the reply it came with; none for a file of the report itself
    pub message_id: Option<i64>,
    /// its name, made safe
    pub name: String,
    /// its type
    pub mime: String,
    /// its size in bytes
    pub size: i64,
    /// where it is under `reports/` (`<report_id>/<file_id>.<ext>`); "" until it is written
    pub path: String,
    /// when it was attached
    pub created_at: f64,
}

impl FileRow {
    /// A file from a row of `kit_bug_report_files`.
    pub(crate) fn from_row(r: &Row) -> FileRow {
        let message_id = match r.get("message_id") {
            None | Some(Value::Null) => None,
            Some(_) => Some(r.i("message_id")),
        };
        FileRow {
            id: r.i("id"),
            report_id: r.i("report_id"),
            message_id,
            name: r.s("name"),
            mime: r.s("mime"),
            size: r.i("size"),
            path: r.s("path"),
            created_at: r.f("created_at"),
        }
    }
}

/// A file to record before it is written (its path is set once its id is known).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewFile {
    /// the report
    pub report_id: i64,
    /// the reply it comes with, if any
    pub message_id: Option<i64>,
    /// its name
    pub name: String,
    /// its type
    pub mime: String,
    /// its size in bytes
    pub size: i64,
    /// when
    pub created_at: f64,
}

/// Which reports a list, a count or an export takes. Every part that is set must match; the default takes all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListFilter {
    /// only this status
    pub status: Option<String>,
    /// only this kind
    pub kind: Option<String>,
    /// only reports whose title, text or reporter's name holds this text (letters in any case)
    pub search: Option<String>,
    /// only this account's reports
    pub user_id: Option<i64>,
    /// only these reports (none: any)
    pub ids: Vec<i64>,
}

/// Whose side of a report: an admin's or the reporter's. A report is unread for an admin when no admin has
/// looked at it yet or the reporter replied since; for the reporter when an admin replied since their last look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// the admins
    Admin,
    /// the account that filed the report
    Reporter,
}

/// Where the bug reports, their replies and their files are kept.
#[async_trait]
pub trait BugReportStore: Send + Sync {
    /// Keep a new report; its id.
    async fn create_report(&self, report: &NewReport) -> Result<i64>;
    /// The report `id`, if there is one.
    async fn report(&self, id: i64) -> Result<Option<Report>>;
    /// The reports `filter` takes, the last changed first, with their counts and whether `side` has seen them.
    async fn list_reports(&self, filter: &ListFilter, side: Side) -> Result<Vec<ReportSummary>>;
    /// How many of the reports `filter` takes have something `side` hasn't seen.
    async fn unread_count(&self, filter: &ListFilter, side: Side) -> Result<i64>;
    /// Record that `side` looked at report `id` at `at`.
    async fn mark_seen(&self, id: i64, side: Side, at: f64) -> Result<()>;
    /// Set the status of report `id`, changed at `at`.
    async fn set_status(&self, id: i64, status: &str, at: f64) -> Result<()>;
    /// Record that `side` replied to report `id` at `at`: its last reply is `side`'s, its status is `status`, and
    /// `side` has seen it.
    async fn record_reply(&self, id: i64, side: Side, status: &str, at: f64) -> Result<()>;
    /// Delete report `id` with its replies and the records of its files (the files on disk are the caller's).
    async fn delete_report(&self, id: i64) -> Result<()>;
    /// Keep a reply; its id.
    async fn add_message(&self, message: &NewMessage) -> Result<i64>;
    /// The replies to report `id`, oldest first.
    async fn messages(&self, report_id: i64) -> Result<Vec<Message>>;
    /// Record a file; its id.
    async fn add_file(&self, file: &NewFile) -> Result<i64>;
    /// Set where file `id` was written.
    async fn set_file_path(&self, id: i64, path: &str) -> Result<()>;
    /// The files of report `report_id`, its replies' among them, oldest first.
    async fn files(&self, report_id: i64) -> Result<Vec<FileRow>>;
    /// File `file_id` of report `report_id`; none when it is another report's.
    async fn file(&self, report_id: i64, file_id: i64) -> Result<Option<FileRow>>;
    /// The bytes of the files the reports of account `user_id` hold.
    async fn user_bytes(&self, user_id: i64) -> Result<i64>;
    /// The reports `filter` takes, oldest (lowest id) first, for an export.
    async fn export_reports(&self, filter: &ListFilter) -> Result<Vec<Report>>;
    /// The bytes of the files the reports `filter` takes hold.
    async fn export_bytes(&self, filter: &ListFilter) -> Result<i64>;
}

/// A crash report from an app, as it is kept.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Crash {
    /// its id
    pub id: i64,
    /// the account that sent it
    pub user_id: i64,
    /// that account's name
    pub username: String,
    /// what happened (`crash`, `anr` …)
    pub kind: String,
    /// the device's name
    pub device: String,
    /// the device's id
    pub device_id: String,
    /// the app's version
    pub app_version: String,
    /// the operating system
    pub os: String,
    /// when it happened
    pub time: f64,
    /// when it arrived
    pub received_at: f64,
    /// the line shown in the list
    pub first_line: String,
    /// the stack trace and log
    pub text: String,
}

impl Crash {
    /// A crash report from a row of `kit_client_crashes`.
    pub(crate) fn from_row(r: &Row) -> Crash {
        Crash {
            id: r.i("id"),
            user_id: r.i("user_id"),
            username: r.s("username"),
            kind: r.s("kind"),
            device: r.s("device"),
            device_id: r.s("device_id"),
            app_version: r.s("app_version"),
            os: r.s("os"),
            time: r.f("time"),
            received_at: r.f("received_at"),
            first_line: r.s("first_line"),
            text: r.s("text"),
        }
    }
}

/// A crash report to keep: the report (its `id` is ignored) and, when the app sent a key, the text that makes
/// the same crash sent twice one report.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NewCrash {
    /// the report
    pub crash: Crash,
    /// `<user id>|<device id>|<key>`, or none
    pub dedupe: Option<String>,
}

/// Where the crash reports apps upload are kept.
#[async_trait]
pub trait CrashStore: Send + Sync {
    /// Keep a crash report and drop the oldest past the newest `keep`; (its id, false). When a report with the
    /// same `dedupe` is already kept: (that report's id, true), and nothing changes.
    async fn add_crash(&self, crash: &NewCrash, keep: i64) -> Result<(i64, bool)>;
    /// The newest `limit` crash reports, by when they happened.
    async fn crashes(&self, limit: i64) -> Result<Vec<Crash>>;
    /// Delete every crash report.
    async fn clear_crashes(&self) -> Result<()>;
}

/// The SQLite steps of the tables `kit_bug_reports`, `kit_bug_report_messages` and `kit_bug_report_files`.
pub const BUG_REPORTS_SQLITE: Migrations = Migrations {
    component: "bug-reports",
    steps: &["CREATE TABLE kit_bug_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            page TEXT NOT NULL DEFAULT '',
            app_version TEXT NOT NULL DEFAULT '',
            device TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'new',
            created_at REAL NOT NULL DEFAULT 0,
            updated_at REAL NOT NULL DEFAULT 0,
            last_reply_by TEXT NOT NULL DEFAULT '',
            user_seen_at REAL NOT NULL DEFAULT 0,
            admin_seen_at REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_bug_report_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            report_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            is_admin INTEGER NOT NULL DEFAULT 0,
            body TEXT NOT NULL DEFAULT '',
            created_at REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_bug_report_files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            report_id INTEGER NOT NULL,
            message_id INTEGER,
            name TEXT NOT NULL DEFAULT '',
            mime TEXT NOT NULL DEFAULT '',
            size INTEGER NOT NULL DEFAULT 0,
            path TEXT NOT NULL DEFAULT '',
            created_at REAL NOT NULL DEFAULT 0
        );
        CREATE INDEX kit_bug_reports_user ON kit_bug_reports(user_id);
        CREATE INDEX kit_bug_report_messages_report ON kit_bug_report_messages(report_id);
        CREATE INDEX kit_bug_report_files_report ON kit_bug_report_files(report_id);"],
};

/// The Postgres steps of the tables `kit_bug_reports`, `kit_bug_report_messages` and `kit_bug_report_files`.
pub const BUG_REPORTS_POSTGRES: Migrations = Migrations {
    component: "bug-reports",
    steps: &["CREATE TABLE kit_bug_reports (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            body TEXT NOT NULL DEFAULT '',
            page TEXT NOT NULL DEFAULT '',
            app_version TEXT NOT NULL DEFAULT '',
            device TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'new',
            created_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            updated_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            last_reply_by TEXT NOT NULL DEFAULT '',
            user_seen_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            admin_seen_at DOUBLE PRECISION NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_bug_report_messages (
            id BIGSERIAL PRIMARY KEY,
            report_id BIGINT NOT NULL,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            is_admin BOOLEAN NOT NULL DEFAULT FALSE,
            body TEXT NOT NULL DEFAULT '',
            created_at DOUBLE PRECISION NOT NULL DEFAULT 0
        );
        CREATE TABLE kit_bug_report_files (
            id BIGSERIAL PRIMARY KEY,
            report_id BIGINT NOT NULL,
            message_id BIGINT,
            name TEXT NOT NULL DEFAULT '',
            mime TEXT NOT NULL DEFAULT '',
            size BIGINT NOT NULL DEFAULT 0,
            path TEXT NOT NULL DEFAULT '',
            created_at DOUBLE PRECISION NOT NULL DEFAULT 0
        );
        CREATE INDEX kit_bug_reports_user ON kit_bug_reports(user_id);
        CREATE INDEX kit_bug_report_messages_report ON kit_bug_report_messages(report_id);
        CREATE INDEX kit_bug_report_files_report ON kit_bug_report_files(report_id);"],
};

/// The SQLite steps of the crash report table `kit_client_crashes`.
pub const CRASHES_SQLITE: Migrations = Migrations {
    component: "crash-reports",
    steps: &["CREATE TABLE kit_client_crashes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT '',
            device TEXT NOT NULL DEFAULT '',
            device_id TEXT NOT NULL DEFAULT '',
            app_version TEXT NOT NULL DEFAULT '',
            os TEXT NOT NULL DEFAULT '',
            time REAL NOT NULL DEFAULT 0,
            received_at REAL NOT NULL DEFAULT 0,
            first_line TEXT NOT NULL DEFAULT '',
            text TEXT NOT NULL DEFAULT '',
            dedupe TEXT
        );
        CREATE UNIQUE INDEX kit_client_crashes_dedupe ON kit_client_crashes(dedupe);"],
};

/// The Postgres steps of the crash report table `kit_client_crashes`.
pub const CRASHES_POSTGRES: Migrations = Migrations {
    component: "crash-reports",
    steps: &["CREATE TABLE kit_client_crashes (
            id BIGSERIAL PRIMARY KEY,
            user_id BIGINT NOT NULL,
            username TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT '',
            device TEXT NOT NULL DEFAULT '',
            device_id TEXT NOT NULL DEFAULT '',
            app_version TEXT NOT NULL DEFAULT '',
            os TEXT NOT NULL DEFAULT '',
            time DOUBLE PRECISION NOT NULL DEFAULT 0,
            received_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            first_line TEXT NOT NULL DEFAULT '',
            text TEXT NOT NULL DEFAULT '',
            dedupe TEXT
        );
        CREATE UNIQUE INDEX kit_client_crashes_dedupe ON kit_client_crashes(dedupe);"],
};

/// The statements both SQL stores share, written once with the placeholders of either database.
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub(crate) mod sql {
    use super::{ListFilter, Side};
    use serde_json::Value;

    /// A statement's parameters as they are added, with the placeholder for each: `?` for SQLite, `$1, $2 …`
    /// for Postgres.
    pub(crate) struct Params {
        /// Postgres placeholders
        pg: bool,
        /// the parameters so far
        pub(crate) values: Vec<Value>,
    }

    impl Params {
        /// No parameters yet, for SQLite (`pg` false) or Postgres.
        pub(crate) fn new(pg: bool) -> Params {
            Params { pg, values: Vec::new() }
        }

        /// Add `v`; its placeholder.
        pub(crate) fn bind(&mut self, v: impl Into<Value>) -> String {
            self.values.push(v.into());
            if self.pg {
                format!("${}", self.values.len())
            } else {
                "?".into()
            }
        }
    }

    /// `WHERE …` of the reports (as `r`) `f` takes.
    pub(crate) fn report_where(f: &ListFilter, p: &mut Params) -> String {
        let mut parts = vec!["1=1".to_string()];
        if let Some(s) = &f.status {
            parts.push(format!("r.status = {}", p.bind(s.as_str())));
        }
        if let Some(k) = &f.kind {
            parts.push(format!("r.kind = {}", p.bind(k.as_str())));
        }
        if let Some(s) = f.search.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            let like = format!("%{}%", s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
            // SQLite's LIKE ignores the case of ASCII letters; Postgres' ILIKE of all
            let op = if p.pg { "ILIKE" } else { "LIKE" };
            let (a, b, c) = (p.bind(like.as_str()), p.bind(like.as_str()), p.bind(like.as_str()));
            parts.push(format!("(r.title {op} {a} ESCAPE '\\' OR r.body {op} {b} ESCAPE '\\' OR r.username {op} {c} ESCAPE '\\')"));
        }
        if let Some(u) = f.user_id {
            parts.push(format!("r.user_id = {}", p.bind(u)));
        }
        if !f.ids.is_empty() {
            let marks: Vec<String> = f.ids.iter().map(|id| p.bind(*id)).collect();
            parts.push(format!("r.id IN ({})", marks.join(", ")));
        }
        parts.join(" AND ")
    }

    /// When the report's (`r`) last reply came; 0 without one.
    const LAST_MESSAGE: &str = "COALESCE((SELECT MAX(m.created_at) FROM kit_bug_report_messages m WHERE m.report_id = r.id), 0)";

    /// "The report (`r`) has something `side` hasn't seen" (see [`Side`]).
    pub(crate) fn unread(side: Side) -> String {
        match side {
            Side::Admin => format!("(r.admin_seen_at < r.created_at OR (r.last_reply_by = 'user' AND {LAST_MESSAGE} > r.admin_seen_at))"),
            Side::Reporter => format!("(r.last_reply_by = 'admin' AND {LAST_MESSAGE} > r.user_seen_at)"),
        }
    }

    /// The list of the reports `f` takes, the last changed first.
    pub(crate) fn list(f: &ListFilter, side: Side, p: &mut Params) -> String {
        format!(
            "SELECT r.*,
                    (SELECT COUNT(*) FROM kit_bug_report_messages m WHERE m.report_id = r.id) AS message_count,
                    (SELECT COUNT(*) FROM kit_bug_report_files f WHERE f.report_id = r.id) AS file_count,
                    {} AS unread
             FROM kit_bug_reports r WHERE {} ORDER BY r.updated_at DESC, r.id DESC",
            unread(side),
            report_where(f, p)
        )
    }

    /// How many of the reports `f` takes `side` hasn't seen all of, as `n`.
    pub(crate) fn unread_count(f: &ListFilter, side: Side, p: &mut Params) -> String {
        format!("SELECT COUNT(*) AS n FROM kit_bug_reports r WHERE {} AND {}", unread(side), report_where(f, p))
    }

    /// The reports `f` takes, by id.
    pub(crate) fn export(f: &ListFilter, p: &mut Params) -> String {
        format!("SELECT r.* FROM kit_bug_reports r WHERE {} ORDER BY r.id", report_where(f, p))
    }

    /// The bytes of the files of the reports `f` takes, as `n` (a whole number in either database).
    pub(crate) fn bytes(f: &ListFilter, p: &mut Params) -> String {
        format!(
            "SELECT CAST(COALESCE(SUM(f.size), 0) AS BIGINT) AS n FROM kit_bug_report_files f JOIN kit_bug_reports r ON r.id = f.report_id WHERE {}",
            report_where(f, p)
        )
    }

    /// What `last_reply_by` holds after a reply by `side`, and the column of `side`'s last look.
    pub(crate) fn side(side: Side) -> (&'static str, &'static str) {
        match side {
            Side::Admin => ("admin", "admin_seen_at"),
            Side::Reporter => ("user", "user_seen_at"),
        }
    }
}

/// A crash report as the API lists it.
pub(crate) fn crash_json(c: &Crash) -> Value {
    json!({
        "id": c.id, "kind": c.kind, "user": c.username, "device": c.device, "device_id": c.device_id, "app_version": c.app_version,
        "os": c.os, "time": c.time, "received_at": c.received_at, "first_line": c.first_line, "text": c.text,
    })
}

/// What every [`BugReportStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
#[cfg(any(test, feature = "testing"))]
pub async fn reports_contract(store: &dyn BugReportStore) {
    let new = |user_id: i64, username: &str, kind: &str, title: &str, body: &str, at: f64| NewReport {
        user_id,
        username: username.into(),
        kind: kind.into(),
        title: title.into(),
        body: body.into(),
        page: "#/items/3".into(),
        app_version: "1.2.0".into(),
        device: "Phone".into(),
        created_at: at,
        admin_seen_at: 0.0,
    };
    let all = ListFilter::default();
    let a = store.create_report(&new(2, "ana", "bug", "Save button does nothing", "Nothing happened.", 100.0)).await.unwrap();
    let b = store.create_report(&new(3, "other", "howto", "Where are my notes?", "50_50 chance", 110.0)).await.unwrap();
    assert!(b > a);
    let r = store.report(a).await.unwrap().unwrap();
    let want = Report {
        id: a,
        user_id: 2,
        username: "ana".into(),
        kind: "bug".into(),
        title: "Save button does nothing".into(),
        body: "Nothing happened.".into(),
        page: "#/items/3".into(),
        app_version: "1.2.0".into(),
        device: "Phone".into(),
        status: "new".into(),
        created_at: 100.0,
        updated_at: 100.0,
        last_reply_by: String::new(),
        user_seen_at: 100.0,
        admin_seen_at: 0.0,
    };
    assert_eq!(r, want);
    assert_eq!(store.report(b + 100).await.unwrap(), None);

    // lists: the last changed first; each filter
    let ids = |l: Vec<ReportSummary>| l.into_iter().map(|s| s.report.id).collect::<Vec<_>>();
    assert_eq!(ids(store.list_reports(&all, Side::Admin).await.unwrap()), vec![b, a]);
    assert_eq!(ids(store.list_reports(&ListFilter { status: Some("new".into()), ..all.clone() }, Side::Admin).await.unwrap()), vec![b, a]);
    assert_eq!(
        ids(store.list_reports(&ListFilter { status: Some("fixed".into()), ..all.clone() }, Side::Admin).await.unwrap()),
        Vec::<i64>::new()
    );
    assert_eq!(ids(store.list_reports(&ListFilter { kind: Some("howto".into()), ..all.clone() }, Side::Admin).await.unwrap()), vec![b]);
    assert_eq!(ids(store.list_reports(&ListFilter { search: Some("SAVE".into()), ..all.clone() }, Side::Admin).await.unwrap()), vec![a]);
    assert_eq!(ids(store.list_reports(&ListFilter { search: Some("OTHER".into()), ..all.clone() }, Side::Admin).await.unwrap()), vec![b]);
    assert_eq!(ids(store.list_reports(&ListFilter { search: Some("50_".into()), ..all.clone() }, Side::Admin).await.unwrap()), vec![b]);
    assert_eq!(
        ids(store.list_reports(&ListFilter { search: Some("0%".into()), ..all.clone() }, Side::Admin).await.unwrap()),
        Vec::<i64>::new()
    );
    assert_eq!(
        ids(store.list_reports(&ListFilter { search: Some("e_s".into()), ..all.clone() }, Side::Admin).await.unwrap()),
        Vec::<i64>::new()
    );
    assert_eq!(ids(store.list_reports(&ListFilter { user_id: Some(3), ..all.clone() }, Side::Reporter).await.unwrap()), vec![b]);
    assert_eq!(ids(store.list_reports(&ListFilter { ids: vec![a, b + 100], ..all.clone() }, Side::Admin).await.unwrap()), vec![a]);

    // unread: new reports are news to the admins, not to their reporters
    let mine = ListFilter { user_id: Some(2), ..all.clone() };
    assert_eq!(store.unread_count(&all, Side::Admin).await.unwrap(), 2);
    assert_eq!(store.unread_count(&mine, Side::Reporter).await.unwrap(), 0);
    store.mark_seen(a, Side::Admin, 120.0).await.unwrap();
    assert_eq!(store.unread_count(&all, Side::Admin).await.unwrap(), 1);
    assert_eq!(store.report(a).await.unwrap().unwrap().admin_seen_at, 120.0);

    // an admin's reply: answered, unread for the reporter
    let m1 = NewMessage { report_id: a, user_id: 1, username: "admin".into(), is_admin: true, body: "Looking.".into(), created_at: 130.0 };
    let m1 = store.add_message(&m1).await.unwrap();
    store.record_reply(a, Side::Admin, "answered", 130.0).await.unwrap();
    let r = store.report(a).await.unwrap().unwrap();
    assert_eq!((r.status.as_str(), r.last_reply_by.as_str(), r.updated_at, r.admin_seen_at), ("answered", "admin", 130.0, 130.0));
    assert_eq!(store.unread_count(&mine, Side::Reporter).await.unwrap(), 1);
    let listed = store.list_reports(&mine, Side::Reporter).await.unwrap();
    assert_eq!((listed[0].message_count, listed[0].unread), (1, true));
    store.mark_seen(a, Side::Reporter, 140.0).await.unwrap();
    assert_eq!(store.unread_count(&mine, Side::Reporter).await.unwrap(), 0);
    assert_eq!(store.report(a).await.unwrap().unwrap().user_seen_at, 140.0);

    // the reporter's reply with a file: unread for the admins again
    let m2 = NewMessage { report_id: a, user_id: 2, username: "ana".into(), is_admin: false, body: String::new(), created_at: 150.0 };
    let m2 = store.add_message(&m2).await.unwrap();
    store.record_reply(a, Side::Reporter, "new", 150.0).await.unwrap();
    let r = store.report(a).await.unwrap().unwrap();
    assert_eq!((r.status.as_str(), r.last_reply_by.as_str(), r.user_seen_at), ("new", "user", 150.0));
    assert_eq!(store.unread_count(&ListFilter { ids: vec![a], ..all.clone() }, Side::Admin).await.unwrap(), 1);
    let messages = store.messages(a).await.unwrap();
    assert_eq!(messages.iter().map(|m| m.id).collect::<Vec<_>>(), vec![m1, m2]);
    assert_eq!(
        messages[0],
        Message { id: m1, report_id: a, user_id: 1, username: "admin".into(), is_admin: true, body: "Looking.".into(), created_at: 130.0 }
    );
    assert!(!messages[1].is_admin);
    assert!(store.messages(b).await.unwrap().is_empty());

    // files: recorded, then their path set
    let file = |message_id: Option<i64>, name: &str, size: i64| NewFile {
        report_id: a,
        message_id,
        name: name.into(),
        mime: "text/plain".into(),
        size,
        created_at: 160.0,
    };
    let f1 = store.add_file(&file(None, "shot.png", 1000)).await.unwrap();
    let f2 = store.add_file(&file(Some(m2), "console.log", 24)).await.unwrap();
    store.set_file_path(f1, &format!("{a}/{f1}.png")).await.unwrap();
    let files = store.files(a).await.unwrap();
    assert_eq!(files.iter().map(|f| f.id).collect::<Vec<_>>(), vec![f1, f2]);
    assert_eq!(
        files[0],
        FileRow {
            id: f1,
            report_id: a,
            message_id: None,
            name: "shot.png".into(),
            mime: "text/plain".into(),
            size: 1000,
            path: format!("{a}/{f1}.png"),
            created_at: 160.0
        }
    );
    assert_eq!((files[1].message_id, files[1].path.as_str()), (Some(m2), ""));
    assert_eq!(store.file(a, f2).await.unwrap().unwrap().name, "console.log");
    assert_eq!(store.file(b, f2).await.unwrap(), None);
    assert_eq!(store.user_bytes(2).await.unwrap(), 1024);
    assert_eq!(store.user_bytes(3).await.unwrap(), 0);
    assert_eq!(store.list_reports(&mine, Side::Reporter).await.unwrap()[0].file_count, 2);

    // export: by id; the bytes the files hold
    assert_eq!(store.export_reports(&all).await.unwrap().iter().map(|r| r.id).collect::<Vec<_>>(), vec![a, b]);
    assert_eq!(store.export_reports(&ListFilter { kind: Some("howto".into()), ..all.clone() }).await.unwrap()[0].id, b);
    assert_eq!(store.export_bytes(&all).await.unwrap(), 1024);
    assert_eq!(store.export_bytes(&ListFilter { ids: vec![b], ..all.clone() }).await.unwrap(), 0);

    // status, then delete: the replies and files go with the report
    store.set_status(b, "fixed", 170.0).await.unwrap();
    let r = store.report(b).await.unwrap().unwrap();
    assert_eq!((r.status.as_str(), r.updated_at), ("fixed", 170.0));
    store.delete_report(a).await.unwrap();
    assert_eq!(store.report(a).await.unwrap(), None);
    assert!(store.messages(a).await.unwrap().is_empty() && store.files(a).await.unwrap().is_empty());
    assert_eq!(store.export_bytes(&all).await.unwrap(), 0);
    assert_eq!(ids(store.list_reports(&all, Side::Admin).await.unwrap()), vec![b]);
}

/// What every [`CrashStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
#[cfg(any(test, feature = "testing"))]
pub async fn crash_contract(store: &dyn CrashStore) {
    let crash = |text: &str, time: f64, key: Option<&str>| NewCrash {
        crash: Crash {
            id: 0,
            user_id: 2,
            username: "ana".into(),
            kind: "crash".into(),
            device: "Phone".into(),
            device_id: "dev-1".into(),
            app_version: "1.2.0".into(),
            os: "Android 15".into(),
            time,
            received_at: 500.0,
            first_line: text.into(),
            text: format!("{text}\n\tat Demo.main"),
        },
        dedupe: key.map(|k| format!("2|dev-1|{k}")),
    };
    assert!(store.crashes(10).await.unwrap().is_empty());
    let (a, dup) = store.add_crash(&crash("Boom", 300.0, Some("k1")), 3).await.unwrap();
    assert!(!dup);
    assert_eq!(store.add_crash(&crash("Boom again", 301.0, Some("k1")), 3).await.unwrap(), (a, true));
    let (b, _) = store.add_crash(&crash("Later", 400.0, None), 3).await.unwrap();
    let (c, _) = store.add_crash(&crash("Earlier", 200.0, None), 3).await.unwrap();
    let list = store.crashes(10).await.unwrap();
    assert_eq!(list.iter().map(|c| c.id).collect::<Vec<_>>(), vec![b, a, c]);
    let mut want = crash("Boom", 300.0, None).crash;
    want.id = a;
    assert_eq!(list[1], want);
    assert_eq!(store.crashes(1).await.unwrap()[0].id, b);
    // past `keep` the oldest kept goes first
    let (d, _) = store.add_crash(&crash("Fourth", 100.0, Some("k2")), 3).await.unwrap();
    let ids: Vec<i64> = store.crashes(10).await.unwrap().iter().map(|c| c.id).collect();
    assert_eq!(ids, vec![b, c, d]);
    // a dropped report's key is free again
    assert!(!store.add_crash(&crash("Boom", 300.0, Some("k1")), 3).await.unwrap().1);
    store.clear_crashes().await.unwrap();
    assert!(store.crashes(10).await.unwrap().is_empty());
}
