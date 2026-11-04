//! `.toast`: short messages bottom right, gone after four seconds. An error stays until it is
//! dismissed and carries a Copy button, so it can be read in full and pasted into a report. A toast
//! can carry one action ("Up next cleared · Undo", the web's `toast_action`) and then stays as long
//! as the action can be taken.
//!
//! [`Toasts`] is the stack an app keeps (one per window): [`Toasts::push`] applies the rules
//! (a repeated error does not pile up, at most [`Toast::STAYING`] errors stay, at most
//! [`Toasts::MAX`] toasts show), [`Toasts::expire`] runs on the app's tick, and [`view`] draws the
//! stack over the window. The action's payload `A` is the app's own message for it.

use crate::theme;
use crate::widgets::{btn_sm, t, El};
use iced::widget::{button, column, container, row, space};
use iced::{Alignment, Background, Border, Color, Length};
use kit_rules::toasts::{ToastAction, ToastPolicy, ToastStack};
use std::time::{Duration, Instant};

pub use kit_rules::toasts::{Toast, ToastKind};

/// How long a toast stays (`toast()` in web-rs/src/ui.rs: 3.8 s then fades).
pub const LIFE: Duration = Duration::from_millis(kit_rules::toasts::WEB_SHOW_MS);

/// At most this many errors stay on screen; a new one pushes out the oldest (the web's).
pub const STAYING: usize = kit_rules::toasts::WEB_ERROR_TOASTS;

/// The toasts on screen, oldest first, under kit-rules' desktop policy
/// ([`ToastPolicy::desktop`]), on the app's own clock.
#[derive(Clone, Debug)]
pub struct Toasts<A> {
    stack: ToastStack<A>,
    start: Instant,
}

impl<A> Default for Toasts<A> {
    fn default() -> Self {
        Toasts { stack: ToastStack::new(ToastPolicy::desktop()), start: Instant::now() }
    }
}

impl<A: Clone> Toasts<A> {
    /// At most this many toasts show; a new one pushes out the oldest.
    pub const MAX: usize = kit_rules::toasts::DESKTOP_MAX_TOASTS;

    fn now_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    /// Show a toast.
    pub fn push(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.push_with(text, kind, None);
    }

    /// A toast with one action ("Up next cleared · Undo"): it stays `stay`, its button gives `then`
    /// back through [`Toasts::take_action`].
    pub fn push_action(&mut self, text: impl Into<String>, label: &str, then: A, stay: Duration) {
        self.push_with(text, ToastKind::Ok, Some((label.to_string(), then, stay)));
    }

    /// Show a toast, with or without an action (its button, what it gives back, how long it stays).
    pub fn push_with(&mut self, text: impl Into<String>, kind: ToastKind, action: Option<(String, A, Duration)>) {
        let now = self.now_ms();
        let (action, life) = match action {
            Some((label, payload, stay)) => (Some(ToastAction { label, payload }), Some(stay.as_millis() as u64)),
            None => (None, None),
        };
        self.stack.add(text, kind, action, life, now);
    }

    /// Drop the toasts whose time is up (call it on the app's tick).
    pub fn expire(&mut self) {
        let now = self.now_ms();
        self.stack.tick(now);
    }

    /// Close one toast (a click on it).
    pub fn dismiss(&mut self, id: u64) {
        self.stack.close(id);
    }

    /// The words of one toast, for its Copy button.
    pub fn text(&self, id: u64) -> Option<String> {
        self.list().iter().find(|t| t.id == id).map(|t| t.text.clone())
    }

    /// Take a toast's action: the toast closes and its payload comes back.
    pub fn take_action(&mut self, id: u64) -> Option<A> {
        self.stack.take_action(id)
    }

    /// The toasts, oldest first.
    pub fn list(&self) -> &[Toast<A>] {
        self.stack.items()
    }

