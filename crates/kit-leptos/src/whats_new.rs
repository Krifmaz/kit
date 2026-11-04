//! What's new: the popup on the first start after the server was updated, and the changelog page, the full
//! changelog with the switch that turns the popup off.
//!
//! The server's paths ([`Paths::changelog`](crate::config::Paths::changelog), `changelog_new`,
//! `changelog_seen`, `popup`), the changelog page's route ([`Routes::changelog`](crate::config::Routes)) and the
//! words ([`ChangelogWords`]) come from [`crate::config`].

use crate::api;
use crate::html::{html, opt, seq};
use crate::icons::chevron;
use crate::modal::{self, Modal, ModalOpts};
use crate::toasts::toast;
use kit_rules::changelog::{group, Block, ChangelogWords, Group, Release};
use leptos::prelude::*;
use serde::Deserialize;
use std::rc::Rc;

/// The answer of `GET /api/changelog/new` (the configured `changelog_new`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct NewAnswer {
    /// Whether the popup shows.
    pub show: bool,
    /// The releases this account has not seen.
    pub releases: Vec<Release>,
}

/// The answer of `GET /api/changelog` (the configured `changelog`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ChangelogAnswer {
    /// The version the server runs.
    pub version: String,
    /// Whether the popup is on for this account.
    pub popup: bool,
    /// Every release, newest first, with its notes.
    pub releases: Vec<Release>,
}

fn words() -> ChangelogWords {
    crate::config::get().words.changelog
}

/// After signing in: shows the popup when something is new since this account last saw it, then marks it
/// seen. `navigate` opens a path of the app (Full changelog goes to the configured changelog route).
pub fn start(navigate: Rc<dyn Fn(&str)>) {
    let paths = crate::config::get().paths.clone();
    crate::rt::spawn(async move {
        let Ok(r) = api::get(&paths.changelog_new).await else { return };
        let r: NewAnswer = serde_json::from_value(r).unwrap_or_default();
        if !r.show {
            return;
        }
        popup(r.releases, navigate);
        let _ = api::post_empty(&paths.changelog_seen).await;
    });
}

/// One release in the popup: its title, summary and highlights.
pub fn release_view(r: &Release) -> impl IntoView {
    let summary = (!r.summary.is_empty()).then(|| view! { <p {..()} class="muted" style="margin:0">{r.summary.clone()}</p> });
    let highlights = (!r.highlights.is_empty()).then(|| {
        let items = seq(r.highlights.iter().map(|h| view! { <li>{h.clone()}</li> }));
        view! { <ul style="margin:0;padding-left:20px">{items}</ul> }
    });
    view! { <div {..()} class="stack" style="gap:6px"><h3 style="margin:0">{r.title.clone()}</h3>{opt(summary)}{opt(highlights)}</div> }
}

/// The popup's body.
pub fn popup_body(releases: &[Release]) -> impl IntoView {
    let list = seq(releases.iter().map(release_view));
    view! { <div {..()} class="stack" style="gap:18px">{list}</div> }
}

/// The popup's footer: the switch, Full changelog and Close. `on_full` runs for Full changelog.
pub fn popup_footer(on_full: impl Fn() + 'static) -> impl IntoView {
    let w = words();
    view! {
        <label {..()} class="switch" style="margin-right:auto">
            <input type="checkbox" id="wn-on" checked on:change=move |e| save_popup(event_target_checked(&e))/><i>{""}</i><span>{w.show_this}</span>
        </label>
        <button class="btn" type="button" id="wn-full" on:click=move |_| on_full()>{w.full_changelog}</button>
        <button class="btn primary" type="button" data-close>{w.close}</button>
    }
}

/// Opens the popup for `releases`.
pub fn popup(releases: Vec<Release>, navigate: Rc<dyn Fn(&str)>) -> Modal {
    let handle: Rc<std::cell::RefCell<Option<Modal>>> = Rc::default();
    let h = handle.clone();
    let route = crate::config::get().routes.changelog.clone();
    let m = modal::open(ModalOpts::with_view(words().whats_new, move || popup_body(&releases)).footer_view(move || {
        popup_footer(move || {
            if let Some(m) = h.borrow().as_ref() {
                m.close();
            }
            navigate(&route);
        })
    }));
    *handle.borrow_mut() = Some(m.clone());
    m
}

/// Saves the switch (`PUT` to the configured `popup` path, `{"<popup_field>": on}`) and says so.
pub fn save_popup(on: bool) {
    let paths = crate::config::get().paths.clone();
    crate::rt::spawn(async move {
        let mut body = serde_json::Map::new();
        body.insert(paths.popup_field.clone(), serde_json::Value::Bool(on));
        match api::put(&paths.popup, serde_json::Value::Object(body)).await {
            Ok(_) => toast(words().saved(on), "ok"),
            Err(e) => toast(&e.message, "err"),
        }
    });
}

const CELL: &str = "text-align:left;padding:4px 10px 4px 0;vertical-align:top";

