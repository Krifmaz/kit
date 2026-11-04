//! What every kit server crate shares, so each one can be added to any app in a few lines.
//!
//! An app hands the crates its own state. The state tells them who is signed in ([`Host`]) and gives them the
//! app's database ([`Db`]); the crates answer errors one way ([`Error`], `{"detail": …}`).
//!
//! - [`error`]: the error answer every route gives.
//! - [`db`]: the database trait (JSON rows and parameters), [`p!`] and [`RowExt`].
//! - [`auth`]: the signed-in account, the [`Host`] trait and the [`SignedIn`] / [`Admin`] extractors.
//! - [`version`]: version numbers compared by their numeric parts.
//! - `sqlite` (feature `sqlite`): `SqliteDb`, a ready database for apps without one.
//! - `testing` (feature `testing`): `TestHost`, an app state for tests.

pub mod auth;
pub mod db;
pub mod error;
#[cfg(feature = "sqlite")]
pub mod sqlite;
#[cfg(feature = "testing")]
pub mod testing;
pub mod version;

pub use auth::{Account, Admin, Host, SignedIn};
pub use db::{Db, Row, RowExt};
pub use error::{err, Error, Result};
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteDb;

/// serde_json for [`p!`], so a crate that uses the macro needs no dependency of its own on it.
#[doc(hidden)]
pub use serde_json as __json;
