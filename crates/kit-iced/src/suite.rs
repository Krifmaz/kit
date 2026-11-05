//! Suite on the desktop (`contracts/suite.md`): the shared sidebar of every linked app's sections with Edit
//! sidebar, the search across apps and "related in …" links. The rules (order, hidden entries, groups) are
//! kit-rules' `suite`, re-exported here; the words are [`SuiteWords`].
//!
//! Like every kit-iced component these hold no network code. [`Sidebar`] and [`Search`] keep their state, draw
//! it and answer each message with what the app should do next ([`Out`]): ask the server
//! (`GET`/`PUT /api/kit/suite/sidebar`, `GET /api/kit/suite/search?q=`), open a page of this app, or open another
//! app's address in the browser.

use crate::theme::{self, Btn};
use crate::widgets::{btn_sm, card_flat, error_box, h3, small_muted, t, El};
use iced::widget::{button, column, container, row, svg, text_input};
use iced::{Alignment, Color, Length};
use serde_json::Value;

pub use kit_rules::suite::{move_item, prefs_of, toggle_hidden, SearchGroup, SearchHit, SidebarItem, SidebarPrefs, SuiteWords};

/// An icon of the suite (SVG path data in a 24×24 box) of `size` px in `color`. Path data that can't be drawn
/// draws nothing.
pub fn path_icon<'a, M: 'a>(d: &str, size: f32, color: Color) -> El<'a, M> {
    let esc = d.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
    let doc = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="{esc}"/></svg>"##
    );
    svg::Svg::new(svg::Handle::from_memory(doc.into_bytes()))
        .width(size)
        .height(size)
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

/// What the app does after a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// `PUT /api/kit/suite/sidebar` with this body, then [`Sidebar::load`] its answer.
    Save(Value),
    /// Open this app's page at `path` (the section's path in its manifest).
    Local(String),
    /// Open another app at this address, in the browser.
    Browse(String),
    /// `GET /api/kit/suite/search?q=` with this text (already percent-encoded in the path), then
    /// [`Search::load`] its answer.
    Search(String),
}

/// A message of [`Sidebar`].
#[derive(Debug, Clone, PartialEq)]
pub enum SidebarMsg {
    /// An entry was clicked.
    Open(String),
    /// Edit sidebar.
    Edit,
    /// Move an entry up (-1) or down (1).
    Move(String, i32),
    /// Hide or show an entry.
    Toggle(String),
    /// Put the sidebar back in the apps' order.
    Reset,
    /// Done: keep the order.
    Done,
}

/// The shared sidebar's state: its entries, and the draft while the person edits it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sidebar {
    /// Every entry in the person's order, hidden ones marked.
    pub items: Vec<SidebarItem>,
    /// The entries being edited (Some while editing).
    pub draft: Option<Vec<SidebarItem>>,
    /// The words.
    pub words: SuiteWords,
}

impl Sidebar {
    /// Keep a sidebar answer (`{items, apps}`) and stop editing.
    pub fn load(&mut self, answer: &Value) {
        self.items = answer.get("items").cloned().and_then(|i| serde_json::from_value(i).ok()).unwrap_or_default();
        self.draft = None;
    }

    /// What the app does after `msg`.
    pub fn update(&mut self, msg: SidebarMsg) -> Option<Out> {
        match msg {
            SidebarMsg::Open(key) => {
                let i = self.items.iter().find(|i| i.key == key)?;
                Some(if i.local { Out::Local(i.path.clone()) } else { Out::Browse(i.url.clone()) })
            }
            SidebarMsg::Edit => {
                self.draft = Some(self.items.clone());
                None
            }
            SidebarMsg::Move(key, by) => {
                if let Some(d) = self.draft.as_mut() {
                    move_item(d, &key, by);
                }
                None
            }
            SidebarMsg::Toggle(key) => {
                if let Some(d) = self.draft.as_mut() {
                    toggle_hidden(d, &key);
                }
                None
            }
            SidebarMsg::Reset => Some(Out::Save(serde_json::json!({}))),
            SidebarMsg::Done => {
                let d = self.draft.as_ref()?;
                Some(Out::Save(serde_json::to_value(prefs_of(d)).unwrap_or_default()))
            }
        }
    }

