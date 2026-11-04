//! The confirm dialog: before an action that can't be undone (or simply matters), a question, a line
//! saying what happens, [`CANCEL`] and the action's own button, red when it destroys something.
//!
//! The app keeps one `Option<Confirm<A>>`: [`Confirm::new`] when the person asks for the action,
//! [`view`] over the page while it is open, and [`Confirm::update`] for its messages, which hands the
//! action `A` back on OK (the app then does it) and nothing on Cancel, the X, the backdrop or Escape.
//! Either way the app drops the dialog.
//!
//! ```
//! use kit_iced::confirm::{Confirm, Msg};
//! #[derive(Clone, Debug, PartialEq)]
//! enum Action { Delete(u32) }
//! let d = Confirm::new("Delete report?", "It's gone for everyone. This can't be undone.", "Delete", Action::Delete(7)).danger();
//! assert_eq!(d.update(Msg::Ok), Some(Action::Delete(7)));
//! assert_eq!(d.update(Msg::Cancel), None);
//! ```

use crate::theme::Btn;
use crate::widgets::{btn, modal, muted, El};
use iced::widget::row;

pub use kit_rules::dialogs::CANCEL;

/// An open confirm dialog for the action `A`.
#[derive(Clone, Debug, PartialEq)]
pub struct Confirm<A> {
    /// The question ("Delete report?").
    pub title: String,
    /// What happens, in a sentence.
    pub text: String,
    /// The action's button ("Delete").
    pub ok: String,
    /// The action destroys something: its button is red.
    pub danger: bool,
    /// What the app does on OK.
    pub action: A,
}

/// The dialog's messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    /// The action's button.
    Ok,
    /// Cancel, the X, the backdrop or Escape.
    Cancel,
}

impl<A: Clone> Confirm<A> {
    /// A dialog asking `title`, saying `text`, with `ok` as the action's button.
    pub fn new(title: impl Into<String>, text: impl Into<String>, ok: impl Into<String>, action: A) -> Confirm<A> {
        Confirm { title: title.into(), text: text.into(), ok: ok.into(), danger: false, action }
    }

    /// The same dialog with a red action button (it deletes or removes something).
    pub fn danger(mut self) -> Confirm<A> {
        self.danger = true;
        self
    }

    /// The action to do after `msg` (OK), or None (Cancel). The app closes the dialog after either.
    pub fn update(&self, msg: Msg) -> Option<A> {
        match msg {
            Msg::Ok => Some(self.action.clone()),
            Msg::Cancel => None,
        }
    }
}

/// The dialog: the title, the sentence, then [`CANCEL`] and the action's button.
pub fn view<A>(d: &Confirm<A>) -> El<'_, Msg> {
    let footer = row![
        btn(CANCEL, Btn::Default, Some(Msg::Cancel)),
        btn(d.ok.as_str(), if d.danger { Btn::Danger } else { Btn::Primary }, Some(Msg::Ok)),
    ]
    .spacing(10);
    modal(d.title.as_str(), muted(d.text.as_str()).size(14), Some(footer.into()), false, Msg::Cancel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_hands_the_action_back_and_cancel_nothing() {
        let d = Confirm::new("Sign out everywhere?", "Every device signs in again.", "Sign out", 3u8);
        assert!(!d.danger);
        assert!(d.clone().danger().danger);
        assert_eq!(d.update(Msg::Ok), Some(3));
        assert_eq!(d.update(Msg::Cancel), None);
    }

    #[test]
    fn the_dialog_shows_the_words_and_sends_the_buttons() {
        let d = Confirm::new("Delete report?", "This can't be undone.", "Delete", ()).danger();
        let mut ui = iced_test::simulator(view(&d));
        assert!(ui.find("Delete report?").is_ok());
        assert!(ui.find("This can't be undone.").is_ok());
        assert!(ui.click(CANCEL).is_ok());
        assert!(ui.click("Delete").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [Msg::Cancel, Msg::Ok]);
    }
}
