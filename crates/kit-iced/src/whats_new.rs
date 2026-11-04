//! What's new: the popup on the first start after the server was updated, and the changelog page
//! (Settings → What's new); the API is contracts/changelog.md.
//!
//! The flow, with the requests made by the app:
//! 1. After the session is known the app asks `GET /api/changelog/new` and hands the answer to
//!    [`Popup::loaded`]. When it says `show`, the popup opens and the app sends
//!    `POST /api/changelog/seen` (so the website does not show it again either).
//! 2. The popup lists each new release's highlights. Full changelog opens the changelog page; the
//!    switch turns the popup off (the app saves `PUT /api/auth/prefs {"whats_new_popup": on}` and
//!    toasts [`saved_words`], or calls [`Popup::save_failed`] and toasts the error).
//! 3. The changelog page ([`Changelog`]) shows every release this server includes
//!    (`GET /api/changelog`, newest first and open) with the same switch.
//!
//! Hotfixes never re-trigger the popup: the server decides that (`show` is false for them).

use crate::json::{s, truthy};
use crate::settings::{card, err_text, help, switch};
use crate::theme::{self, Badge, Btn};
use crate::widgets::{self, t, El};
use iced::widget::{button, column, row, space, text, toggler};
use iced::{Alignment, Length};
use serde_json::Value;
use std::collections::HashSet;

/// The popup's title.
pub const TITLE: &str = kit_rules::changelog::WHATS_NEW;
/// The popup's switch.
pub const SHOW_AFTER_UPDATES: &str = kit_rules::changelog::SHOW_THIS_AFTER_UPDATES;
/// The changelog page's switch.
pub const SHOW_WHATS_NEW: &str = kit_rules::changelog::SHOW_WHATS_NEW_AFTER_UPDATES;
/// The popup's button to the changelog page.
pub const FULL_CHANGELOG: &str = kit_rules::changelog::FULL_CHANGELOG;
/// The changelog page without releases.
pub const NO_NOTES: &str = kit_rules::changelog::NO_RELEASE_NOTES;

pub use kit_rules::changelog::saved_words;

/// The help line under the changelog page's title: `app` is the app's name ("Demo").
pub fn runs_words(app: &str, version: &str) -> String {
    kit_rules::changelog::settings_intro(app, version)
}

// ----- the popup ---------------------------------------------------------------------------

/// The popup's state: the new releases while it is open, and the switch.
#[derive(Debug, Default)]
pub struct Popup {
    releases: Option<Vec<Value>>,
    on: bool,
}

/// What the popup's buttons send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupMsg {
    /// Close (the X, the backdrop, Escape, Close).
    Close,
    /// Full changelog: the app closes the popup ([`Popup::close`]) and opens the changelog page.
    Full,
    /// The switch: the app saves it.
    Popup(bool),
}

impl Popup {
    /// The answer of `GET /api/changelog/new`. True when the popup opened: the app then marks this
    /// version seen (`POST /api/changelog/seen`). An older server without a changelog (an error) shows
    /// nothing, so the app simply does not call this.
    pub fn loaded(&mut self, v: &Value) -> bool {
        if truthy(v, "show") {
            self.releases = Some(v["releases"].as_array().cloned().unwrap_or_default());
            self.on = true;
            return true;
        }
        false
    }

    /// Whether the popup is open.
    pub fn is_open(&self) -> bool {
        self.releases.is_some()
    }

    /// Close it.
    pub fn close(&mut self) {
        self.releases = None;
    }

    /// The switch was moved (before it is saved).
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Saving the switch failed: it goes back.
    pub fn save_failed(&mut self, on: bool) {
        self.on = !on;
    }

    /// Where the switch stands.
    pub fn on(&self) -> bool {
        self.on
    }
}

