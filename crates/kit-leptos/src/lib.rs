//! # kit-leptos
//!
//! The kit's web components as Leptos components (client-side rendering, WebAssembly), on top of kit-rules:
//! every decision comes from kit-rules, every word is a kit-rules default an app may change, and every server
//! path is configured per app ([`config`]). The markup uses the class names of the kit's own stylesheet
//! ([`CSS`], `style/kit.css`), which is built only on the design tokens (`design/out/tokens.css`), so each of the
//! themes (`encore`, `dark`, `oled`, `light`) draws every component.
//!
//! | Module | Component |
//! | --- | --- |
//! | [`config`] | the app's id, name, version, logo, paths, routes and words |
//! | [`toasts`] | `#toasts`: toasts, errors with Copy and Close, Undo toasts, the update toast |
//! | [`clipboard`] | copy, with the plain-http fallback |
//! | [`modal`] | modals, the confirm dialog ([`modal::confirm`], [`modal::ConfirmDialog`]) |
//! | [`details`] | the whole text of an error with Copy |
//! | [`whats_new`] | the What's new popup and the changelog page |
//! | [`update`] | reloading into a new build (health polling, the service worker) |
//! | [`notice`] | the notice banners with Check again |
//! | [`dropzone`] | drag and drop onto every upload |
//! | [`schema_form`] | the settings cards drawn from the server's schema |
//! | [`bug_report`] | the New report sheet |
//! | [`diagnostics`] | the Diagnostics page |
//! | [`offline`] | the offline and connection banners |
//! | [`palette`] | the command palette (Ctrl+K / ⌘K): the app's commands and the ones it loads on opening |
//! | [`signup`] | the sign-in page's sign-up line, the sign-up and confirmation pages, the admin's sign-up page |
//! | [`suite`] | the shared sidebar of linked apps, search across apps, related links, Sign in with …, Linked apps |
//!
//! An app sets its configuration once ([`config::set`]); the global functions ([`toasts::toast`],
//! [`toasts::toast_action`], [`modal::open`], [`modal::confirm`], [`palette::open`], [`details::show`], [`clipboard::copy_text`])
//! work from any code, Leptos or hand-written. Server calls go through [`api`], whose backend an app can replace
//! ([`api::set_backend`]).

/// The kit's stylesheet (`style/kit.css`): every component's classes in the kit's look (docs/DESIGN.md §5), on the
/// variables of `tokens.css`. Load
/// `tokens.css` first, then this (trunk: `<link data-trunk rel="css" href="…/kit.css">`).
pub const CSS: &str = include_str!("../style/kit.css");

pub mod api;
pub mod bug_report;
pub mod clipboard;
pub mod config;
pub mod crash;
pub mod details;
pub mod diagnostics;
pub mod dropzone;
pub mod html;
pub mod icons;
pub mod json;
pub mod modal;
pub mod notice;
pub mod offline;
pub mod palette;
pub mod rt;
pub mod schema_form;
pub mod signup;
pub mod suite;
pub mod toasts;
pub mod update;
pub mod whats_new;

#[cfg(test)]
pub(crate) mod testing;

/// What a Leptos app usually imports.
pub mod prelude {
    pub use crate::clipboard::copy_text;
    pub use crate::config::AppConfig;
    pub use crate::details::show as show_details;
    pub use crate::modal::{confirm, confirm_dialog, open as open_modal, ConfirmDialog, ModalOpts, ModalRoot};
    pub use crate::palette::{open as open_palette, Command, Palette};
    pub use crate::toasts::{toast, toast_action, Toasts};
    pub use kit_rules::app::App;
    pub use kit_rules::dialogs::Confirm;
}
