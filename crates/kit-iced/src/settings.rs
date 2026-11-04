//! The pieces every settings screen draws: the titled card,
//! help lines, switches, selects, inputs, test results, links and the two-column row.

use crate::theme::{self, Btn};
use crate::widgets::{self, icon, t, El, Icon};
use iced::widget::text::IntoFragment;
use iced::widget::{button, column, container, row, space, text_input, toggler, Text};
use iced::{Alignment, Length};

/// A switch with its words on the right.
pub fn switch<'a, M: Clone + 'a>(on: bool, label: impl IntoFragment<'a>, f: impl Fn(bool) -> M + 'a) -> El<'a, M> {
    toggler(on).label(label).text_size(14).spacing(10).on_toggle(f).style(theme::switch).into()
}

/// A switch that can be read-only (`None`: set by an environment variable, or not yours).
pub fn switch_maybe<'a, M: Clone + 'a, F: Fn(bool) -> M + 'a>(on: bool, label: impl IntoFragment<'a>, f: Option<F>) -> El<'a, M> {
    let dim = f.is_none();
    let p = theme::look();
    let w = toggler(on).label(label).text_size(14).spacing(10).on_toggle_maybe(f).style(theme::switch);
    if dim {
        container(w).style(move |_| container::Style { text_color: Some(p.muted), ..container::Style::default() }).into()
    } else {
        w.into()
    }
}

/// One option of a [`select`] built at run time (device lists, filters the engine offers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opt {
    /// What is saved.
    pub key: String,
    /// What is shown.
    pub label: String,
}

impl Opt {
    /// An option.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self { key: key.into(), label: label.into() }
    }
}

impl std::fmt::Display for Opt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// `select.input` over run-time options; `selected` is a key (an unknown one shows nothing).
pub fn select<'a, M: Clone + 'a>(options: Vec<Opt>, selected: &str, on_pick: impl Fn(String) -> M + 'a) -> El<'a, M> {
    let current = options.iter().find(|o| o.key == selected).cloned();
    iced::widget::pick_list(options, current, move |o: Opt| on_pick(o.key))
        .text_size(13)
        .padding([7, 10])
        .style(theme::pick)
        .menu_style(theme::pick_menu)
        .into()
}

/// [`select`] over a `(key, label)` table.
pub fn select_table<'a, M: Clone + 'a>(table: &[(&str, &str)], selected: &str, on_pick: impl Fn(String) -> M + 'a) -> El<'a, M> {
    select(table.iter().map(|(k, l)| Opt::new(*k, *l)).collect(), selected, on_pick)
}

/// `.input`: a one-line text box (add `.secure(true)`, `.on_submit(..)`, `.width(..)`).
pub fn input<'a, M: Clone + 'a>(placeholder: &str, value: &str, on_input: impl Fn(String) -> M + 'a) -> text_input::TextInput<'a, M> {
    text_input(placeholder, value).on_input(on_input).padding([9, 12]).size(14).style(theme::input)
}

/// A read-only `.input` (set by an environment variable).
pub fn input_locked<'a, M: Clone + 'a>(value: &str) -> text_input::TextInput<'a, M> {
    text_input("", value).padding([9, 12]).size(14).style(theme::input)
}

/// `.small.muted`: a help line under a field or a heading.
pub fn help<'a>(s: impl IntoFragment<'a>) -> Text<'a> {
    t(s).size(12.5).color(theme::look().muted)
}

/// A small line in the warning colour.
pub fn warn_text<'a>(s: impl IntoFragment<'a>) -> Text<'a> {
    t(s).size(12.5).color(theme::look().warn)
}

/// A small line in the error colour.
pub fn err_text<'a>(s: impl IntoFragment<'a>) -> Text<'a> {
    t(s).size(12.5).color(theme::look().err)
}

/// A titled block inside a card: `h3` over its lines.
pub fn block<'a, M: 'a>(title: &'a str, lines: Vec<El<'a, M>>) -> El<'a, M> {
    let mut c = column![widgets::h3(title)].spacing(8);
    for l in lines {
        c = c.push(l);
    }
    c.width(Length::Fill).into()
}

/// `.card.stack` with an `h2`: the frame of most settings pages.
pub fn card<'a, M: 'a>(title: &'a str, lines: Vec<El<'a, M>>) -> El<'a, M> {
    let mut c = column![widgets::h2(title)].spacing(14);
    for l in lines {
        c = c.push(l);
    }
    widgets::card(c.width(Length::Fill)).width(Length::Fill).into()
}

/// `.test-result`: a check or a cross with the message, green or red.
pub fn result_line<'a, M: 'a>(ok: bool, message: impl IntoFragment<'a>) -> El<'a, M> {
    let p = theme::look();
    let color = if ok { p.ok } else { p.err };
    row![icon(if ok { Icon::Check } else { Icon::X }, 15.0, color), t(message).size(13).color(color)]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}

/// A test result like [`result_line`]; a failure carries Copy (`on_copy` gets the whole text).
pub fn result_line_copy<'a, M: Clone + 'a>(ok: bool, message: &str, on_copy: impl Fn(String) -> M) -> El<'a, M> {
    if ok {
        return result_line(ok, message.to_string());
    }
    let color = theme::look().err;
    row![
        icon(Icon::X, 15.0, color),
        container(t(message.to_string()).size(13).color(color)).width(Length::Fill),
        crate::details::copy_btn(on_copy(message.to_string())),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

/// A link-looking button in the accent colour (`<a href>` on the web).
pub fn link<'a, M: Clone + 'a>(label: impl IntoFragment<'a>, on: M) -> El<'a, M> {
    button(t(label).size(13).color(theme::look().accent)).padding([2, 4]).style(theme::btn(Btn::Ghost)).on_press(on).into()
}

/// A small muted link-looking button (the `clear`, `Sign out`, reset links).
pub fn link_muted<'a, M: Clone + 'a>(label: impl IntoFragment<'a>, on: M) -> El<'a, M> {
    button(t(label).size(12.5).color(theme::look().muted)).padding([2, 4]).style(theme::btn(Btn::Ghost)).on_press(on).into()
}

/// The left words and the right value or button of a row.
pub fn between<'a, M: 'a>(left: El<'a, M>, right: El<'a, M>) -> El<'a, M> {
    row![left, space().width(Length::Fill), right].spacing(12).align_y(Alignment::Center).width(Length::Fill).into()
}

/// Two fields side by side (the web's `.form-grid`); `right` None leaves the row half empty.
pub fn two<'a, M: 'a>(left: El<'a, M>, right: Option<El<'a, M>>) -> El<'a, M> {
    let mut r = row![container(left).width(Length::FillPortion(1))].spacing(16).align_y(Alignment::Start);
    r = r.push(match right {
        Some(x) => container(x).width(Length::FillPortion(1)),
        None => container(space()).width(Length::FillPortion(1)),
    });
    r.width(Length::Fill).into()
}
