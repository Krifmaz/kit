//! The kit's own data behind small traits, so no crate assumes one app's database.
//!
//! Each server crate keeps what it stores behind a trait of its own ([`PrefsStore`] here; the bug reports,
//! settings and notices crates have theirs) with two ready implementations: SQLite (feature `sqlite`, on
//! [`SqliteDb`](crate::SqliteDb)) and Postgres (feature `postgres`, on [`PgDb`](crate::pg::PgDb)). Both keep
//! the kit's data in tables of its own, all named `kit_*`, made by [`Migrations`]; an app's own tables (its
//! accounts among them) are never read or written. An app with another database implements the traits itself.
//!
//! The traits are async ([`async_trait`]) so an app can keep the data on an async database; the SQLite
//! implementations answer at once.

use crate::{p, Db, Result, RowExt};
pub use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

/// An app's database as the kit's SQLite stores hold it: any [`Db`] (the kit's own `SqliteDb` or the app's),
/// shared by clones.
pub type SharedDb = Arc<dyn Db>;

/// The steps that make and change one component's `kit_*` tables, oldest first. A database records the steps
/// it has run in `kit_migrations` (`component`, `step`), so each runs once, in order, in a transaction of its
/// own. A step is never edited once released: a change is a new step at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migrations {
    /// whose tables these are (`"prefs"`, `"bug-reports"` …); unique among the components of one database
    pub component: &'static str,
    /// the SQL of each step (one or more statements), oldest first
    pub steps: &'static [&'static str],
}

impl Migrations {
    /// Run the steps `db` (a SQLite database: the kit's `SqliteDb` or the app's own) has not run yet. Each step
    /// runs in one batch with its record in `kit_migrations`, between `BEGIN IMMEDIATE` and `COMMIT`; a step that
    /// fails is rolled back (`ROLLBACK` on the same database), so `db` must run a batch and the write after it on
    /// one connection, as a database with one write connection does.
    pub fn run(&self, db: &dyn Db) -> Result<()> {
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS kit_migrations (component TEXT NOT NULL, step INTEGER NOT NULL, applied_at REAL NOT NULL, PRIMARY KEY (component, step))",
        )?;
        let done = db
            .one("SELECT COALESCE(MAX(step), 0) AS n FROM kit_migrations WHERE component = ?", p![self.component])?
            .map(|r| r.i("n"))
            .unwrap_or(0);
        for (i, step) in self.steps.iter().enumerate() {
            let n = i as i64 + 1;
            if n <= done {
                continue;
            }
            // the component is a name in the kit's code, quoted as SQL text all the same; a batch takes no parameters
            let component = self.component.replace('\'', "''");
            let batch = format!(
                "BEGIN IMMEDIATE;\n{step};\nINSERT INTO kit_migrations (component, step, applied_at) VALUES ('{component}', {n}, {});\nCOMMIT;",
                crate::db::now()
            );
            if let Err(e) = db.execute_batch(&batch) {
                // the batch stopped at the failing statement with its transaction open: undo it
                let _ = db.execute_batch("ROLLBACK");
                return Err(e);
            }
        }
        Ok(())
    }
}

/// Where each account's preferences are kept: a JSON object per account id (`whats_new_seen`,
/// `whats_new_popup` and anything the app keeps there itself).
#[async_trait]
pub trait PrefsStore: Send + Sync {
    /// The preferences of account `account`; `{}` when none were saved.
    async fn prefs(&self, account: i64) -> Result<Value>;
    /// Keep `prefs` (a JSON object) as the preferences of account `account`, replacing what was there.
    async fn save_prefs(&self, account: i64, prefs: &Value) -> Result<()>;
}

/// The SQLite steps of the preferences table `kit_account_prefs`.
pub const PREFS_SQLITE: Migrations = Migrations {
    component: "prefs",
    steps: &["CREATE TABLE kit_account_prefs (
        account_id INTEGER PRIMARY KEY,
        prefs TEXT NOT NULL DEFAULT '{}',
        updated_at REAL NOT NULL DEFAULT 0
    )"],
};

/// The Postgres steps of the preferences table `kit_account_prefs`.
pub const PREFS_POSTGRES: Migrations = Migrations {
    component: "prefs",
    steps: &["CREATE TABLE kit_account_prefs (
        account_id BIGINT PRIMARY KEY,
        prefs TEXT NOT NULL DEFAULT '{}',
        updated_at DOUBLE PRECISION NOT NULL DEFAULT 0
    )"],
};

/// Preferences as text: `{}` for anything that is not a JSON object.
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub(crate) fn prefs_from_text(text: &str) -> Value {
    match serde_json::from_str::<Value>(text) {
        Ok(v @ Value::Object(_)) => v,
        _ => Value::Object(Default::default()),
    }
}

/// What every [`PrefsStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
#[cfg(feature = "testing")]
pub async fn prefs_contract(store: &dyn PrefsStore) {
    use serde_json::json;
    assert_eq!(store.prefs(7).await.unwrap(), json!({}));
    store.save_prefs(7, &json!({ "whats_new_seen": "1.2.0", "theme": "dark" })).await.unwrap();
    store.save_prefs(8, &json!({ "whats_new_popup": false })).await.unwrap();
    assert_eq!(store.prefs(7).await.unwrap(), json!({ "whats_new_seen": "1.2.0", "theme": "dark" }));
    store.save_prefs(7, &json!({ "whats_new_seen": "1.3.0" })).await.unwrap();
    assert_eq!(store.prefs(7).await.unwrap(), json!({ "whats_new_seen": "1.3.0" }));
    assert_eq!(store.prefs(8).await.unwrap(), json!({ "whats_new_popup": false }));
}
