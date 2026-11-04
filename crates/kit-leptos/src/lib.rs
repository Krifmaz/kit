//! # kit-leptos
//!
//! Encore's web components as Leptos components (client-side rendering, WebAssembly), on top of kit-rules. Each
//! one draws exactly the HTML and CSS classes Encore's hand-written `web-rs` draws, so Encore's stylesheet is
//! reused unchanged and Encore looks the same when it switches; every decision comes from kit-rules and every
//! word is a kit-rules constant.
//!
//! | Module | Component | From Encore |
//! | --- | --- | --- |
//! | [`toasts`] | `#toasts`, toasts, error toasts with Copy and Close, Undo toasts, the update toast | `ui.rs`, `update.rs` |
//! | [`clipboard`] | copy with the plain-http fallback | `ui.rs` |
//! | [`modal`] | modals and the confirm dialog | `ui.rs` |
//! | [`details`] | the whole text of an error with Copy | `details.rs` |
//! | [`whats_new`] | the What's new popup and Settings → What's new | `whats_new.rs` |
//! | [`update`] | reloading into a new build (health polling, the service worker) | `update.rs`, `sw.rs` |
//! | [`notice`] | the notice banner with Check again | `notice.rs` |
//! | [`dropzone`] | drag and drop onto every upload | `dropzone.rs` |
//! | [`schema_form`] | the settings cards drawn from the schema | `settings/schema_form.rs`, `fields.rs`, `groups.rs` |
//! | [`bug_report`] | the New report sheet | `pages/bugreports.rs` |
//! | [`diagnostics`] | the Diagnostics page | `settings/diagnostics.rs` |
//! | [`offline`] | the offline and connection banners | `offline/mode.rs`, `app.rs` |
//!
//! The global functions ([`toasts::toast`], [`toasts::toast_action`], [`modal::open`],
//! [`modal::confirm_dialog`], [`details::show`], [`clipboard::copy_text`]) work from any code, Leptos or
//! hand-written, so Encore's pages keep calling them while they move over one by one. Server calls go through
//! [`api`], whose backend an app can replace ([`api::set_backend`]).

pub mod api;
pub mod bug_report;
pub mod clipboard;
pub mod details;
pub mod diagnostics;
pub mod dropzone;
pub mod html;
pub mod icons;
pub mod json;
pub mod modal;
pub mod notice;
pub mod offline;
pub mod rt;
pub mod schema_form;
pub mod toasts;
pub mod update;
pub mod whats_new;

#[cfg(test)]
pub(crate) mod testing;

/// What a Leptos app usually imports.
pub mod prelude {
    pub use crate::clipboard::copy_text;
    pub use crate::details::show as show_details;
    pub use crate::modal::{confirm_dialog, open as open_modal, ModalOpts, ModalRoot};
    pub use crate::toasts::{toast, toast_action, Toasts};
}
