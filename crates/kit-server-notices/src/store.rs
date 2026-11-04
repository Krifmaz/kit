//! [`NoticeStore`]: where the last result of each kept notice ([`crate::NoticeCheck::kept`]) is saved, so a
//! server that restarts shows the banner it showed before, until its first new check ends. [`SqliteNotices`]
//! or, with feature `postgres`, [`PgNotices`] keep them in the kit's own `kit_notices` table; an app may
//! implement the trait on its own storage.

use kit_server_core::{async_trait, p, Db, Migrations, Result, RowExt, SqliteDb};
use serde_json::Value;
use std::collections::HashMap;

/// Where the last result of each kept notice is saved: one notice object per notice id.
#[async_trait]
pub trait NoticeStore: Send + Sync {
    /// Every saved result, by notice id.
    async fn load(&self) -> Result<HashMap<String, Value>>;
    /// Keep `notice` as the last result of notice `id`.
    async fn save(&self, id: &str, notice: &Value) -> Result<()>;
}

/// The SQLite steps of `kit_notices`.
pub const SQLITE: Migrations = Migrations {
    component: "notices",
    steps: &["CREATE TABLE kit_notices (id TEXT PRIMARY KEY, notice TEXT NOT NULL, checked_at REAL NOT NULL DEFAULT 0)"],
};

/// The Postgres steps of `kit_notices`.
pub const POSTGRES: Migrations = Migrations {
    component: "notices",
    steps: &["CREATE TABLE kit_notices (id TEXT PRIMARY KEY, notice TEXT NOT NULL, checked_at DOUBLE PRECISION NOT NULL DEFAULT 0)"],
};

/// Saved rows by id; a row whose notice is not a JSON object is left out.
fn rows_to_map(rows: Vec<kit_server_core::Row>) -> HashMap<String, Value> {
    rows.into_iter()
        .filter_map(|r| {
            let v: Value = serde_json::from_str(&r.s("notice")).ok()?;
            v.is_object().then(|| (r.s("id"), v))
        })
        .collect()
}

/// The notices' last results in SQLite, in `kit_notices`.
#[derive(Clone)]
pub struct SqliteNotices {
    /// the database
    db: SqliteDb,
}

impl SqliteNotices {
    /// The last results on `db`, its table made or brought up to date.
    pub fn new(db: SqliteDb) -> Result<SqliteNotices> {
        db.migrate(&SQLITE)?;
        Ok(SqliteNotices { db })
    }
}

#[async_trait]
impl NoticeStore for SqliteNotices {
    async fn load(&self) -> Result<HashMap<String, Value>> {
        Ok(rows_to_map(self.db.query("SELECT id, notice FROM kit_notices", p![])?))
    }

    async fn save(&self, id: &str, notice: &Value) -> Result<()> {
        self.db.execute(
            "INSERT INTO kit_notices (id, notice, checked_at) VALUES (?, ?, ?)
             ON CONFLICT (id) DO UPDATE SET notice = excluded.notice, checked_at = excluded.checked_at",
            p![id, notice.to_string(), kit_server_core::db::now()],
        )?;
        Ok(())
    }
}

/// The notices' last results in Postgres, in `kit_notices` (feature `postgres`).
#[cfg(feature = "postgres")]
#[derive(Clone, Debug)]
pub struct PgNotices {
    /// the database
    db: kit_server_core::pg::PgDb,
}

#[cfg(feature = "postgres")]
impl PgNotices {
    /// The last results on `db`, its table made or brought up to date.
    pub async fn new(db: kit_server_core::pg::PgDb) -> Result<PgNotices> {
        db.migrate(&POSTGRES).await?;
        Ok(PgNotices { db })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl NoticeStore for PgNotices {
    async fn load(&self) -> Result<HashMap<String, Value>> {
        Ok(rows_to_map(self.db.query("SELECT id, notice FROM kit_notices", p![]).await?))
    }

    async fn save(&self, id: &str, notice: &Value) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO kit_notices (id, notice, checked_at) VALUES ($1, $2, $3)
                 ON CONFLICT (id) DO UPDATE SET notice = EXCLUDED.notice, checked_at = EXCLUDED.checked_at",
                p![id, notice.to_string(), kit_server_core::db::now()],
            )
            .await?;
        Ok(())
    }
}

/// What every [`NoticeStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
pub async fn notices_contract(store: &dyn NoticeStore) {
    use serde_json::json;
    assert!(store.load().await.unwrap().is_empty());
    store.save("disk", &json!({ "ok": false, "text": "Full" })).await.unwrap();
    store.save("sync", &json!({ "ok": true, "text": null })).await.unwrap();
    store.save("disk", &json!({ "ok": true, "text": null, "free": 12 })).await.unwrap();
    let all = store.load().await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all["disk"], json!({ "ok": true, "text": null, "free": 12 }));
    assert_eq!(all["sync"], json!({ "ok": true, "text": null }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_keeps_the_contract() {
        notices_contract(&SqliteNotices::new(SqliteDb::memory()).unwrap()).await;
    }

    #[cfg(feature = "postgres")]
    #[tokio::test]
    async fn postgres_keeps_the_contract() {
        notices_contract(&PgNotices::new(kit_server_core::pg::test_db().await).await.unwrap()).await;
    }
}