    /// The entries (the visible ones; every one with its buttons while editing), `active` (a local path) marked.
    pub fn view<'a, M: Clone + 'a>(&'a self, active: &str, map: impl Fn(SidebarMsg) -> M + 'a) -> El<'a, M> {
        let p = theme::look();
        let w = &self.words;
        let mut col = column![].spacing(2);
        if let Some(d) = &self.draft {
            let last = d.len().saturating_sub(1);
            for (n, i) in d.iter().enumerate() {
                let color = if i.hidden { p.faint } else { p.text };
                let mut r = row![path_icon(&i.icon, 16.0, color), t(i.title.clone()).size(13).color(color).width(Length::Fill)]
                    .spacing(8)
                    .align_y(Alignment::Center);
                if !i.local {
                    r = r.push(t(i.app_name.clone()).size(11).color(p.faint));
                }
                r = r
                    .push(btn_sm("↑", Btn::Ghost, (n > 0).then(|| map(SidebarMsg::Move(i.key.clone(), -1)))))
                    .push(btn_sm("↓", Btn::Ghost, (n < last).then(|| map(SidebarMsg::Move(i.key.clone(), 1)))))
                    .push(btn_sm(if i.hidden { w.show } else { w.hide }, Btn::Ghost, Some(map(SidebarMsg::Toggle(i.key.clone())))));
                col = col.push(container(r).padding([4, 6]));
            }
            col = col.push(
                row![
                    iced::widget::space().width(Length::Fill),
                    btn_sm(w.reset, Btn::Ghost, Some(map(SidebarMsg::Reset))),
                    btn_sm(w.done, Btn::Primary, Some(map(SidebarMsg::Done)))
                ]
                .spacing(6),
            );
            return col.into();
        }
        for i in self.items.iter().filter(|i| !i.hidden) {
            let on = i.local && i.path == active;
            let mut r = row![path_icon(&i.icon, 18.0, if on { p.text } else { p.muted }), t(i.title.clone()).size(14).width(Length::Fill)]
                .spacing(10)
                .align_y(Alignment::Center);
            if !i.local {
                r = r.push(t(i.app_name.clone()).size(11).color(p.faint));
            }
            col = col.push(
                button(r)
                    .width(Length::Fill)
                    .padding([8, 12])
                    .style(theme::btn(if on { Btn::NavOn } else { Btn::Nav }))
                    .on_press(map(SidebarMsg::Open(i.key.clone()))),
            );
        }
        if !self.items.is_empty() {
            col = col.push(btn_sm(w.edit, Btn::Ghost, Some(map(SidebarMsg::Edit))));
        }
        col.into()
    }
}

/// A message of [`Search`].
#[derive(Debug, Clone, PartialEq)]
pub enum SearchMsg {
    /// The text changed.
    Text(String),
    /// Enter: search now.
    Submit,
    /// A hit was clicked (its full URL, and whether it is this app's).
    Open(String, bool),
}

/// The search across apps: the text, and the last answer's groups.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Search {
    /// The text typed.
    pub text: String,
    /// The last answer's groups (None before the first search).
    pub groups: Option<Vec<SearchGroup>>,
    /// A search is under way.
    pub busy: bool,
    /// The words.
    pub words: SuiteWords,
}

/// The path of a search across apps for `q` (under `/api`), `scope` `all`, `self` or `others`.
pub fn search_path(q: &str, scope: &str) -> String {
    format!("/kit/suite/search?scope={scope}&q={}", kit_rules::suite::encode(q.trim()))
}

/// The groups of a search answer.
pub fn groups_of(answer: &Value) -> Vec<SearchGroup> {
    answer.get("groups").cloned().and_then(|g| serde_json::from_value(g).ok()).unwrap_or_default()
}

impl Search {
    /// Keep a search answer.
    pub fn load(&mut self, answer: &Value) {
        self.groups = Some(groups_of(answer));
        self.busy = false;
    }

    /// What the app does after `msg`.
    pub fn update(&mut self, msg: SearchMsg) -> Option<Out> {
        match msg {
            SearchMsg::Text(s) => {
                self.text = s;
                None
            }
            SearchMsg::Submit if self.text.trim().is_empty() => {
                self.groups = None;
                None
            }
            SearchMsg::Submit => {
                self.busy = true;
                Some(Out::Search(search_path(&self.text, "all")))
            }
            SearchMsg::Open(url, _) => Some(Out::Browse(url)),
        }
    }

    /// The field and the groups.
    pub fn view<'a, M: Clone + 'a>(&'a self, map: impl Fn(SearchMsg) -> M + Clone + 'a) -> El<'a, M> {
        let m = map.clone();
        let field = text_input(self.words.search_placeholder, &self.text)
            .on_input(move |s| m(SearchMsg::Text(s)))
            .on_submit(map(SearchMsg::Submit))
            .padding([9, 12])
            .style(theme::input);
        let mut col = column![field].spacing(10);
        match &self.groups {
            None if self.busy => col = col.push(small_muted("…")),
            None => {}
            Some(g) if g.is_empty() => {
                col = col.push(small_muted(kit_rules::app::fill(self.words.no_results, &[("query", self.text.trim())])))
            }
            Some(g) => col = col.push(groups(g, "{app}", map)),
        }
        col.into()
    }
}

