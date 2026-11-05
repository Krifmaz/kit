//! Readable errors: the details dialog that shows a text in full – wrapped, selectable, with a
//! Copy button – and the small Copy button the error lines carry (the web's details modal with
//! its `<pre>` and `ui::copy_text`). The app holds the one open dialog; a screen only builds the
//! text – and, when there is one, the one thing that fixes it ([`Act`], e.g. "Keep anyway"),
//! which the app sends itself. `P` is the app's page type, for [`Links`].
//!
//! After a copy the app reads the clipboard back and checks it with [`copied`], then shows
//! [`COPIED`] or [`COPY_FAILED`].

use crate::theme::{self, Btn};
use crate::widgets::{self as w, btn, btn_icon, El, Icon};
use iced::widget::text::Wrapping;
use iced::widget::text_editor::{self, Action, Content};
use iced::widget::{button, column, row};
use iced::{Alignment, Length};
use serde_json::Value;

pub use kit_rules::toasts::{COPIED, COPY, COPY_FAILED};

/// A button beside Copy that acts on what the dialog shows: `POST path` with `body`, then the
/// `done` toast (the server's own words when it says no). The page's list shows the change on
/// its next reload.
#[derive(Clone, Debug, PartialEq)]
pub struct Act {
    /// The button's label ("Keep anyway").
    pub label: String,
    /// The API path the app posts to.
    pub path: String,
    /// The body it posts.
    pub body: Value,
    /// The toast when it worked.
    pub done: String,
}

/// A few words above the text, in a warning box: why (in full) and where it came from (e.g. the
/// reason a check turned an item away).
#[derive(Clone, Debug, PartialEq)]
pub struct Lead {
    /// Why, in full (semibold).
    pub text: String,
    /// Where it came from (small, muted); "" for none.
    pub from: String,
}

/// What a dialog links to: a title and a subtitle as links to their pages (above the text), and a
/// main button that opens a page (e.g. "Open" on the item the text is about).
#[derive(Clone, Debug, PartialEq)]
pub struct Links<P> {
    /// The title and the page it opens.
    pub title: Option<(String, P)>,
    /// The subtitle (muted) and the page it opens.
    pub subtitle: Option<(String, P)>,
    /// The main button: its label and the page it opens.
    pub button: Option<(String, P)>,
}

impl<P> Default for Links<P> {
    fn default() -> Self {
        Links { title: None, subtitle: None, button: None }
    }
}

/// The text a dialog shows, in an editor that selects but never edits.
#[derive(Debug)]
pub struct Details<P> {
    /// The dialog's title.
    pub title: String,
    /// The text, in full.
    pub text: String,
    /// The words above the text, when there are any.
    pub lead: Option<Lead>,
    /// The button that acts on it, when there is one.
    pub act: Option<Act>,
    /// The links and the main button.
    pub links: Links<P>,
    /// The icon on the main button of the [`Links`], when it has one ([`Details::with_button_icon`]).
    pub button_icon: Option<Icon>,
    content: Content,
}

/// The dialog's messages.
#[derive(Clone, Debug)]
pub enum Msg<P> {
    /// Selection, cursor moves and scrolling in the text (edits are dropped).
    Text(Action),
    /// Copy the text.
    Copy,
    /// The dialog's [`Act`] button.
    Act,
    /// A link or the main button of the [`Links`]: open that page.
    Open(P),
    /// Close (the X, the backdrop, Escape, the Close button).
    Close,
}

/// What the app does after a message.
#[derive(Debug, PartialEq)]
pub enum Out<P> {
    /// Copy this text.
    Copy(String),
    /// Close the dialog and send this.
    Act(Act),
    /// Close the dialog and open this page.
    Open(P),
}

impl<P: Clone> Details<P> {
    /// A dialog showing `text` under `title`.
    pub fn new(title: String, text: String, lead: Option<Lead>, act: Option<Act>) -> Details<P> {
        let content = Content::with_text(&text);
        Details { title, text, lead, act, links: Links::default(), button_icon: None, content }
    }

    /// The same dialog with [`Links`].
    pub fn with_links(mut self, links: Links<P>) -> Details<P> {
        self.links = links;
        self
    }

    /// The same dialog with `icon` on the main button of its [`Links`] (e.g. [`Icon::Play`] on "Play").
    pub fn with_button_icon(mut self, icon: Icon) -> Details<P> {
        self.button_icon = Some(icon);
        self
    }

    /// The text as the editor holds it (its selection, for tests and for apps that copy a part).
    pub fn content(&self) -> &Content {
        &self.content
    }

    /// Apply a message; `Some(out)` asks the app to copy the text or to send the action.
    pub fn update(&mut self, msg: Msg<P>) -> Option<Out<P>> {
        match msg {
            Msg::Text(a) if !a.is_edit() => self.content.perform(a),
            Msg::Text(_) | Msg::Close => {}
            Msg::Copy => return Some(Out::Copy(self.text.clone())),
            Msg::Act => return self.act.clone().map(Out::Act),
            Msg::Open(p) => return Some(Out::Open(p)),
        }
        None
    }
}

pub use kit_rules::details::copied;

