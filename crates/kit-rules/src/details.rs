//! "Read the whole error": any cut-off line opens its whole text in a dialog, wrapped and selectable, with
//! Copy and Close.
//!
//! From Encore's `web-rs/src/details.rs` (`show`, `selecting`, `init`), the desktop's
//! `clients/desktop/src/widgets/details.rs` (`COPY`, `COPIED`, `COPY_FAILED`, `copied`) and Android's
//! `ui/components/Details.kt` (`DetailsDialog`, `rememberCopied`, `copyLabel`).
//!
//! Whether a line is cut off is left to the drawing (CSS `text-overflow: ellipsis`, Compose `maxLines`, Iced's
//! wrapping): Encore has no rule for it in code, so every status and error line carries the dialog, cut off or
//! not.

pub use crate::toasts::{COPIED, COPY, COPY_FAILED};

/// The dialog's title when the text came without one. From `web-rs/src/details.rs` `show` and `Details.kt`.
pub const DETAILS: &str = "Details";
/// The dialog's Close button. From `web-rs/src/details.rs` `footer_html`.
pub const CLOSE: &str = "Close";
/// How long a sheet's own Copy button says [`COPIED`] before it says [`COPY`] again, in ms.
/// From `Details.kt` `rememberCopied` (`delay(3000)`).
pub const COPIED_FOR_MS: u64 = 3000;

/// The dialog's title: `title`, or [`DETAILS`] when it is blank.
///
/// From `web-rs/src/details.rs` `show` (`title.trim().is_empty()`). Android's `DetailsDialog` only checks for an
/// empty title (`title.ifEmpty`); see [`title_android`].
pub fn title(title: &str) -> &str {
    if title.trim().is_empty() {
        DETAILS
    } else {
        title
    }
}

/// Android's title rule: `title`, or [`DETAILS`] when it is empty (a title of spaces stays).
///
/// From `Details.kt` `DetailsDialog` (`title.ifEmpty { "Details" }`).
pub fn title_android(title: &str) -> &str {
    if title.is_empty() {
        DETAILS
    } else {
        title
    }
}

/// What Copy puts on the clipboard: exactly the text the dialog shows, unchanged.
///
/// From `web-rs/src/details.rs` `footer_html` ("Copy (which copies exactly the text shown)") and the desktop's
/// `Details::update` (`Msg::Copy => Out::Copy(self.text.clone())`).
pub fn copy_text(text: &str) -> &str {
    text
}

/// Whether a click on a line opens the dialog: not when it ends a selection (dragging over a status line to
/// copy part of it). `selection` is the page's selected text at that moment.
///
/// From `web-rs/src/details.rs` `selecting` / `init`.
pub fn opens_on_click(selection: &str) -> bool {
    selection.trim().is_empty()
}

/// Whether a copy took: the clipboard, read back, holds what was sent (line endings and the ends' whitespace
/// aside). `got` is None when it could not be read.
///
/// From the desktop's `widgets/details.rs` `copied`.
pub fn copied(sent: &str, got: Option<&str>) -> bool {
    let norm = |s: &str| s.replace("\r\n", "\n").trim().to_string();
    got.is_some_and(|g| !g.is_empty() && norm(g) == norm(sent))
}

/// The Copy button's words in a sheet or dialog: [`COPIED`] for a moment after it worked (`Some(true)`), else
/// [`COPY`].
///
/// From `Details.kt` `copyLabel`.
pub fn copy_label(copied: Option<bool>) -> &'static str {
    if copied == Some(true) {
        COPIED
    } else {
        COPY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// From `web-rs/src/details.rs` `the_text_is_escaped_once_and_copied_as_shown` (the HTML part stays in
    /// kit-leptos): the dialog copies the very text it shows.
    #[test]
    fn the_text_is_copied_as_shown() {
        let t = "import failed: mkdir /music/<Band> & \"Co\": Input/output error (os error 5)\nline 2";
        assert_eq!(copy_text(t), t);
        assert_eq!(title("Request #12"), "Request #12");
        assert_eq!(title("  "), "Details");
        assert_eq!(title_android(""), "Details");
        assert_eq!(title_android("  "), "  ");
    }

    /// Port of the desktop's `a_copy_is_checked_against_what_the_clipboard_holds`.
    #[test]
    fn a_copy_is_checked_against_what_the_clipboard_holds() {
        assert!(copied("a\nb", Some("a\r\nb\n")));
        assert!(!copied("a", None));
        assert!(!copied("a", Some("")));
        assert!(!copied("a", Some("something older")));
        assert_eq!(COPY_FAILED, "Copy failed – select and copy manually");
    }

    #[test]
    fn a_selection_does_not_open_the_dialog() {
        assert!(opens_on_click(""));
        assert!(opens_on_click(" \n"));
        assert!(!opens_on_click("os error 5"));
    }

    #[test]
    fn the_copy_button_says_it_worked_for_a_moment() {
        assert_eq!(copy_label(None), "Copy");
        assert_eq!(copy_label(Some(false)), "Copy");
        assert_eq!(copy_label(Some(true)), "Copied to clipboard");
    }
}
