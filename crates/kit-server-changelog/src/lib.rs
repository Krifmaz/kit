//! What's new (contracts/changelog.md): an app's release notes for the popup it shows once after an update
//! and for Settings → What's new.
//!
//! The notes are the app's `docs/releases/<MAJOR.MINOR.PATCH>.md`, built into the server ([`build::embed`]).
//! What counts as new is per account: `prefs.whats_new_seen` holds the version the person last saw, and only
//! releases with notes newer than it (and not newer than this server) are new. Hotfix builds (0.36.0.17)
//! share their release's notes, so a hotfix alone never shows the popup again. `prefs.whats_new_popup = false`
//! turns the popup off; the full list still shows everything.
//!
//! From Encore's `server/crates/api/src/routes/changelog.rs` and `build.rs`.
//!
//! - [`build::embed`]: the build script part (all that is left with `default-features = false`).
//! - feature `server` (on by default): `parse`, `plain`, `unseen` (the notes as parts, `Block`, and which ones
//!   are new) and `routes`: `GET /api/changelog`, `GET /api/changelog/new`, `POST /api/changelog/seen` for an
//!   app state that implements `ChangelogHost`.

pub mod build;
#[cfg(feature = "server")]
mod notes;
#[cfg(feature = "server")]
pub mod routes;

#[cfg(feature = "server")]
pub use notes::{base, parse, plain, unseen, Block, Release, HIGHLIGHTS, POPUP_RELEASES};
#[cfg(feature = "server")]
pub use routes::{router, Changelog, ChangelogHost};
