//! What every kit server crate shares, so each one can be added to any app in a few lines.
//!
//! An app hands the crates its own state. The state tells them who is signed in ([`Host`]) and gives each
//! crate a store for what it keeps ([`store`]); the crates answer errors one way ([`Error`], `{"detail": …}`).
//! No crate reads or writes an app's own tables: the kit keeps its data in `kit_*` tables of its own.
//!
//! - [`error`]: the error answer every route gives.
//! - [`store`]: the storage traits' common parts ([`Migrations`], [`PrefsStore`]).
//! - [`db`]: the SQLite statement trait (JSON rows and parameters), [`p!`] and [`RowExt`].
//! - [`auth`]: the signed-in account, the [`Host`] trait and the [`SignedIn`] / [`Admin`] extractors.
//! - [`version`]: version numbers compared by their numeric parts.
//! - `sqlite` (feature `sqlite`): `SqliteDb` and `SqlitePrefs`.
//! - `pg` (feature `postgres`): `PgDb` and `PgPrefs`.
//! - `testing` (feature `testing`): `TestHost`, an app state for tests, and the stores' contracts.

pub mod auth;
pub mod db;
pub mod error;
#[cfg(feature = "postgres")]
pub mod pg;
#[cfg(feature = "sqlite")]
pub mod sqlite;
pub mod store;
#[cfg(feature = "testing")]
pub mod testing;
pub mod version;

pub use auth::{Account, Admin, Host, SignedIn};
pub use db::{Db, Row, RowExt};
pub use error::{err, Error, Result};
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteDb, SqlitePrefs};
pub use store::{async_trait, Migrations, PrefsStore};

/// serde_json for [`p!`], so a crate that uses the macro needs no dependency of its own on it.
#[doc(hidden)]
pub use serde_json as __json;
