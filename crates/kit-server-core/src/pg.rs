//! Postgres (feature `postgres`): [`PgDb`], a pool with JSON rows and parameters like [`Db`](crate::Db)'s,
//! [`PgDb::migrate`] for the kit's `kit_*` tables, and [`PgPrefs`], the account preferences on it.
//!
//! The crates' Postgres stores are written on [`PgDb`]; its statements use Postgres' `$1, $2 …` parameters.
//! sqlx is used through its core and Postgres driver ([`sqlx_core`], [`sqlx_postgres`], re-exported), so it
//! can sit next to rusqlite in one app. TLS is the app's: turn on a TLS feature of `sqlx-core` in the app.

use crate::store::{async_trait, prefs_from_text, Migrations, PrefsStore, PREFS_POSTGRES};
use crate::{Error, Result, Row};
use serde_json::Value;
pub use sqlx_core;
use sqlx_core::column::Column;
use sqlx_core::executor::Executor;
use sqlx_core::query::Query;
use sqlx_core::row::Row as _;
use sqlx_core::type_info::TypeInfo;
pub use sqlx_postgres;
use sqlx_postgres::{PgArguments, PgPool, PgPoolOptions, PgRow, Postgres};

/// A Postgres pool; clones share it.
#[derive(Clone, Debug)]
pub struct PgDb {
    /// the pool
    pub pool: PgPool,
}

/// One parameter bound by its JSON kind: integers as `bigint`, other numbers as `double precision`, text as
/// `text`, booleans as `boolean`, null as a `text` null, arrays and objects as their JSON text.
fn bind<'q>(q: Query<'q, Postgres, PgArguments>, v: &Value) -> Query<'q, Postgres, PgArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(*b),
        Value::Number(n) => match n.as_i64() {
            Some(i) => q.bind(i),
            None => q.bind(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    }
}

/// A statement with its parameters bound. Postgres fixes a prepared statement's parameter types when it is first
/// prepared, and each pooled connection keeps the statements it prepared, keyed by their SQL. A null is bound as
/// a `text` null, so a statement first run with a null where it later gets a number would send that number's
/// bytes as text ("invalid byte sequence for encoding UTF8: 0x00"), on whichever connection kept it. So a
/// statement with a null is prepared for that call only and never kept.
fn statement<'q>(sql: &'q str, params: &[Value]) -> Query<'q, Postgres, PgArguments> {
    let q = params.iter().fold(sqlx_core::query::query::<Postgres>(sql), bind);
    if params.iter().any(Value::is_null) {
        q.persistent(false)
    } else {
        q
    }
}

/// A column of a Postgres row as JSON, by its type (integers, reals, booleans, text, JSON; anything else as
/// text when it reads as text, null otherwise).
fn column(row: &PgRow, i: usize) -> Value {
    let name = row.columns()[i].type_info().name().to_string();
    let get_i = |r: &PgRow| -> Option<i64> {
        r.try_get::<Option<i64>, _>(i).ok().flatten().or_else(|| r.try_get::<Option<i32>, _>(i).ok().flatten().map(i64::from))
    };
    match name.as_str() {
        "INT8" | "INT4" | "INT2" => {
            get_i(row).or_else(|| row.try_get::<Option<i16>, _>(i).ok().flatten().map(i64::from)).map(Value::from).unwrap_or(Value::Null)
        }
        "FLOAT8" => {
            row.try_get::<Option<f64>, _>(i).ok().flatten().and_then(serde_json::Number::from_f64).map(Value::Number).unwrap_or(Value::Null)
        }
        "FLOAT4" => row
            .try_get::<Option<f32>, _>(i)
            .ok()
            .flatten()
            .and_then(|f| serde_json::Number::from_f64(f64::from(f)))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "BOOL" => row.try_get::<Option<bool>, _>(i).ok().flatten().map(Value::Bool).unwrap_or(Value::Null),
        "JSON" | "JSONB" => row.try_get::<Option<Value>, _>(i).ok().flatten().unwrap_or(Value::Null),
        _ => row.try_get::<Option<String>, _>(i).ok().flatten().map(Value::String).unwrap_or(Value::Null),
    }
}

