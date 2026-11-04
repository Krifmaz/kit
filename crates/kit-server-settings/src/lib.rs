//! An app's server settings from one table, its `settings.toml`: every setting with its environment variable,
//! default and whether it is secret, and the Settings pages, cards and fields that show them.
//!
//! - [`file`] (feature `toml`): reads and checks a settings.toml, and generates the app's `Keys`, `TABLE`,
//!   `SCHEMA` and typed accessors from it in the app's build script. A file that does not fit is refused with
//!   the reason.
//! - [`table`]: the table as Rust data ([`KeyDef`], [`Section`], [`Group`], [`Field`], [`Table`]).
//! - [`when`]: a field's visibility rule (`key=value`, `key!=value`).
//! - [`schema`]: the answer of `GET /api/settings/schema` (contracts/settings-schema.md).
//! - [`values`]: how values take their setting's type, the secret mask, and the [`Values`] trait.
//! - [`Store`] (feature `server`): the values on the app's database; environment, then saved, then default.
//! - [`router`] (feature `server`): `GET/PUT /api/settings`, the schema and a saved secret, for any app
//!   that implements [`SettingsHost`], on paths the app may change ([`SettingsPaths`]).

#[cfg(feature = "toml")]
pub mod file;
#[cfg(feature = "server")]
pub mod router;
pub mod schema;
#[cfg(feature = "server")]
pub mod store;
pub mod table;
pub mod values;
pub mod when;

#[cfg(feature = "server")]
pub use router::{router, router_with, Owner, SettingsHost, SettingsPaths};
#[cfg(feature = "server")]
pub use store::Store;
pub use table::{DefaultValue, Field, Group, KeyDef, Section, Table};
pub use values::{coerce, is_masked, Def, EnvLookup, Values, MASK};
