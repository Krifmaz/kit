//! Offline mode that explains itself: why the server can't be reached ([`reason`]), when offline mode opens and
//! what it says ([`mode`]), and the per-account outbox of plays and likes to send later ([`outbox`]).
//!
//! From Encore's `web-rs/src/offline/` (`mod.rs`, `mode.rs`, `outbox.rs`) and Android's `features/offline/`
//! (`OfflineReason.kt`, `OfflineMode.kt`, `OfflineOutbox.kt`). The Cache API and file storage, the downloads
//! and the network stay in the UI crates.

pub mod mode;
pub mod outbox;
pub mod reason;