/// A release's notes: bullets and table rows grouped into their lists and tables.
pub fn notes_view(blocks: &[Block]) -> AnyView {
    seq(group(blocks).into_iter().map(|g| match g {
        Group::Heading(t) => view! { <h3 style="margin:10px 0 0">{t}</h3> }.into_any(),
        Group::Paragraph(t) => view! { <p style="margin:0">{t}</p> }.into_any(),
        Group::List(items) => {
            let lis = seq(items.into_iter().map(|i| {
                let lead = (!i.lead.is_empty()).then(|| view! { <strong>{i.lead}</strong>" " });
                view! { <li>{opt(lead)}{i.text}</li> }
            }));
            view! { <ul {..()} style="margin:0;padding-left:20px" class="stack">{lis}</ul> }.into_any()
        }
        Group::Table(rows) => {
            let trs = seq(rows.into_iter().map(|r| {
                let head = r.head;
                let cells = seq(r.cells.into_iter().map(move |c| {
                    if head {
                        view! { <th style=CELL>{c}</th> }.into_any()
                    } else {
                        view! { <td style=CELL>{c}</td> }.into_any()
                    }
                }));
                view! { <tr>{cells}</tr> }
            }));
            // the `<tbody>` the browser's parser adds around rows written straight into a `<table>`
            view! { <table {..()} class="small" style="border-collapse:collapse"><tbody>{trs}</tbody></table> }.into_any()
        }
    }))
}

/// The changelog page drawn from an answer of `GET /api/changelog`: the card with the version and the popup
/// switch, then every release (the newest unfolded).
#[component]
pub fn ChangelogView(
    /// The app's name, in "This is Kit Demo 1.4.0"; the configured name when left out.
    #[prop(optional, into)]
    app: Option<String>,
    /// The answer of `GET /api/changelog`.
    answer: ChangelogAnswer,
) -> impl IntoView {
    let w = words();
    let app = app.unwrap_or_else(crate::config::app_name);
    let list = if answer.releases.is_empty() {
        view! { <div class="card muted">{w.no_notes}</div> }.into_any()
    } else {
        seq(answer.releases.iter().enumerate().map(|(i, x)| {
            view! {
                <details class="card settings-card" open=i == 0>
                    <summary {..()} class="row" style="justify-content:space-between"><h2 style="margin:0">{x.title.clone()}</h2><span {..()} class="row" style="gap:8px"><span class="badge">{x.version.clone()}</span><span class="chev" {..html(chevron())}>{""}</span></span></summary>
                    <div {..()} class="stack" style="gap:10px;margin-top:12px">{notes_view(&x.blocks)}</div>
                </details>
            }
        }))
    };
    view! {
        <div class="stack">
            <div class="card stack">
                <h2>{w.whats_new}</h2>
                <p {..()} class="small muted" style="margin:0">{w.intro(&app, &answer.version)}</p>
                <label class="switch"><input type="checkbox" id="wn-on" checked=answer.popup on:change=move |e| save_popup(event_target_checked(&e))/><i>{""}</i><span>{w.show_after_updates}</span></label>
            </div>
            {list}
        </div>
    }
}

