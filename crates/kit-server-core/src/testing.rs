//! [`TestHost`]: an app state for the kit crates' tests. SQLite in memory with a small `users` table;
//! a request is signed in as the account whose id is in its `x-test-user` header.

use crate::{err, Account, Db, Host, Result, SqliteDb};
use axum::http::request::Parts;
use std::path::PathBuf;

/// The test app's state; clones share the database.
#[derive(Clone)]
pub struct TestHost {
    /// the database
    pub db: SqliteDb,
    /// a folder of its own for files (removed by nobody: the system's temp folder clears it)
    pub data_dir: PathBuf,
}

impl TestHost {
    /// A fresh app with two accounts: 1 `admin` (an admin) and 2 `ana` (a member).
    pub fn new() -> TestHost {
        let db = SqliteDb::memory();
        db.execute_batch(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, username TEXT, role TEXT DEFAULT 'member', prefs TEXT DEFAULT '{}', disabled INTEGER DEFAULT 0);
             INSERT INTO users (id, username, role) VALUES (1, 'admin', 'owner'), (2, 'ana', 'member');",
        )
        .expect("test tables");
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let data_dir = std::env::temp_dir().join(format!("kit-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).expect("test folder");
        TestHost { db, data_dir }
    }
}

impl Default for TestHost {
    fn default() -> Self {
        TestHost::new()
    }
}

impl Host for TestHost {
    fn db(&self) -> &dyn Db {
        &self.db
    }

    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        let id: i64 = parts.headers.get("x-test-user").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
        let row = self.db.one("SELECT * FROM users WHERE id=? AND disabled=0", crate::p![id])?;
        row.map(Account::from_row).ok_or_else(|| err(401, "Not authenticated"))
    }
}
