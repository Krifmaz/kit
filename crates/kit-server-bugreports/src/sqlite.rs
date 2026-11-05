//! [`SqliteReports`]: the bug reports and crash reports in SQLite (feature `sqlite`, the default), on the kit's
//! `SqliteDb` or the app's own SQLite database.

use crate::store::sql::{self, Params};
use crate::store::{
    BugReportStore, Crash, CrashStore, FileRow, ListFilter, Message, NewCrash, NewFile, NewMessage, NewReport, Report, ReportSummary, Side,
    BUG_REPORTS_SQLITE, CRASHES_SQLITE,
};
use kit_server_core::{async_trait, p, Db, Result, RowExt, SharedDb};
use std::sync::Arc;

/// The bug reports ([`BugReportStore`]) and crash reports ([`CrashStore`]) in SQLite, in `kit_bug_reports`,
/// `kit_bug_report_messages`, `kit_bug_report_files` and `kit_client_crashes`. Clones share the database.
#[derive(Clone)]
pub struct SqliteReports {
    /// the database: the kit's `SqliteDb` or the app's own SQLite `Db`
    db: SharedDb,
}

impl SqliteReports {
    /// The reports on `db` (the kit's `SqliteDb` or the app's own SQLite database), their tables made or brought up to date.
    pub fn new(db: impl Db + 'static) -> Result<SqliteReports> {
        BUG_REPORTS_SQLITE.run(&db)?;
        CRASHES_SQLITE.run(&db)?;
        Ok(SqliteReports { db: Arc::new(db) })
    }
}

#[async_trait]
impl BugReportStore for SqliteReports {
    async fn create_report(&self, r: &NewReport) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_bug_reports (user_id, username, kind, title, body, page, app_version, device, status, created_at, updated_at,
                                          last_reply_by, user_seen_at, admin_seen_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'new', ?, ?, '', ?, ?)",
            p![
                r.user_id,
                r.username,
                r.kind,
                r.title,
                r.body,
                r.page,
                r.app_version,
                r.device,
                r.created_at,
                r.created_at,
                r.created_at,
                r.admin_seen_at
            ],
        )
    }

    async fn report(&self, id: i64) -> Result<Option<Report>> {
        Ok(self.db.one("SELECT * FROM kit_bug_reports WHERE id = ?", p![id])?.map(|r| Report::from_row(&r)))
    }

    async fn list_reports(&self, filter: &ListFilter, side: Side) -> Result<Vec<ReportSummary>> {
        let mut p = Params::new(false);
        let rows = self.db.query(&sql::list(filter, side, &mut p), p.values)?;
        Ok(rows
            .iter()
            .map(|r| ReportSummary {
                report: Report::from_row(r),
                message_count: r.i("message_count"),
                file_count: r.i("file_count"),
                unread: r.b("unread"),
            })
            .collect())
    }

    async fn unread_count(&self, filter: &ListFilter, side: Side) -> Result<i64> {
        let mut p = Params::new(false);
        Ok(self.db.one(&sql::unread_count(filter, side, &mut p), p.values)?.map(|r| r.i("n")).unwrap_or(0))
    }

    async fn mark_seen(&self, id: i64, side: Side, at: f64) -> Result<()> {
        let column = sql::side(side).1;
        self.db.execute(&format!("UPDATE kit_bug_reports SET {column} = ? WHERE id = ?"), p![at, id])?;
        Ok(())
    }

    async fn set_status(&self, id: i64, status: &str, at: f64) -> Result<()> {
        self.db.execute("UPDATE kit_bug_reports SET status = ?, updated_at = ? WHERE id = ?", p![status, at, id])?;
        Ok(())
    }

    async fn record_reply(&self, id: i64, side: Side, status: &str, at: f64) -> Result<()> {
        let (by, column) = sql::side(side);
        self.db.execute(
            &format!("UPDATE kit_bug_reports SET last_reply_by = ?, status = ?, updated_at = ?, {column} = ? WHERE id = ?"),
            p![by, status, at, at, id],
        )?;
        Ok(())
    }

    async fn delete_report(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM kit_bug_report_files WHERE report_id = ?", p![id])?;
        self.db.execute("DELETE FROM kit_bug_report_messages WHERE report_id = ?", p![id])?;
        self.db.execute("DELETE FROM kit_bug_reports WHERE id = ?", p![id])?;
        Ok(())
    }

    async fn add_message(&self, m: &NewMessage) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_bug_report_messages (report_id, user_id, username, is_admin, body, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            p![m.report_id, m.user_id, m.username, m.is_admin, m.body, m.created_at],
        )
    }

    async fn messages(&self, report_id: i64) -> Result<Vec<Message>> {
        let rows = self.db.query("SELECT * FROM kit_bug_report_messages WHERE report_id = ? ORDER BY id", p![report_id])?;
        Ok(rows.iter().map(Message::from_row).collect())
    }

    async fn add_file(&self, f: &NewFile) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_bug_report_files (report_id, message_id, name, mime, size, path, created_at) VALUES (?, ?, ?, ?, ?, '', ?)",
            p![f.report_id, f.message_id, f.name, f.mime, f.size, f.created_at],
        )
    }

    async fn set_file_path(&self, id: i64, path: &str) -> Result<()> {
        self.db.execute("UPDATE kit_bug_report_files SET path = ? WHERE id = ?", p![path, id])?;
        Ok(())
    }

    async fn files(&self, report_id: i64) -> Result<Vec<FileRow>> {
        let rows = self.db.query("SELECT * FROM kit_bug_report_files WHERE report_id = ? ORDER BY id", p![report_id])?;
        Ok(rows.iter().map(FileRow::from_row).collect())
    }

    async fn file(&self, report_id: i64, file_id: i64) -> Result<Option<FileRow>> {
        let row = self.db.one("SELECT * FROM kit_bug_report_files WHERE id = ? AND report_id = ?", p![file_id, report_id])?;
        Ok(row.map(|r| FileRow::from_row(&r)))
    }

    async fn user_bytes(&self, user_id: i64) -> Result<i64> {
        self.export_bytes(&ListFilter { user_id: Some(user_id), ..ListFilter::default() }).await
    }

    async fn export_reports(&self, filter: &ListFilter) -> Result<Vec<Report>> {
        let mut p = Params::new(false);
        Ok(self.db.query(&sql::export(filter, &mut p), p.values)?.iter().map(Report::from_row).collect())
    }

    async fn export_bytes(&self, filter: &ListFilter) -> Result<i64> {
        let mut p = Params::new(false);
        Ok(self.db.one(&sql::bytes(filter, &mut p), p.values)?.map(|r| r.i("n")).unwrap_or(0))
    }
}

