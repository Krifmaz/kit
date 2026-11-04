//! Dialogs: the words every dialog shares, and the confirm dialog ("Delete this?" with Cancel and OK).
//!
//! A confirm dialog asks one question before something happens: its title, one line of text, Cancel and the OK
//! button, which says what will happen ("Delete") and is red when that cannot be undone ([`Confirm::danger`]).
//! Any way of closing it but OK (Cancel, Escape, a press beside it, a new page) answers no ([`Answer`]).

pub use crate::details::CLOSE;

/// A confirm dialog's (and a sheet's) dismiss button.
pub const CANCEL: &str = "Cancel";
/// A confirm dialog's OK button when the app names no action.
pub const OK: &str = "OK";

/// The words dialogs share. [`DialogWords::DEFAULT`] holds the kit's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DialogWords {
    /// The dismiss button ([`CANCEL`]).
    pub cancel: &'static str,
    /// The close button and the × of a dialog's head ([`CLOSE`]).
    pub close: &'static str,
    /// The OK button when a confirm dialog names no action ([`OK`]).
    pub ok: &'static str,
}

impl DialogWords {
    /// The kit's words.
    pub const DEFAULT: DialogWords = DialogWords { cancel: CANCEL, close: CLOSE, ok: OK };
}

impl Default for DialogWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// What a confirm dialog asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    /// The question ("Delete this report?").
    pub title: String,
    /// One line about what happens ("It is gone for good."), "" for none.
    pub text: String,
    /// The OK button's words: what will happen ("Delete"); "" for [`DialogWords::ok`].
    pub ok: String,
    /// The Cancel button's words; "" for [`DialogWords::cancel`].
    pub cancel: String,
    /// The action cannot be undone: the OK button is drawn as a danger button.
    pub danger: bool,
}

impl Confirm {
    /// A confirm dialog asking `title`, with no text, the default buttons and no danger.
    pub fn new(title: impl Into<String>) -> Self {
        Confirm { title: title.into(), text: String::new(), ok: String::new(), cancel: String::new(), danger: false }
    }

    /// With this line of text.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// With this OK button.
    pub fn ok(mut self, label: impl Into<String>) -> Self {
        self.ok = label.into();
        self
    }

    /// With this Cancel button.
    pub fn cancel(mut self, label: impl Into<String>) -> Self {
        self.cancel = label.into();
        self
    }

    /// Whether OK is a danger button.
    pub fn danger(mut self, danger: bool) -> Self {
        self.danger = danger;
        self
    }

    /// The OK button's words under `words`.
    pub fn ok_label<'a>(&'a self, words: &DialogWords) -> &'a str {
        if self.ok.trim().is_empty() {
            words.ok
        } else {
            &self.ok
        }
    }

    /// The Cancel button's words under `words`.
    pub fn cancel_label<'a>(&'a self, words: &DialogWords) -> &'a str {
        if self.cancel.trim().is_empty() {
            words.cancel
        } else {
            &self.cancel
        }
    }

    /// The OK button's style: `danger` or `primary`.
    pub fn ok_style(&self) -> &'static str {
        if self.danger {
            "danger"
        } else {
            "primary"
        }
    }
}

/// The answer of a confirm dialog, once: the first way it closes decides, later ones change nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Answer(Option<bool>);

impl Answer {
    /// OK was pressed: yes (unless it was answered already). Returns whether this decided it.
    pub fn ok(&mut self) -> bool {
        self.set(true)
    }

    /// It closed another way (Cancel, Escape, a press beside it, a new page): no (unless it was answered
    /// already). Returns whether this decided it.
    pub fn dismissed(&mut self) -> bool {
        self.set(false)
    }

    fn set(&mut self, yes: bool) -> bool {
        if self.0.is_some() {
            return false;
        }
        self.0 = Some(yes);
        true
    }

    /// The answer: None while the dialog is open.
    pub fn get(&self) -> Option<bool> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words() {
        assert_eq!((CANCEL, CLOSE, OK), ("Cancel", "Close", "OK"));
        assert_eq!(DialogWords::default(), DialogWords::DEFAULT);
    }

    #[test]
    fn a_confirm_dialog_names_its_buttons() {
        let w = DialogWords::DEFAULT;
        let plain = Confirm::new("Leave this page?");
        assert_eq!((plain.ok_label(&w), plain.cancel_label(&w), plain.ok_style()), ("OK", "Cancel", "primary"));
        let del = Confirm::new("Delete this report?").text("It is gone for good.").ok("Delete").cancel("Keep").danger(true);
        assert_eq!((del.ok_label(&w), del.cancel_label(&w), del.ok_style()), ("Delete", "Keep", "danger"));
        assert_eq!(del.text, "It is gone for good.");
        let other = DialogWords { ok: "Ja", cancel: "Nein", close: "Schließen" };
        assert_eq!((plain.ok_label(&other), plain.cancel_label(&other)), ("Ja", "Nein"));
    }

    #[test]
    fn the_first_way_it_closes_answers() {
        let mut a = Answer::default();
        assert_eq!(a.get(), None);
        assert!(a.ok());
        assert!(!a.dismissed(), "closing after OK changes nothing");
        assert_eq!(a.get(), Some(true));
        let mut b = Answer::default();
        assert!(b.dismissed());
        assert!(!b.ok());
        assert_eq!(b.get(), Some(false));
    }
}