    /// Nothing to show.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

/// The toast column, to be laid over the window bottom right, `bottom` px above the bottom edge (a
/// player bar's height plus 18; Encore passes [`theme::PLAYER_H`] + 18). `on_dismiss(id)` closes one
/// early; `on_copy(id)` copies an error's text; `on_action(id)` takes a toast's action.
pub fn view<'a, A, M: Clone + 'a>(
    toasts: &'a [Toast<A>],
    bottom: f32,
    on_dismiss: impl Fn(u64) -> M,
    on_copy: impl Fn(u64) -> M,
    on_action: impl Fn(u64) -> M,
) -> El<'a, M> {
    let p = theme::look();
    let mut col = column![].spacing(8).align_x(Alignment::End);
    for toast in toasts {
        let edge = match toast.kind {
            ToastKind::Info | ToastKind::Plain => p.accent,
            ToastKind::Ok => p.ok,
            ToastKind::Warn => p.warn,
            ToastKind::Err => p.err,
        };
        let words = t(toast.text.clone()).size(13.5);
        let inner: El<'a, M> = if toast.stays() {
            column![words, row![space().width(Length::Fill), crate::details::copy_btn(on_copy(toast.id))]].spacing(4).into()
        } else if let Some(a) = toast.action.as_ref() {
            row![words, space().width(Length::Fixed(12.0)), btn_sm(a.label.as_str(), theme::Btn::Default, Some(on_action(toast.id)))]
                .align_y(Alignment::Center)
                .into()
        } else {
            words.into()
        };
        let card = container(inner).padding([12, 16]).max_width(380).style(theme::toast);
        let bar = container(space()).width(4).height(Length::Fill).style(move |_| container::Style {
            background: Some(Background::Color(edge)),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: iced::border::Radius::new(0.0).top_left(12.0).bottom_left(12.0),
            },
            ..container::Style::default()
        });
        col = col.push(
            button(row![bar, card].height(Length::Shrink).align_y(Alignment::Center))
                .padding(0)
                .style(theme::btn(theme::Btn::Row))
                .on_press(on_dismiss(toast.id)),
        );
    }
    container(col).width(Length::Fill).height(Length::Fill).align_x(Alignment::End).align_y(Alignment::End).padding([bottom, 18.0]).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stack_keeps_its_caps() {
        let mut s = Toasts::<u8>::default();
        s.push("Disk full", ToastKind::Err);
        s.push("Disk full", ToastKind::Err);
        assert_eq!(s.list().len(), 1, "the same error again does not pile up");
        for e in ["a", "b", "c"] {
            s.push(e, ToastKind::Err);
        }
        let staying: Vec<_> = s.list().iter().filter(|t| t.stays()).map(|t| t.text.as_str()).collect();
        assert_eq!(staying, ["a", "b", "c"], "at most three errors stay: the oldest goes");
        for i in 0..4 {
            s.push(format!("ok {i}"), ToastKind::Ok);
        }
        assert_eq!(s.list().len(), Toasts::<u8>::MAX);
        assert_eq!(s.list()[0].text, "c");
        // the failed copy says what to do instead: it goes like any other toast
        s.push(crate::details::COPY_FAILED, ToastKind::Err);
        assert!(!s.list().last().unwrap().stays());
        s.expire();
        assert!(s.list().iter().any(|t| t.kind == ToastKind::Ok), "4.2 s have not passed");
    }

    #[test]
    fn an_action_comes_back_once_and_keeps_its_time() {
        let mut s = Toasts::default();
        s.push_action("Up next cleared", "Undo", 42u8, Duration::from_secs(6));
        let id = s.list()[0].id;
        assert_eq!(s.list()[0].life_ms, 6000, "a toast with Undo stays as long as Undo can be pressed");
        assert_eq!(s.text(id).as_deref(), Some("Up next cleared"));
        assert_eq!(s.take_action(id), Some(42));
        assert!(s.is_empty());
        assert_eq!(s.take_action(id), None);
        assert_eq!(LIFE, Duration::from_millis(4200));
    }
}
