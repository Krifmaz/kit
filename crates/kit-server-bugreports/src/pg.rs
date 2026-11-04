//! [`PgReports`]: the bug reports and crash reports in Postgres (feature `postgres`), on the kit's [`PgDb`].

use crate::store::sql::{self, Params};
use crate::store::{
    BugReportStore, Crash, CrashStore, FileRow, ListFilter, Message, NewCrash, NewFile, NewMessage, NewReport, Report, ReportSummary, Side,
    BUG_REPORTS_POSTGRES, CRASHES_POSTGRES,
};
use kit_server_core::pg::PgDb;
use kit_server_core::{async_trait, p, Result, RowExt};

/// The bug reports ([`BugReportStore`]) and crash reports ([`CrashStore`]) in Postgres, in `kit_bug_reports`,
/// `kit_bug_report_messages`, `kit_bug_report_files` and `kit_client_crashes`. Clones share the pool.
#[derive(Clone)]
pub struct PgReports {
    /// the database
    db: PgDb,
}

impl PgReports {
    /// The reports on `db`, their tables made or brought up to date.
    pub async fn new(db: PgDb) -> Result<PgReports> {
        db.migrate(&BUG_REPORTS_POSTGRES).await?;
        db.migrate(&CRASHES_POSTGRES).await?;
        Ok(PgReports { db })
    }

    /// Runs an `INSERT … RETURNING id`; the new row's id.
    async fn insert(&self, sql: &str, params: Vec<serde_json::Value>) -> Result<i64> {
        let row = self.db.one(sql, params).await?.ok_or_else(|| kit_server_core::err(500, "Database error"))?;
        Ok(row.i("id"))
    }
}

#[async_trait]
impl BugReportStore for PgReports {
    async fn create_report(&self, r: &NewReport) -> Result<i64> {
        self.insert(
            "INSERT INTO kit_bug_reports (user_id, username, kind, title, body, page, app_version, device, status, created_at, updated_at,
                                          last_reply_by, user_seen_at, admin_seen_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'new', $9::DOUBLE PRECISION, $10::DOUBLE PRECISION, '', $11::DOUBLE PRECISION, $12::DOUBLE PRECISION)
             RETURNING id",
            p![r.user_id, r.username, r.kind, r.title, r.body, r.page, r.app_version, r.device, r.created_at, r.created_at, r.created_at, r.admin_seen_at],
        ).await
    }

    async fn report(&self, id: i64) -> Result<Option<Report>> {
        Ok(self.db.one("SELECT * FROM kit_bug_reports WHERE id = $1", p![id]).await?.map(|r| Report::from_row(&r)))
    }

    async fn list_reports(&self, filter: &ListFilter, side: Side) -> Result<Vec<ReportSummary>> {
        let mut p = Params::new(true);
        let rows = self.db.query(&sql::list(filter, side, &mut p), p.values).await?;
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
        let mut p = Params::new(true);
        Ok(self.db.one(&sql::unread_count(filter, side, &mut p), p.values).await?.map(|r| r.i("n")).unwrap_or(0))
    }

    async fn mark_seen(&self, id: i64, side: Side, at: f64) -> Result<()> {
        let column = sql::side(side).1;
        self.db.execute(&format!("UPDATE kit_bug_reports SET {column} = $1::DOUBLE PRECISION WHERE id = $2"), p![at, id]).await?;
        Ok(())
    }

    async fn set_status(&self, id: i64, status: &str, at: f64) -> Result<()> {
        self.db
            .execute("UPDATE kit_bug_reports SET status = $1, updated_at = $2::DOUBLE PRECISION WHERE id = $3", p![status, at, id])
            .await?;
        Ok(())
    }

    async fn record_reply(&self, id: i64, side: Side, status: &str, at: f64) -> Result<()> {
        let (by, column) = sql::side(side);
        self.db.execute(
            &format!("UPDATE kit_bug_reports SET last_reply_by = $1, status = $2, updated_at = $3::DOUBLE PRECISION, {column} = $4::DOUBLE PRECISION WHERE id = $5"),
            p![by, status, at, at, id],
        ).await?;
        Ok(())
    }

    async fn delete_report(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM kit_bug_report_files WHERE report_id = $1", p![id]).await?;
        self.db.execute("DELETE FROM kit_bug_report_messages WHERE report_id = $1", p![id]).await?;
        self.db.execute("DELETE FROM kit_bug_reports WHERE id = $1", p![id]).await?;
        Ok(())
    }

    async fn add_message(&self, m: &NewMessage) -> Result<i64> {
        self.insert(
            "INSERT INTO kit_bug_report_messages (report_id, user_id, username, is_admin, body, created_at) VALUES ($1, $2, $3, $4, $5, $6::DOUBLE PRECISION) RETURNING id",
            p![m.report_id, m.user_id, m.username, m.is_admin, m.body, m.created_at],
        ).await
    }

    async fn messages(&self, report_id: i64) -> Result<Vec<Message>> {
        let rows = self.db.query("SELECT * FROM kit_bug_report_messages WHERE report_id = $1 ORDER BY id", p![report_id]).await?;
        Ok(rows.iter().map(Message::from_row).collect())
    }

    async fn add_file(&self, f: &NewFile) -> Result<i64> {
        self.insert(
            "INSERT INTO kit_bug_report_files (report_id, message_id, name, mime, size, path, created_at) VALUES ($1, $2::BIGINT, $3, $4, $5, '', $6::DOUBLE PRECISION) RETURNING id",
            p![f.report_id, f.message_id, f.name, f.mime, f.size, f.created_at],
        ).await
    }

    async fn set_file_path(&self, id: i64, path: &str) -> Result<()> {
        self.db.execute("UPDATE kit_bug_report_files SET path = $1 WHERE id = $2", p![path, id]).await?;
        Ok(())
    }

    async fn files(&self, report_id: i64) -> Result<Vec<FileRow>> {
        let rows = self.db.query("SELECT * FROM kit_bug_report_files WHERE report_id = $1 ORDER BY id", p![report_id]).await?;
        Ok(rows.iter().map(FileRow::from_row).collect())
    }

    async fn file(&self, report_id: i64, file_id: i64) -> Result<Option<FileRow>> {
        let row = self.db.one("SELECT * FROM kit_bug_report_files WHERE id = $1 AND report_id = $2", p![file_id, report_id]).await?;
        Ok(row.map(|r| FileRow::from_row(&r)))
    }

    async fn user_bytes(&self, user_id: i64) -> Result<i64> {
        self.export_bytes(&ListFilter { user_id: Some(user_id), ..ListFilter::default() }).await
    }

    async fn export_reports(&self, filter: &ListFilter) -> Result<Vec<Report>> {
        let mut p = Params::new(true);
        Ok(self.db.query(&sql::export(filter, &mut p), p.values).await?.iter().map(Report::from_row).collect())
    }

    async fn export_bytes(&self, filter: &ListFilter) -> Result<i64> {
        let mut p = Params::new(true);
        Ok(self.db.one(&sql::bytes(filter, &mut p), p.values).await?.map(|r| r.i("n")).unwrap_or(0))
    }
}

