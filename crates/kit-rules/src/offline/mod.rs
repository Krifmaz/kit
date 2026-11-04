//! Offline mode that explains itself: why the server can't be reached ([`reason`]), when offline mode opens and
//! what it says ([`mode`]), and the per-account outbox of the app's actions to send later ([`outbox`]).
//!
//! The browser's storage, the files an app saves for offline use and the network stay in the UI crates; here
//! are the rules, fed with answers and the time.

pub mod mode;
pub mod outbox;
pub mod reason;
