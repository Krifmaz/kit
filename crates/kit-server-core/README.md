# kit-server-core

What every kit server crate shares, so each one can be added to any app in a few lines:

- **`Error`** — the error answer every route gives: `{"detail": "<plain sentence>"}` with an HTTP status,
  exactly as Encore's `encore_core::error` answers (extra fields merge into the body).
- **`Db`** — the app's database behind a small trait: JSON rows and JSON parameters, as Encore's
  `encore_db`, so each crate keeps Encore's SQL. `p![…]` builds parameters; `RowExt` reads columns.
- **`Host`** — implemented once by the app for its state: the database and who is signed in. Handlers take
  `SignedIn` or `Admin`; the app's own 401/403 answers come through unchanged.
- **`version`** — version numbers compared by their numeric parts; `base("0.36.0.17")` is `0.36.0`.
- **`SqliteDb`** (feature `sqlite`) — a ready SQLite database for apps without one.
- **`testing::TestHost`** (feature `testing`) — an app state for tests: SQLite in memory, accounts 1 (admin)
  and 2 (member), signed in by the `x-test-user` header.

```rust
#[derive(Clone)]
struct App { db: kit_server_core::SqliteDb }

impl kit_server_core::Host for App {
    fn db(&self) -> &dyn kit_server_core::Db { &self.db }
    async fn signed_in(&self, parts: &mut axum::http::request::Parts) -> kit_server_core::Result<kit_server_core::Account> {
        my_session_lookup(self, parts).await
    }
}
```
