//! [`TestHost`]: an app state for the kit crates' tests. Its accounts are fixed (1 `admin`, the owner; 2 `ana`,
//! a member); a request is signed in as the account whose id is in its `x-test-user` header. The kit's data is
//! in SQLite in memory ([`TestHost::db`]).

use crate::{err, Account, Host, Result, SqliteDb, SqlitePrefs};
use axum::http::request::Parts;
use std::path::PathBuf;

/// The test app's state; clones share the database.
#[derive(Clone)]
pub struct TestHost {
    /// the database the kit's SQLite stores use
    pub db: SqliteDb,
    /// the account preferences, on [`TestHost::db`]
    pub prefs: SqlitePrefs,
    /// a folder of its own for files (removed by nobody: the system's temp folder clears it)
    pub data_dir: PathBuf,
}

impl TestHost {
    /// A fresh app with two accounts: 1 `admin` (the owner) and 2 `ana` (a member).
    pub fn new() -> TestHost {
        let db = SqliteDb::memory();
        let prefs = SqlitePrefs::new(db.clone()).expect("test tables");
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let data_dir = std::env::temp_dir().join(format!("kit-test-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).expect("test folder");
        TestHost { db, prefs, data_dir }
    }

    /// Account `id` of the test app: 1 `admin` (the owner), 2 `ana` and 3 `other` (members), 4 `boss` (an admin,
    /// not the owner); none for any other id.
    pub fn account(id: i64) -> Option<Account> {
        match id {
            1 => Some(Account { id: 1, username: "admin".into(), admin: true, owner: true }),
            2 => Some(Account::member(2, "ana")),
            3 => Some(Account::member(3, "other")),
            4 => Some(Account { id: 4, username: "boss".into(), admin: true, owner: false }),
            _ => None,
        }
    }
}

impl Default for TestHost {
    fn default() -> Self {
        TestHost::new()
    }
}

impl Host for TestHost {
    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        let id: i64 = parts.headers.get("x-test-user").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
        TestHost::account(id).ok_or_else(|| err(401, "Not authenticated"))
    }
}
