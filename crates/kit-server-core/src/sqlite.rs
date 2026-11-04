//! [`SqliteDb`]: a SQLite database, one connection behind a lock, for the kit's SQLite stores, for an app that
//! has no database layer of its own (the template's apps) and for the crates' tests; [`SqliteDb::migrate`] makes
//! the kit's `kit_*` tables, and [`SqlitePrefs`] keeps the account preferences on it.

use crate::store::{async_trait, prefs_from_text, Migrations, PrefsStore, PREFS_SQLITE};
use crate::{p, Db, Error, Result, Row, RowExt};
use rusqlite::types::{Value as SqlValue, ValueRef};
use rusqlite::{params_from_iter, Connection};
use serde_json::Value;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A SQLite database; clones share the connection.
#[derive(Clone)]
pub struct SqliteDb {
    /// the one connection
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDb {
    /// Open (or create) the database file at `path`, in WAL mode.
    pub fn open(path: &Path) -> Result<SqliteDb> {
        let conn = Connection::open(path).map_err(Error::database)?;
        conn.pragma_update(None, "journal_mode", "WAL").map_err(Error::database)?;
        Ok(SqliteDb { conn: Arc::new(Mutex::new(conn)) })
    }

    /// A database in memory (tests).
    pub fn memory() -> SqliteDb {
        let conn = Connection::open_in_memory().expect("SQLite in memory");
        SqliteDb { conn: Arc::new(Mutex::new(conn)) }
    }

    /// Run the steps of `m` this database has not run yet, each in a transaction with its record in
    /// `kit_migrations`.
    pub fn migrate(&self, m: &Migrations) -> Result<()> {
        let mut conn = self.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kit_migrations (component TEXT NOT NULL, step INTEGER NOT NULL, applied_at REAL NOT NULL, PRIMARY KEY (component, step))",
        )
        .map_err(Error::database)?;
        let done: i64 = conn
            .query_row("SELECT COALESCE(MAX(step), 0) FROM kit_migrations WHERE component = ?", [m.component], |r| r.get(0))
            .map_err(Error::database)?;
        for (i, step) in m.steps.iter().enumerate() {
            let n = i as i64 + 1;
            if n <= done {
                continue;
            }
            let tx = conn.transaction().map_err(Error::database)?;
            tx.execute_batch(step).map_err(Error::database)?;
            tx.execute(
                "INSERT INTO kit_migrations (component, step, applied_at) VALUES (?, ?, ?)",
                rusqlite::params![m.component, n, crate::db::now()],
            )
            .map_err(Error::database)?;
            tx.commit().map_err(Error::database)?;
        }
        Ok(())
    }

    /// The connection, even after a panic in another thread held it.
    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }
}

/// A JSON parameter as SQLite's value: arrays and objects go in as their JSON text.
fn to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(i64::from(*b)),
        Value::Number(n) => n.as_i64().map(SqlValue::Integer).unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}

/// A SQLite column value as JSON (blobs as text).
fn to_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(i) => Value::from(i),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null),
        ValueRef::Text(t) | ValueRef::Blob(t) => Value::String(String::from_utf8_lossy(t).into_owned()),
    }
}

impl Db for SqliteDb {
    fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        let conn = self.lock();
        let mut st = conn.prepare(sql).map_err(Error::database)?;
        let names: Vec<String> = st.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = st.query(params_from_iter(params.iter().map(to_sql))).map_err(Error::database)?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(Error::database)? {
            let mut row = Row::new();
            for (i, n) in names.iter().enumerate() {
                row.insert(n.clone(), to_json(r.get_ref(i).map_err(Error::database)?));
            }
            out.push(row);
        }
        Ok(out)
    }

    fn execute(&self, sql: &str, params: Vec<Value>) -> Result<i64> {
        let conn = self.lock();
        conn.execute(sql, params_from_iter(params.iter().map(to_sql))).map_err(Error::database)?;
        Ok(conn.last_insert_rowid())
    }

    fn execute_batch(&self, sql: &str) -> Result<()> {
        self.lock().execute_batch(sql).map_err(Error::database)
    }
}

/// The account preferences in SQLite, in `kit_account_prefs`.
#[derive(Clone)]
pub struct SqlitePrefs {
    /// the database
    db: SqliteDb,
}

impl SqlitePrefs {
    /// The preferences on `db`, its table made or brought up to date.
    pub fn new(db: SqliteDb) -> Result<SqlitePrefs> {
        db.migrate(&PREFS_SQLITE)?;
        Ok(SqlitePrefs { db })
    }
}

#[async_trait]
impl PrefsStore for SqlitePrefs {
    async fn prefs(&self, account: i64) -> Result<serde_json::Value> {
        let row = self.db.one("SELECT prefs FROM kit_account_prefs WHERE account_id = ?", p![account])?;
        Ok(prefs_from_text(&row.map(|r| r.s("prefs")).unwrap_or_default()))
    }

    async fn save_prefs(&self, account: i64, prefs: &serde_json::Value) -> Result<()> {
        self.db.execute(
            "INSERT INTO kit_account_prefs (account_id, prefs, updated_at) VALUES (?, ?, ?)
             ON CONFLICT (account_id) DO UPDATE SET prefs = excluded.prefs, updated_at = excluded.updated_at",
            p![account, prefs.to_string(), crate::db::now()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_once_in_order() {
        let db = SqliteDb::memory();
        const M: Migrations = Migrations {
            component: "t",
            steps: &["CREATE TABLE kit_t (id INTEGER PRIMARY KEY)", "ALTER TABLE kit_t ADD COLUMN n INTEGER DEFAULT 3"],
        };
        db.migrate(&M).unwrap();
        db.migrate(&M).unwrap();
        db.execute("INSERT INTO kit_t (id) VALUES (1)", p![]).unwrap();
        assert_eq!(db.one("SELECT n FROM kit_t", p![]).unwrap().unwrap().i("n"), 3);
        assert_eq!(db.one("SELECT COUNT(*) n FROM kit_migrations", p![]).unwrap().unwrap().i("n"), 2);
        const BAD: Migrations = Migrations { component: "bad", steps: &["CREATE TABLE kit_b (id INTEGER)", "NOT SQL"] };
        assert!(db.migrate(&BAD).is_err());
        assert_eq!(db.one("SELECT MAX(step) s FROM kit_migrations WHERE component='bad'", p![]).unwrap().unwrap().i("s"), 1);
    }

    #[cfg(feature = "testing")]
    #[test]
    fn prefs_keep_their_contract() {
        let prefs = SqlitePrefs::new(SqliteDb::memory()).unwrap();
        futures_lite_block_on(crate::store::prefs_contract(&prefs));
    }

    /// Runs a future to its end on this thread (the SQLite stores never wait).
    #[cfg(feature = "testing")]
    fn futures_lite_block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(f)
    }

    #[test]
    fn rows_and_parameters_are_json() {
        let db = SqliteDb::memory();
        db.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, at REAL, tags TEXT)").unwrap();
        let id = db.execute("INSERT INTO t (name, at, tags) VALUES (?,?,?)", p!["a", 1.5, ["x"]]).unwrap();
        assert_eq!(id, 1);
        let r = db.one("SELECT * FROM t WHERE id=?", p![id]).unwrap().unwrap();
        assert_eq!(r.s("name"), "a");
        assert_eq!(r.f("at"), 1.5);
        assert_eq!(r.s("tags"), "[\"x\"]");
        assert!(db.one("SELECT * FROM t WHERE id=?", p![9]).unwrap().is_none());
        assert_eq!(db.query("SELECT nope FROM t", p![]).unwrap_err().detail, "Database error");
    }
}
