//! # kit-rules
//!
//! The client-side decisions Encore writes once per client (web `web-rs`, the desktop app `clients/desktop`,
//! the Android app `clients/android`), written once in plain Rust: no UI, no `web-sys`, no async runtime and
//! no clock. Time is passed in as milliseconds (`now_ms: u64`), so the same code runs in the browser
//! (`wasm32-unknown-unknown`), in an Iced app and in tests, and ports line by line to Kotlin.
//!
//! Every rule, number and word here comes from Encore's working code; each item's documentation names the
//! file and function it was taken from. Where Encore's clients differ on purpose (the web keeps three error
//! toasts, Android ten), the difference is a policy struct with named presets (`web()`, `desktop()`,
//! `android()`), never a silent choice.
//!
//! | Module | Decides |
//! | --- | --- |
//! | [`toasts`] | errors stay with Copy; duplicates merge; Undo; stack caps; how long each kind stays |
//! | [`details`] | the "read the whole error" dialog: its title, what Copy copies, whether a copy took |
//! | [`changelog`] | release notes into blocks, blocks into lists and tables, when What's new pops up |
//! | [`version`] | versions compared by their numbers; a hotfix's release |
//! | [`update`] | when an open page reloads into a new build, waits for the service worker, backs off |
//! | [`notices`] | which notice banner shows, Check again's toast, how often to ask |
//! | [`offline`] | why the server can't be reached, offline mode's rules, the per-account outbox |
//! | [`uploads`] | the accept checks shared by the file picker and drag & drop, and upload limits |
//!
//! The user-visible words are `pub const`s (or small functions where Encore fills in a number or a name),
//! so a kit-wide parity test can check that every interface says the same thing.

pub mod changelog;
pub mod details;
pub mod notices;
pub mod offline;
pub mod toasts;
pub mod update;
pub mod uploads;
pub mod version;