#[async_trait]
impl CrashStore for PgReports {
    async fn add_crash(&self, n: &NewCrash, keep: i64) -> Result<(i64, bool)> {
        if let Some(d) = &n.dedupe {
            if let Some(r) = self.db.one("SELECT id FROM kit_client_crashes WHERE dedupe = $1", p![d]).await? {
                return Ok((r.i("id"), true));
            }
        }
        let c = &n.crash;
        let id = self.insert(
            "INSERT INTO kit_client_crashes (user_id, username, kind, device, device_id, app_version, os, time, received_at, first_line, text, dedupe)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::DOUBLE PRECISION, $9::DOUBLE PRECISION, $10, $11, $12) RETURNING id",
            p![c.user_id, c.username, c.kind, c.device, c.device_id, c.app_version, c.os, c.time, c.received_at, c.first_line, c.text, n.dedupe],
        ).await?;
        self.db
            .execute(
                "DELETE FROM kit_client_crashes WHERE id NOT IN (SELECT id FROM kit_client_crashes ORDER BY id DESC LIMIT $1)",
                p![keep],
            )
            .await?;
        Ok((id, false))
    }

    async fn crashes(&self, limit: i64) -> Result<Vec<Crash>> {
        let rows = self.db.query("SELECT * FROM kit_client_crashes ORDER BY time DESC, id DESC LIMIT $1", p![limit]).await?;
        Ok(rows.iter().map(Crash::from_row).collect())
    }

    async fn clear_crashes(&self) -> Result<()> {
        self.db.execute("DELETE FROM kit_client_crashes", p![]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit_server_core::pg::test_db;

    #[tokio::test]
    async fn reports_keep_their_contract() {
        let store = PgReports::new(test_db().await).await.unwrap();
        crate::store::reports_contract(&store).await;
    }

    #[tokio::test]
    async fn crashes_keep_their_contract() {
        let store = PgReports::new(test_db().await).await.unwrap();
        crate::store::crash_contract(&store).await;
    }

    #[tokio::test]
    async fn migrations_run_once() {
        let db = test_db().await;
        PgReports::new(db.clone()).await.unwrap();
        PgReports::new(db.clone()).await.unwrap();
        let n = db
            .one("SELECT COUNT(*) AS n FROM kit_migrations WHERE component IN ('bug-reports', 'crash-reports')", p![])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(n.i("n"), 2);
    }
}