/// A Postgres row as a JSON object.
fn to_row(r: &PgRow) -> Row {
    let mut row = Row::new();
    for (i, c) in r.columns().iter().enumerate() {
        row.insert(c.name().to_string(), column(r, i));
    }
    row
}

impl PgDb {
    /// A pool of up to 10 connections to `url` (`postgres://user:password@host/database`).
    pub async fn connect(url: &str) -> Result<PgDb> {
        let pool = PgPoolOptions::new().max_connections(10).connect(url).await.map_err(Error::database)?;
        Ok(PgDb { pool })
    }

    /// The database on a pool the app already has.
    pub fn new(pool: PgPool) -> PgDb {
        PgDb { pool }
    }

    /// Rows of a statement.
    pub async fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Row>> {
        let q = statement(sql, &params);
        let rows = q.fetch_all(&self.pool).await.map_err(Error::database)?;
        Ok(rows.iter().map(to_row).collect())
    }

    /// The first row of a statement, if any.
    pub async fn one(&self, sql: &str, params: Vec<Value>) -> Result<Option<Row>> {
        Ok(self.query(sql, params).await?.into_iter().next())
    }

    /// Runs a statement; the number of rows it changed. A statement that makes a row and needs its id says
    /// `RETURNING id` and is run with [`PgDb::one`].
    pub async fn execute(&self, sql: &str, params: Vec<Value>) -> Result<u64> {
        let q = statement(sql, &params);
        Ok(q.execute(&self.pool).await.map_err(Error::database)?.rows_affected())
    }

    /// Runs several statements without parameters.
    pub async fn execute_batch(&self, sql: &str) -> Result<()> {
        (&self.pool).execute(sql).await.map_err(Error::database)?;
        Ok(())
    }

    /// Run the steps of `m` this database has not run yet, each in a transaction with its record in
    /// `kit_migrations`. Servers that start together wait for each other (an advisory lock), so a step runs once.
    pub async fn migrate(&self, m: &Migrations) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(Error::database)?;
        sqlx_core::query::query::<Postgres>("SELECT pg_advisory_xact_lock(7426511)").execute(&mut *tx).await.map_err(Error::database)?;
        sqlx_core::query::query::<Postgres>(
            "CREATE TABLE IF NOT EXISTS kit_migrations (component TEXT NOT NULL, step BIGINT NOT NULL, applied_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (component, step))",
        )
        .execute(&mut *tx)
        .await
        .map_err(Error::database)?;
        let done: Option<i64> =
            sqlx_core::query_scalar::query_scalar::<Postgres, Option<i64>>("SELECT MAX(step) FROM kit_migrations WHERE component = $1")
                .bind(m.component)
                .fetch_one(&mut *tx)
                .await
                .map_err(Error::database)?;
        let done = done.unwrap_or(0);
        for (i, step) in m.steps.iter().enumerate() {
            let n = i as i64 + 1;
            if n <= done {
                continue;
            }
            (&mut *tx).execute(*step).await.map_err(Error::database)?;
            sqlx_core::query::query::<Postgres>("INSERT INTO kit_migrations (component, step) VALUES ($1, $2)")
                .bind(m.component)
                .bind(n)
                .execute(&mut *tx)
                .await
                .map_err(Error::database)?;
        }
        tx.commit().await.map_err(Error::database)?;
        Ok(())
    }
}

/// The account preferences in Postgres, in `kit_account_prefs`.
#[derive(Clone, Debug)]
pub struct PgPrefs {
    /// the database
    db: PgDb,
}

impl PgPrefs {
    /// The preferences on `db`, its table made or brought up to date.
    pub async fn new(db: PgDb) -> Result<PgPrefs> {
        db.migrate(&PREFS_POSTGRES).await?;
        Ok(PgPrefs { db })
    }
}

#[async_trait]
impl PrefsStore for PgPrefs {
    async fn prefs(&self, account: i64) -> Result<Value> {
        let row = self.db.one("SELECT prefs FROM kit_account_prefs WHERE account_id = $1", crate::p![account]).await?;
        Ok(prefs_from_text(&row.map(|r| crate::RowExt::s(&r, "prefs")).unwrap_or_default()))
    }