/// The dialog: the lead when there is one, the text in full (monospace, wrapped, selectable),
/// Close and Copy (and the action, when the text came with one).
pub fn view<P: Clone>(d: &Details<P>) -> El<'_, Msg<P>> {
    let lines = d.text.lines().count().max(1) as f32;
    let editor = text_editor::TextEditor::new(&d.content)
        .on_action(Msg::Text)
        .font(theme::mono())
        .size(12.5)
        .padding(12)
        .wrapping(Wrapping::WordOrGlyph)
        .height(Length::Fixed((lines * 19.0 + 24.0).clamp(60.0, 480.0)))
        .style(w::text_area::style);
    let mut body = column![].spacing(12).width(Length::Fill);
    // what the text is about, as links: the title · the subtitle
    let links: Vec<El<'_, Msg<P>>> = [(&d.links.title, true), (&d.links.subtitle, false)]
        .into_iter()
        .filter_map(|(l, title)| l.as_ref().map(|(label, page)| (label.clone(), page.clone(), title)))
        .map(|(label, page, title)| if title { w::link(label, 14.0, Msg::Open(page)) } else { w::muted_link(label, 14.0, Msg::Open(page)) })
        .collect();
    if !links.is_empty() {
        let mut r = row![].spacing(6).align_y(Alignment::Center);
        for (i, l) in links.into_iter().enumerate() {
            if i > 0 {
                r = r.push(w::small_muted("·"));
            }
            r = r.push(l);
        }
        body = body.push(r);
    }
    if let Some(l) = &d.lead {
        let mut words = column![w::t(l.text.clone()).size(14).font(theme::semibold()).wrapping(Wrapping::WordOrGlyph)].spacing(3);
        if !l.from.is_empty() {
            words = words.push(w::small_muted(l.from.clone()).wrapping(Wrapping::WordOrGlyph));
        }
        body = body.push(iced::widget::container(words).padding([10, 14]).width(Length::Fill).style(theme::warn_box));
    }
    body = body.push(editor);
    // the action (Keep anyway) is the dialog's main button; Copy is then a plain one
    let main = d.act.is_some() || d.links.button.is_some();
    let mut footer = row![
        btn(kit_rules::details::CLOSE, Btn::Default, Some(Msg::Close)),
        btn_icon(Icon::Copy, COPY, if main { Btn::Default } else { Btn::Primary }, Some(Msg::Copy)),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if let Some((label, page)) = &d.links.button {
        let open = Some(Msg::Open(page.clone()));
        footer = footer.push(match d.button_icon {
            Some(i) => btn_icon(i, label.clone(), Btn::Primary, open),
            None => btn(label.clone(), Btn::Primary, open),
        });
    }
    if let Some(a) = &d.act {
        footer = footer.push(btn(a.label.clone(), Btn::Primary, Some(Msg::Act)));
    }
    w::modal(d.title.clone(), body, Some(footer.into()), true, Msg::Close)
}

/// The small Copy button beside an error line.
pub fn copy_btn<'a, M: Clone + 'a>(on: M) -> El<'a, M> {
    let p = theme::look();
    button(row![w::icon(Icon::Copy, 13.0, p.muted), w::t(COPY).size(12).color(p.muted)].spacing(5).align_y(Alignment::Center))
        .padding([3, 7])
        .style(theme::btn(Btn::Ghost))
        .on_press(on)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_is_checked_against_what_the_clipboard_holds() {
        assert!(copied("a\nb", Some("a\r\nb\n")));
        assert!(!copied("a", None));
        assert!(!copied("a", Some("")));
        assert!(!copied("a", Some("something older")));
        assert_eq!(COPY_FAILED, "Copy failed – select and copy manually");
    }

    #[test]
    fn the_text_selects_but_never_changes() {
        let mut d: Details<()> = Details::new("Request details".into(), "line one\nline two".into(), None, None);
        assert_eq!(d.update(Msg::Text(Action::Edit(text_editor::Edit::Insert('x')))), None);
        assert_eq!(d.content.text().trim_end(), "line one\nline two");
        assert_eq!(d.update(Msg::Text(Action::SelectAll)), None);
        assert_eq!(d.content.selection().as_deref(), Some("line one\nline two"));
        assert_eq!(d.update(Msg::Copy), Some(Out::Copy("line one\nline two".into())));
        // no action: its message does nothing
        assert_eq!(d.update(Msg::Act), None);
    }

    #[test]
    fn the_action_goes_to_the_app() {
        let act = Act {
            label: "Keep anyway".into(),
            path: "/items/12/keep-anyway".into(),
            body: serde_json::json!({ "rejected": 1 }),
            done: "It will be kept without the check".into(),
        };
        let mut d: Details<()> = Details::new("Item 12".into(), "Rejected by the size check".into(), None, Some(act.clone()));
        assert_eq!(d.update(Msg::Act), Some(Out::Act(act)));
    }

    #[test]
    fn the_links_and_the_main_button_open_pages() {
        let links = Links {
            title: Some(("Report.pdf".into(), 1u8)),
            subtitle: Some(("Documents".into(), 2u8)),
            button: Some(("Open".into(), 3u8)),
        };
        let mut d: Details<u8> = Details::new("Import".into(), "done".into(), None, None).with_links(links);
        assert_eq!(d.update(Msg::Open(3)), Some(Out::Open(3)));
        {
            let mut ui = iced_test::simulator(view(&d));
            assert!(ui.find("Report.pdf").is_ok() && ui.find("Documents").is_ok());
            assert!(ui.click("Open").is_ok());
        }
        // the main button can carry an icon and still opens its page
        let d = d.with_button_icon(Icon::Play);
        assert_eq!(d.button_icon, Some(Icon::Play));
        let mut ui = iced_test::simulator(view(&d));
        assert!(ui.click("Open").is_ok());
        assert_eq!(ui.into_messages().filter(|m| matches!(m, Msg::Open(3))).count(), 1);
    }
}
