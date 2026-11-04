# kit-server-core

What every kit server crate shares, so each one can be added to any app in a few lines:

- **`Error`** — the error answer every route gives: `{"detail": "<plain sentence>"}` with an HTTP status,
  so every client shows a kit error the same way (extra fields merge into the body).
- **Storage** — no crate reads or writes an app's own tables. Each crate keeps its data behind a small async
  trait of its own (`PrefsStore` here; `BugReportStore`, `SettingsStore`, `NoticeStore` in their crates) with two
  ready implementations: SQLite and Postgres. Both keep the kit's data in tables of its own, all named `kit_*`,
  made and changed by `Migrations` (each step runs once, in order, recorded in `kit_migrations`).
- **`PrefsStore`** — each account's preferences, one JSON object per account id (`kit_account_prefs`):
  `SqlitePrefs` (feature `sqlite`) or `pg::PgPrefs` (feature `postgres`).
- **`Host`** — implemented once by the app for its state: who is signed in (an `Account`: id, username, admin,
  owner). Handlers take `SignedIn` or `Admin`; the app's own 401/403 answers come through unchanged.
- **`version`** — version numbers compared by their numeric parts; `base("1.4.0.17")` is `1.4.0`.
- **`SqliteDb`** (feature `sqlite`) — a SQLite database (JSON rows and parameters through the `Db` trait,
  `p![…]`, `RowExt`) with `migrate`, for the SQLite stores and for apps without a database layer of their own.
- **`pg::PgDb`** (feature `postgres`) — a Postgres pool with the same JSON rows (`$1` parameters) and `migrate`,
  on sqlx's core and Postgres driver (so it sits next to rusqlite; TLS is the app's sqlx feature).
- **`testing::TestHost`** (feature `testing`) — an app state for tests: SQLite in memory, fixed accounts
  (1 the owner, 2 and 3 members, 4 an admin), signed in by the `x-test-user` header; `pg::test_db()` (with
  `postgres`) gives a test its own schema in the database `KIT_TEST_POSTGRES_URL` names, and fails without it.

```rust
#[derive(Clone)]
struct App { prefs: kit_server_core::SqlitePrefs } // SqlitePrefs::new(SqliteDb::open(path)?)? at start

impl kit_server_core::Host for App {
    async fn signed_in(&self, parts: &mut axum::http::request::Parts) -> kit_server_core::Result<kit_server_core::Account> {
        my_session_lookup(self, parts).await
    }
}
```