    async fn save_prefs(&self, account: i64, prefs: &Value) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO kit_account_prefs (account_id, prefs, updated_at) VALUES ($1, $2, $3)
                 ON CONFLICT (account_id) DO UPDATE SET prefs = EXCLUDED.prefs, updated_at = EXCLUDED.updated_at",
                crate::p![account, prefs.to_string(), crate::db::now()],
            )
            .await?;
        Ok(())
    }
}

/// A Postgres database of its own for one test: a new schema in the database `KIT_TEST_POSTGRES_URL` names,
/// which every connection of the pool uses. It panics when the variable is not set, so a Postgres test run
/// without a database fails instead of passing untested.
#[cfg(feature = "testing")]
pub async fn test_db() -> PgDb {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let url = std::env::var("KIT_TEST_POSTGRES_URL")
        .expect("KIT_TEST_POSTGRES_URL is not set: the Postgres tests need a database (postgres://user:password@host/database)");
    let schema = format!("kit_test_{}_{}_{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed), crate::db::now() as u64);
    let admin = PgPoolOptions::new().max_connections(1).connect(&url).await.expect("the test database can't be reached");
    (&admin).execute(format!("CREATE SCHEMA {schema}").as_str()).await.expect("a test schema");
    admin.close().await;
    let set = format!("SET search_path TO {schema}");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |conn, _| {
            let set = set.clone();
            Box::pin(async move {
                conn.execute(set.as_str()).await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .expect("the test database can't be reached");
    PgDb { pool }
}

#[cfg(all(test, feature = "testing"))]
mod tests {
    use super::*;
    use crate::{p, RowExt};

    #[tokio::test]
    async fn rows_parameters_and_migrations() {
        let db = test_db().await;
        const M: Migrations = Migrations {
            component: "t",
            steps: &[
                "CREATE TABLE kit_t (id BIGSERIAL PRIMARY KEY, name TEXT, at DOUBLE PRECISION, on_ BOOLEAN)",
                "ALTER TABLE kit_t ADD COLUMN n BIGINT DEFAULT 3",
            ],
        };
        db.migrate(&M).await.unwrap();
        db.migrate(&M).await.unwrap();
        let r = db.one("INSERT INTO kit_t (name, at, on_) VALUES ($1, $2, $3) RETURNING id", p!["a", 1.5, true]).await.unwrap().unwrap();
        assert_eq!(r.i("id"), 1);
        let r = db.one("SELECT * FROM kit_t WHERE id = $1", p![1]).await.unwrap().unwrap();
        assert_eq!((r.s("name"), r.f("at"), r.b("on_"), r.i("n")), ("a".into(), 1.5, true, 3));
        let n = db.one("SELECT COUNT(*) AS n FROM kit_migrations WHERE component = 't'", p![]).await.unwrap().unwrap();
        assert_eq!(n.i("n"), 2);
        assert_eq!(db.query("SELECT nope FROM kit_t", p![]).await.unwrap_err().detail, "Database error");
    }

    #[tokio::test]
    async fn a_null_then_a_number_in_one_statement() {
        // one connection, so the second call meets whatever the first one left prepared
        let url = std::env::var("KIT_TEST_POSTGRES_URL").expect("KIT_TEST_POSTGRES_URL");
        let db = PgDb::new(PgPoolOptions::new().max_connections(1).connect(&url).await.expect("the test database"));
        db.execute_batch("CREATE TEMP TABLE kit_null_t (n BIGINT)").await.unwrap();
        let sql = "INSERT INTO kit_null_t (n) VALUES ($1::BIGINT)";
        for v in [Value::Null, Value::from(7), Value::Null, Value::from(8)] {
            db.execute(sql, vec![v]).await.unwrap();
        }
        let r = db.one("SELECT COUNT(n) AS c, SUM(n)::BIGINT AS s FROM kit_null_t", p![]).await.unwrap().unwrap();
        assert_eq!((r.i("c"), r.i("s")), (2, 15));
    }

    #[tokio::test]
    async fn prefs_keep_their_contract() {
        let prefs = PgPrefs::new(test_db().await).await.unwrap();
        crate::store::prefs_contract(&prefs).await;
    }
}