/// The popup, while it is open: each new release's title, summary and highlights; the switch, Full
/// changelog and Close.
pub fn popup(state: &Popup) -> Option<El<'_, PopupMsg>> {
    let releases = state.releases.as_ref()?;
    let p = theme::look();
    let mut body = column![].spacing(18).width(Length::Fill);
    for r in releases {
        let mut c = column![widgets::h3(s(r, "title"))].spacing(6);
        let summary = s(r, "summary");
        if !summary.is_empty() {
            c = c.push(widgets::muted(summary).size(14));
        }
        for h in r["highlights"].as_array().into_iter().flatten() {
            c = c.push(
                row![t("•").size(14).color(p.muted), t(h.as_str().unwrap_or("").to_string()).size(14).width(Length::Fill)].spacing(8),
            );
        }
        body = body.push(c);
    }
    let footer = row![
        toggler(state.on).label(SHOW_AFTER_UPDATES).text_size(13).spacing(8).on_toggle(PopupMsg::Popup).style(theme::switch),
        space().width(Length::Fill),
        widgets::btn(FULL_CHANGELOG, Btn::Default, Some(PopupMsg::Full)),
        widgets::btn(kit_rules::changelog::CLOSE, Btn::Primary, Some(PopupMsg::Close)),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    Some(widgets::modal(TITLE, body, Some(footer.into()), false, PopupMsg::Close))
}

// ----- the changelog page ------------------------------------------------------------------

/// The changelog page's state.
#[derive(Debug, Default)]
pub struct Changelog {
    data: Option<Value>,
    popup: bool,
    /// The releases shown open (by position); the newest is open at first.
    open: HashSet<usize>,
    error: String,
}

/// What the changelog page sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangelogMsg {
    /// The switch: the app saves it.
    Popup(bool),
    /// Open or close a release's notes.
    Toggle(usize),
}

impl Changelog {
    /// The page opens: loading, with the newest release open. The app then asks `GET /api/changelog`.
    pub fn reset(&mut self) {
        self.data = None;
        self.error.clear();
        self.open = HashSet::from([0]);
    }

    /// The answer of `GET /api/changelog`, or the error's words.
    pub fn loaded(&mut self, answer: Result<Value, String>) {
        match answer {
            Ok(v) => {
                self.popup = truthy(&v, "popup");
                self.data = Some(v);
            }
            Err(e) => self.error = e,
        }
    }

    /// Apply a message. `Some(on)` asks the app to save the switch.
    pub fn update(&mut self, msg: ChangelogMsg) -> Option<bool> {
        match msg {
            ChangelogMsg::Popup(on) => {
                self.popup = on;
                Some(on)
            }
            ChangelogMsg::Toggle(i) => {
                if !self.open.remove(&i) {
                    self.open.insert(i);
                }
                None
            }
        }
    }

    /// Saving the switch failed: it goes back.
    pub fn save_failed(&mut self) {
        self.popup = !self.popup;
    }

    /// Where the switch stands (for the toast after it was saved: [`saved_words`]).
    pub fn popup(&self) -> bool {
        self.popup
    }
}

/// A release's notes as the server sends them (`k`: h, p, li, row; contracts/changelog.md).
pub fn notes<'a, M: 'a>(blocks: &[Value]) -> El<'a, M> {
    let p = theme::look();
    let mut c = column![].spacing(8).width(Length::Fill);
    for b in blocks {
        let line: El<'a, M> = match s(b, "k").as_str() {
            "h" => iced::widget::container(widgets::h3(s(b, "text"))).padding(widgets::pad(8.0, 0.0, 0.0, 0.0)).into(),
            "p" => t(s(b, "text")).size(14).into(),
            "li" => {
                let lead = s(b, "lead");
                let mut spans: Vec<iced::widget::text::Span<'a, ()>> = Vec::new();
                if !lead.is_empty() {
                    spans.push(iced::widget::span(format!("{lead} ")).font(theme::bold()));
                }
                spans.push(iced::widget::span(s(b, "text")));
                row![t("•").size(14).color(p.muted), iced::widget::rich_text(spans).size(14).width(Length::Fill)].spacing(8).into()
            }
            "row" => {
                let head = truthy(b, "head");
                let mut r = row![].spacing(12);
                for c in b["cells"].as_array().into_iter().flatten() {
                    let cell = t(c.as_str().unwrap_or("").to_string()).size(13).width(Length::FillPortion(1));
                    r = r.push(if head { cell.font(theme::semibold()) } else { cell.color(p.muted) });
                }
                r.into()
            }
            _ => continue,
        };
        c = c.push(line);
    }
    c.into()
}

