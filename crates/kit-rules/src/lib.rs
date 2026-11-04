//! # kit-rules
//!
//! The client-side decisions every kit app makes the same way on the web, the desktop and Android, written
//! once in plain Rust: no UI, no `web-sys`, no async runtime and no clock. Time is passed in as milliseconds
//! (`now_ms: u64`), so the same code runs in the browser (`wasm32-unknown-unknown`), in an Iced app and in
//! tests, and ports line by line to Kotlin.
//!
//! Nothing here belongs to one app: the app's id, name and version come in as [`app::App`] (or plain
//! parameters), and every word a person reads is a `pub const` default gathered in a `Words` struct per
//! module ([`toasts::ToastWords`], [`changelog::ChangelogWords`], …), which an app replaces in part or in whole.
//! Where platforms differ on purpose (the web keeps three error toasts, Android ten), the difference is a
//! policy struct with named presets per platform (`web()`, `desktop()`, `android()`), never a silent choice.
//!
//! | Module | Decides |
//! | --- | --- |
//! | [`app`] | an app's id, name and version; storage keys; filling word templates |
//! | [`toasts`] | errors stay with Copy; duplicates merge; Undo; stack caps; how long each kind stays |
//! | [`dialogs`] | the words every dialog shares; the confirm dialog's buttons and answer |
//! | [`details`] | the "read the whole error" dialog: its title, what Copy copies, whether a copy took |
//! | [`changelog`] | release notes into blocks, blocks into lists and tables, when What's new pops up |
//! | [`version`] | versions compared by their numbers; a hotfix's release |
//! | [`update`] | when an open page reloads into a new build, waits for the service worker, backs off |
//! | [`notices`] | which notice banners show, Check again's toast, how often to ask |
//! | [`offline`] | why the server can't be reached, offline mode's rules, the per-account outbox of app actions |
//! | [`uploads`] | the accept checks shared by the file picker and drag & drop, and upload limits |
//! | [`bugs`] | bug reports: kinds, statuses, the New report sheet's words and checks |
//! | [`diagnostics`] | the Diagnostics page's groups, words and copied report |

pub mod app;
pub mod bugs;
pub mod changelog;
pub mod details;
pub mod diagnostics;
pub mod dialogs;
pub mod notices;
pub mod offline;
pub mod toasts;
pub mod update;
pub mod uploads;
pub mod version;
