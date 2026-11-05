//! # kit-iced
//!
//! Desktop building blocks for any app as [Iced](https://iced.rs) 0.14 components, styled from the kit's
//! design tokens (`design/out/iced_tokens.rs`). App names, words, routes and logos are the app's.
//!
//! | Module | What it gives an app |
//! | --- | --- |
//! | [`theme`] | the palettes (every theme and accent), [`theme::default_look`], and every widget style |
//! | [`widgets`] | text styles, buttons, cards, fields, modal, badges, skeletons, icons, text area |
//! | [`toast`] | toasts: errors stay with Copy, duplicates don't pile up, Undo, a cap on the stack |
//! | [`details`] | the details dialog: any text in full, selectable, with Copy |
//! | [`whats_new`] | the What's new popup and the changelog page |
//! | [`update`] | the update notice and the release checks (full release vs hotfix, SHA256SUMS) |
//! | [`notice`] | a plain-language notice banner with Check again |
//! | [`drop_zone`] | drag and drop for every upload, checked like the file dialog |
//! | [`settings`] | the pieces of a settings screen: card, help, switch, select, input, test result |
//! | [`form`] | a settings form drawn from a schema |
//! | [`bug_report`] | the bug report sheet (kind, title, description, files) and report labels |
//! | [`diagnostics`] | the diagnostics checks: status dot, name, message, fix, and a copyable report |
//! | [`offline`] | the offline banner |
//! | [`signup`] | the sign-up page and the admin's sign-up page: who can sign up, invites, waiting sign-ups |
//! | [`suite`] | linked apps: the shared sidebar with Edit sidebar, the search across apps, "related in …" |
//! | [`confirm`] | the confirm dialog: a question, Cancel and the action (red when it destroys something) |
//!
//! Components hold no network code: they keep their state, draw it and say what the app should do next,
//! and the app makes the requests with its own client. That keeps them usable by any app.

pub mod bug_report;
pub mod confirm;
pub mod details;
pub mod diagnostics;
pub mod drop_zone;
pub mod form;
pub mod json;
pub mod notice;
pub mod offline;
pub mod settings;
pub mod signup;
pub mod suite;
pub mod theme;
pub mod toast;
pub mod update;
pub mod whats_new;
pub mod widgets;

pub use widgets::El;