/// The changelog page: loads `GET /api/changelog` and draws [`ChangelogView`] (the error's sentence when the
/// call fails).
#[component]
pub fn ChangelogPage(
    /// The app's name; the configured name when left out.
    #[prop(optional, into)]
    app: Option<String>,
) -> impl IntoView {
    let app = app.unwrap_or_else(crate::config::app_name);
    let answer: RwSignal<Option<Result<ChangelogAnswer, String>>, LocalStorage> = RwSignal::new_local(None);
    let path = crate::config::get().paths.changelog.clone();
    crate::rt::spawn(async move {
        let r = api::get(&path).await.map(|v| serde_json::from_value(v).unwrap_or_default()).map_err(|e| e.message);
        answer.set(Some(r));
    });
    move || match answer.get() {
        None => view! { <div {..()} class="skeleton" style="height:200px"></div> }.into_any(),
        Some(Ok(a)) => view! { <ChangelogView app=app.clone() answer=a/> }.into_any(),
        Some(Err(e)) => view! { <div class="error-box">{e}</div> }.into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};
    use serde_json::{json, Value};

    const CELL: &str = r#"style="text-align:left;padding:4px 10px 4px 0;vertical-align:top""#;

    fn answer() -> Value {
        json!({ "version": "1.4.0", "popup": true, "releases": [
            { "version": "1.4.0", "title": "Kit Demo 1.4.0", "summary": "Share <folders> with others.", "highlights": ["Share a folder by link.", "See who opened what."],
              "blocks": [
                { "k": "p", "text": "Sharing & more." },
                { "k": "h", "text": "Sharing" },
                { "k": "li", "lead": "Share a folder by link.", "text": "Folders → Share makes a link." },
                { "k": "li", "text": "Links can expire." },
                { "k": "row", "cells": ["Setting", "Default"], "head": true },
                { "k": "row", "cells": ["a", "b"] },
                { "k": "quote", "text": "unknown" },
                { "k": "li", "text": "after an unknown block" }
              ] },
            { "version": "1.3.2", "title": "Kit Demo 1.3.2", "summary": "", "highlights": [], "blocks": [] }
        ] })
    }

    fn chev() -> String {
        format!(r#"<span class="chev">{}</span>"#, chevron())
    }

    #[test]
    fn the_changelog_page_lists_every_release() {
        let a: ChangelogAnswer = serde_json::from_value(answer()).unwrap();
        let want = format!(
            r#"<div class="stack">
              <div class="card stack">
                <h2>What's new</h2>
                <p class="small muted" style="margin:0">This is Kit Demo 1.4.0. After an update, a short list of what changed shows once.</p>
                <label class="switch"><input type="checkbox" id="wn-on" checked><i></i><span>Show what's new after updates</span></label>
              </div>
              <details class="card settings-card" open>
                <summary class="row" style="justify-content:space-between"><h2 style="margin:0">Kit Demo 1.4.0</h2><span class="row" style="gap:8px"><span class="badge">1.4.0</span>{chev}</span></summary>
                <div class="stack" style="gap:10px;margin-top:12px">
                  <p style="margin:0">Sharing &amp; more.</p>
                  <h3 style="margin:10px 0 0">Sharing</h3>
                  <ul style="margin:0;padding-left:20px" class="stack"><li><strong>Share a folder by link.</strong> Folders → Share makes a link.</li><li>Links can expire.</li></ul>
                  <table class="small" style="border-collapse:collapse"><tbody><tr><th {CELL}>Setting</th><th {CELL}>Default</th></tr><tr><td {CELL}>a</td><td {CELL}>b</td></tr></tbody></table>
                  <ul style="margin:0;padding-left:20px" class="stack"><li>after an unknown block</li></ul>
                </div>
              </details>
              <details class="card settings-card">
                <summary class="row" style="justify-content:space-between"><h2 style="margin:0">Kit Demo 1.3.2</h2><span class="row" style="gap:8px"><span class="badge">1.3.2</span>{chev}</span></summary>
                <div class="stack" style="gap:10px;margin-top:12px"></div>
              </details>
            </div>"#,
            chev = chev()
        );
        assert_same_html(&render(|| view! { <ChangelogView app="Kit Demo" answer=a.clone()/> }), &want);
        // no notes in this build, popup off, the configured name
        let empty: ChangelogAnswer = serde_json::from_value(json!({ "version": "1.0.0", "popup": false, "releases": [] })).unwrap();
        let want = r#"<div class="stack"><div class="card stack"><h2>What's new</h2><p class="small muted" style="margin:0">This is App 1.0.0. After an update, a short list of what changed shows once.</p><label class="switch"><input type="checkbox" id="wn-on"><i></i><span>Show what's new after updates</span></label></div><div class="card muted">No release notes in this build.</div></div>"#;
        assert_same_html(&render(|| view! { <ChangelogView answer=empty.clone()/> }), want);
    }

    #[test]
    fn the_popup_shows_each_new_release() {
        let a: ChangelogAnswer = serde_json::from_value(answer()).unwrap();
        let want = r#"<div class="stack" style="gap:18px">
            <div class="stack" style="gap:6px"><h3 style="margin:0">Kit Demo 1.4.0</h3><p class="muted" style="margin:0">Share &lt;folders&gt; with others.</p><ul style="margin:0;padding-left:20px"><li>Share a folder by link.</li><li>See who opened what.</li></ul></div>
            <div class="stack" style="gap:6px"><h3 style="margin:0">Kit Demo 1.3.2</h3></div>
        </div>"#;
        assert_same_html(&render(|| popup_body(&a.releases)), want);
        let foot = r#"<label class="switch" style="margin-right:auto"><input type="checkbox" id="wn-on" checked><i></i><span>Show this after updates</span></label><button class="btn" type="button" id="wn-full">Full changelog</button><button class="btn primary" type="button" data-close>Close</button>"#;
        assert_same_html(&render(|| popup_footer(|| {})), foot);
    }

    #[test]
    fn an_app_words_it_its_own_way() {
        let mut cfg = crate::config::AppConfig::new(kit_rules::app::App::named("Notes"));
        cfg.words.changelog.whats_new = "Neu";
        cfg.words.changelog.intro = "{app} {version}";
        crate::config::set(cfg);
        let a: ChangelogAnswer = serde_json::from_value(json!({ "version": "2.0.0", "releases": [] })).unwrap();
        let html = render(|| view! { <ChangelogView answer=a.clone()/> });
        assert!(html.contains("<h2>Neu</h2>") && html.contains(">Notes 2.0.0</p>"), "{html}");
        crate::config::set(crate::config::AppConfig::default());
    }
}
