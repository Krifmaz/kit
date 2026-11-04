//! [`SqliteDb`]: the kit's own SQLite database, one connection behind a lock, for an app that has no database
//! layer of its own (the template's apps) and for the crates' tests. Encore passes its own `encore_db::Db`.

use crate::{Db, Error, Result, Row};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{p, RowExt};

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