#[async_trait]
impl CrashStore for SqliteReports {
    async fn add_crash(&self, n: &NewCrash, keep: i64) -> Result<(i64, bool)> {
        if let Some(d) = &n.dedupe {
            if let Some(r) = self.db.one("SELECT id FROM kit_client_crashes WHERE dedupe = ?", p![d])? {
                return Ok((r.i("id"), true));
            }
        }
        let c = &n.crash;
        let id = match self.db.execute(
            "INSERT INTO kit_client_crashes (user_id, username, kind, device, device_id, app_version, os, time, received_at, first_line, text, dedupe)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            p![c.user_id, c.username, c.kind, c.device, c.device_id, c.app_version, c.os, c.time, c.received_at, c.first_line, c.text, n.dedupe],
        ) {
            Ok(id) => id,
            // The same crash sent twice at once: the other one is kept.
            Err(e) => match &n.dedupe {
                Some(d) => match self.db.one("SELECT id FROM kit_client_crashes WHERE dedupe = ?", p![d])? {
                    Some(r) => return Ok((r.i("id"), true)),
                    None => return Err(e),
                },
                None => return Err(e),
            },
        };
        self.db.execute(
            "DELETE FROM kit_client_crashes WHERE id NOT IN (SELECT id FROM kit_client_crashes ORDER BY id DESC LIMIT ?)",
            p![keep],
        )?;
        Ok((id, false))
    }

    async fn crashes(&self, limit: i64) -> Result<Vec<Crash>> {
        let rows = self.db.query("SELECT * FROM kit_client_crashes ORDER BY time DESC, id DESC LIMIT ?", p![limit])?;
        Ok(rows.iter().map(Crash::from_row).collect())
    }

    async fn clear_crashes(&self) -> Result<()> {
        self.db.execute("DELETE FROM kit_client_crashes", p![])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit_server_core::SqliteDb;

    #[tokio::test]
    async fn reports_keep_their_contract() {
        let store = SqliteReports::new(SqliteDb::memory()).unwrap();
        crate::store::reports_contract(&store).await;
    }

    #[tokio::test]
    async fn crashes_keep_their_contract() {
        let store = SqliteReports::new(SqliteDb::memory()).unwrap();
        crate::store::crash_contract(&store).await;
    }

    #[test]
    fn migrations_run_once() {
        let db = SqliteDb::memory();
        SqliteReports::new(db.clone()).unwrap();
        SqliteReports::new(db.clone()).unwrap();
        let n = db.one("SELECT COUNT(*) n FROM kit_migrations WHERE component IN ('bug-reports', 'crash-reports')", p![]).unwrap().unwrap();
        assert_eq!(n.i("n"), 2);
    }
}