/// The changelog page: the card with the switch, then every release (title, version, its notes
/// when open). `app` is the app's name for the help line.
pub fn changelog<'a>(state: &'a Changelog, app: &str) -> El<'a, ChangelogMsg> {
    if !state.error.is_empty() {
        return card(TITLE, vec![err_text(state.error.clone()).into()]);
    }
    let Some(data) = &state.data else { return widgets::skeleton_rows(4) };
    let mut page = column![card(
        TITLE,
        vec![help(runs_words(app, &s(data, "version"))).into(), switch(state.popup, SHOW_WHATS_NEW, ChangelogMsg::Popup),]
    )]
    .spacing(14);
    let releases = data["releases"].as_array().cloned().unwrap_or_default();
    if releases.is_empty() {
        page = page.push(widgets::card(widgets::muted(NO_NOTES)).width(Length::Fill));
    }
    for (i, r) in releases.into_iter().enumerate() {
        let open = state.open.contains(&i);
        let head = button(
            row![
                widgets::h2(s(&r, "title")),
                space().width(Length::Fill),
                widgets::badge(s(&r, "version"), Badge::Plain),
                text(if open { "▴" } else { "▾" }).size(16),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding(0)
        .width(Length::Fill)
        .style(theme::btn(Btn::Row))
        .on_press(ChangelogMsg::Toggle(i));
        let mut body = column![head].spacing(12);
        if open {
            let blocks = r["blocks"].as_array().cloned().unwrap_or_default();
            body = body.push(notes(&blocks));
        }
        page = page.push(widgets::card(body.width(Length::Fill)).width(Length::Fill));
    }
    page.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_popup_opens_only_when_the_server_says_show() {
        let mut p = Popup::default();
        assert!(!p.loaded(&json!({ "show": false, "releases": [{ "title": "0.40" }] })));
        assert!(!p.is_open());
        assert!(p.loaded(&json!({ "show": true, "releases": [{ "title": "0.40", "highlights": ["Faster"] }] })));
        assert!(p.is_open() && p.on());
        assert!(popup(&p).is_some());
        p.set_on(false);
        p.save_failed(false);
        assert!(p.on(), "a failed save puts the switch back");
        p.close();
        assert!(popup(&p).is_none());
    }

    #[test]
    fn the_changelog_keeps_the_newest_open_and_toggles() {
        let mut c = Changelog::default();
        c.reset();
        c.loaded(Ok(json!({ "version": "0.40.1", "popup": true, "releases": [{ "title": "a" }, { "title": "b" }] })));
        assert!(c.popup());
        assert!(c.open.contains(&0) && !c.open.contains(&1));
        assert_eq!(c.update(ChangelogMsg::Toggle(1)), None);
        assert!(c.open.contains(&1));
        assert_eq!(c.update(ChangelogMsg::Toggle(0)), None);
        assert!(!c.open.contains(&0));
        assert_eq!(c.update(ChangelogMsg::Popup(false)), Some(false));
        c.save_failed();
        assert!(c.popup());
        let _ = changelog(&c, "Demo");
    }

    #[test]
    fn the_words_name_the_app() {
        assert_eq!(saved_words(true), "What's new will show after updates");
        assert_eq!(saved_words(false), "What's new won't show after updates – it stays in Settings");
        assert_eq!(runs_words("Demo", "0.40.1"), "This is Demo 0.40.1. After an update, a short list of what changed shows once.");
    }
}
