//! The words every dialog shares.
//!
//! From Encore's `web-rs/src/ui.rs` (`modal`, `confirm_dialog_html`).

pub use crate::details::CLOSE;

/// A confirm dialog's (and a sheet's) dismiss button. From `web-rs/src/ui.rs` `confirm_dialog_html`.
pub const CANCEL: &str = "Cancel";

#[cfg(test)]
mod tests {
    #[test]
    fn the_words() {
        assert_eq!((super::CANCEL, super::CLOSE), ("Cancel", "Close"));
    }
}