/// Search groups: each app's icon and name (`title` filled with `{app}`), its hits (a click sends
/// `SearchMsg::Open(url, local)`), or the reason it failed.
pub fn groups<'a, M: Clone + 'a>(gs: &'a [SearchGroup], title: &'a str, map: impl Fn(SearchMsg) -> M + 'a) -> El<'a, M> {
    draw_groups(gs, title, false, map)
}

/// The groups, only those that found something when `found_only`.
fn draw_groups<'a, M: Clone + 'a>(gs: &'a [SearchGroup], title: &'a str, found_only: bool, map: impl Fn(SearchMsg) -> M + 'a) -> El<'a, M> {
    let p = theme::look();
    let mut col = column![].spacing(12);
    for g in gs.iter().filter(|g| !found_only || !g.results.is_empty()) {
        let head = row![path_icon(&g.icon, 16.0, p.muted), h3(kit_rules::app::fill(title, &[("app", &g.app_name)]))]
            .spacing(8)
            .align_y(Alignment::Center);
        let mut list = column![].spacing(0);
        for h in &g.results {
            let mut c = column![t(h.title.clone()).size(14)];
            if let Some(s) = &h.subtitle {
                c = c.push(t(s.clone()).size(12).color(p.muted));
            }
            list = list.push(
                button(c)
                    .width(Length::Fill)
                    .padding([8, 14])
                    .style(theme::btn(Btn::Ghost))
                    .on_press(map(SearchMsg::Open(h.url.clone(), g.local))),
            );
        }
        let mut block = column![head].spacing(6);
        if let Some(e) = &g.error {
            block = block.push(error_box(e));
        }
        if !g.results.is_empty() {
            block = block.push(card_flat(list).width(Length::Fill));
        }
        col = col.push(block);
    }
    col.into()
}

/// "Related in <app>" for the groups of a `scope=others` search (only apps that found something).
pub fn related<'a, M: Clone + 'a>(gs: &'a [SearchGroup], words: &'a SuiteWords, map: impl Fn(SearchMsg) -> M + 'a) -> El<'a, M> {
    draw_groups(gs, words.related_in, true, map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answer() -> Value {
        json!({ "items": [
            { "key": "me/a", "app": "me", "app_name": "Me", "section": "a", "title": "A", "icon": "M0 0", "url": "https://me.test/a", "path": "/a", "local": true },
            { "key": "other/b", "app": "other", "app_name": "Other", "section": "b", "title": "B", "icon": "M\"<x", "url": "https://other.test/b", "path": "/b", "local": false },
            { "key": "other/c", "app": "other", "app_name": "Other", "section": "c", "title": "C", "icon": "", "url": "https://other.test/c", "path": "/c", "hidden": true }
        ] })
    }

    #[test]
    fn the_sidebar_opens_and_edits() {
        let mut s = Sidebar::default();
        s.load(&answer());
        assert_eq!(s.items.len(), 3);
        assert_eq!(s.update(SidebarMsg::Open("me/a".into())), Some(Out::Local("/a".into())));
        assert_eq!(s.update(SidebarMsg::Open("other/b".into())), Some(Out::Browse("https://other.test/b".into())));
        assert_eq!(s.update(SidebarMsg::Open("nope".into())), None);
        assert_eq!(s.update(SidebarMsg::Done), None, "nothing to keep before editing");
        let _ = s.view("/a", |m| m);
        s.update(SidebarMsg::Edit);
        s.update(SidebarMsg::Move("other/c".into(), -1));
        s.update(SidebarMsg::Toggle("other/c".into()));
        let _ = s.view("/a", |m| m);
        let Some(Out::Save(body)) = s.update(SidebarMsg::Done) else { panic!("Done saves") };
        assert_eq!(body, json!({ "order": ["me/a", "other/c", "other/b"], "hidden": [] }));
        assert_eq!(s.update(SidebarMsg::Reset), Some(Out::Save(json!({}))));
        s.load(&answer());
        assert!(s.draft.is_none());
    }

    #[test]
    fn the_search_asks_once_per_submit() {
        let mut s = Search::default();
        assert_eq!(s.update(SearchMsg::Submit), None);
        s.update(SearchMsg::Text("a b".into()));
        assert_eq!(s.update(SearchMsg::Submit), Some(Out::Search("/kit/suite/search?scope=all&q=a%20b".into())));
        assert!(s.busy);
        let _ = s.view(|m| m);
        s.load(&json!({ "groups": [
            { "app": "x", "app_name": "X", "results": [{ "title": "T", "subtitle": "S", "url": "https://x.test/1" }] },
            { "app": "y", "app_name": "Y", "results": [], "error": "Y can't be reached." }
        ] }));
        assert_eq!(s.groups.as_ref().unwrap().len(), 2);
        assert!(!s.busy);
        let _ = s.view(|m| m);
        let words = SuiteWords::DEFAULT;
        let g = s.groups.clone().unwrap();
        let _ = related(&g, &words, |m| m);
        assert_eq!(s.update(SearchMsg::Open("https://x.test/1".into(), false)), Some(Out::Browse("https://x.test/1".into())));
        assert_eq!(search_path(" q ", "others"), "/kit/suite/search?scope=others&q=q");
        s.load(&json!({ "groups": [] }));
        let _ = s.view(|m| m);
    }
}
