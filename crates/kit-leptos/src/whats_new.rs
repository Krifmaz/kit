//! What's new: the popup on the first start after the server was updated, and Settings → What's new, the full
//! changelog with the switch that turns the popup off.
//!
//! From Encore's `web-rs/src/whats_new.rs` (`start`, `popup`, `release_html`, `save_popup`, `notes_html`,
//! `render`). The calls are `contracts/changelog.md`'s: `GET /api/changelog/new` (`{show, releases}`), then
//! `POST /api/changelog/seen` once the popup showed, `GET /api/changelog` for the page and
//! `PUT /api/auth/prefs {whats_new_popup}` for the switch. Notes are grouped by kit-rules'
//! [`changelog::group`](kit_rules::changelog::group); the words are kit-rules' constants.

use crate::api;
use crate::html::html;
use crate::icons::chevron;
use crate::modal::{self, Modal, ModalOpts};
use crate::toasts::toast;
use kit_rules::changelog::{
    group, saved_words, settings_intro, Block, Group, Release, CLOSE, FULL_CHANGELOG, NO_RELEASE_NOTES, SETTINGS_PATH,
    SHOW_THIS_AFTER_UPDATES, SHOW_WHATS_NEW_AFTER_UPDATES, WHATS_NEW,
};
use leptos::prelude::*;
use serde::Deserialize;
use std::rc::Rc;

/// The answer of `GET /api/changelog/new`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct NewAnswer {
    /// Whether the popup shows.
    pub show: bool,
    /// The releases this account has not seen.
    pub releases: Vec<Release>,
}

/// The answer of `GET /api/changelog`.
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

/// After signing in: shows the popup when something is new since this account last saw it, then marks it
/// seen. `navigate` opens a path of the app (Full changelog goes to [`SETTINGS_PATH`]). From `whats_new.rs`
/// `start`.
pub fn start(navigate: Rc<dyn Fn(&str)>) {
    crate::rt::spawn(async move {
        let Ok(r) = api::get("/changelog/new").await else { return };
        let r: NewAnswer = serde_json::from_value(r).unwrap_or_default();
        if !r.show {
            return;
        }
        popup(r.releases, navigate);
        let _ = api::post_empty("/changelog/seen").await;
    });
}

/// One release in the popup: its title, summary and highlights. From `whats_new.rs` `release_html`.
pub fn release_view(r: &Release) -> impl IntoView {
    let summary = (!r.summary.is_empty()).then(|| view! { <p class="muted" style="margin:0">{r.summary.clone()}</p> });
    let highlights = (!r.highlights.is_empty()).then(|| {
        let items: Vec<_> = r.highlights.iter().map(|h| view! { <li>{h.clone()}</li> }).collect();
        view! { <ul style="margin:0;padding-left:20px">{items}</ul> }
    });
    view! { <div class="stack" style="gap:6px"><h3 style="margin:0">{r.title.clone()}</h3>{summary}{highlights}</div> }
}

/// The popup's body. From `whats_new.rs` `popup`.
pub fn popup_body(releases: &[Release]) -> impl IntoView {
    let list: Vec<_> = releases.iter().map(release_view).collect();
    view! { <div class="stack" style="gap:18px">{list}</div> }
}

/// The popup's footer: the switch, Full changelog and Close. `on_full` runs for Full changelog. From
/// `whats_new.rs` `popup`.
pub fn popup_footer(on_full: impl Fn() + 'static) -> impl IntoView {
    view! {
        <label class="switch" style="margin-right:auto">
            <input type="checkbox" id="wn-on" checked on:change=move |e| save_popup(event_target_checked(&e))/><i></i><span>{SHOW_THIS_AFTER_UPDATES}</span>
        </label>
        <button class="btn" type="button" id="wn-full" on:click=move |_| on_full()>{FULL_CHANGELOG}</button>
        <button class="btn primary" type="button" data-close>{CLOSE}</button>
    }
}

/// Opens the popup for `releases`. From `whats_new.rs` `popup`.
pub fn popup(releases: Vec<Release>, navigate: Rc<dyn Fn(&str)>) -> Modal {
    let handle: Rc<std::cell::RefCell<Option<Modal>>> = Rc::default();
    let h = handle.clone();
    let m = modal::open(ModalOpts::with_view(WHATS_NEW, move || popup_body(&releases)).footer_view(move || {
        popup_footer(move || {
            if let Some(m) = h.borrow().as_ref() {
                m.close();
            }
            navigate(SETTINGS_PATH);
        })
    }));
    *handle.borrow_mut() = Some(m.clone());
    m
}

/// Saves the switch (`PUT /api/auth/prefs`) and says so. From `whats_new.rs` `save_popup`.
pub fn save_popup(on: bool) {
    crate::rt::spawn(async move {
        match api::put("/auth/prefs", serde_json::json!({ "whats_new_popup": on })).await {
            Ok(_) => toast(saved_words(on), "ok"),
            Err(e) => toast(&e.message, "err"),
        }
    });
}

const CELL: &str = "text-align:left;padding:4px 10px 4px 0;vertical-align:top";

/// A release's notes: bullets and table rows grouped into their lists and tables. From `whats_new.rs`
/// `notes_html` and `block_html`.
pub fn notes_view(blocks: &[Block]) -> impl IntoView {
    group(blocks)
        .into_iter()
        .map(|g| match g {
            Group::Heading(t) => view! { <h3 style="margin:10px 0 0">{t}</h3> }.into_any(),
            Group::Paragraph(t) => view! { <p style="margin:0">{t}</p> }.into_any(),
            Group::List(items) => {
                let lis: Vec<_> = items
                    .into_iter()
                    .map(|i| {
                        let lead = (!i.lead.is_empty()).then(|| view! { <strong>{i.lead}</strong>" " });
                        view! { <li>{lead}{i.text}</li> }
                    })
                    .collect();
                view! { <ul style="margin:0;padding-left:20px" class="stack">{lis}</ul> }.into_any()
            }
            Group::Table(rows) => {
                let trs: Vec<_> = rows
                    .into_iter()
                    .map(|r| {
                        let head = r.head;
                        let cells: Vec<_> = r
                            .cells
                            .into_iter()
                            .map(|c| {
                                if head {
                                    view! { <th style=CELL>{c}</th> }.into_any()
                                } else {
                                    view! { <td style=CELL>{c}</td> }.into_any()
                                }
                            })
                            .collect();
                        view! { <tr>{cells}</tr> }
                    })
                    .collect();
                view! { <table class="small" style="border-collapse:collapse">{trs}</table> }.into_any()
            }
        })
        .collect::<Vec<_>>()
}

/// Settings → What's new drawn from an answer of `GET /api/changelog`, for `app` ("Encore"). From
/// `whats_new.rs` `render`.
#[component]
pub fn ChangelogView(
    /// The app's name, in "This server runs Encore 0.36.4".
    #[prop(into)]
    app: String,
    /// The answer of `GET /api/changelog`.
    answer: ChangelogAnswer,
) -> impl IntoView {
    let list = if answer.releases.is_empty() {
        view! { <div class="card muted">{NO_RELEASE_NOTES}</div> }.into_any()
    } else {
        answer
            .releases
            .iter()
            .enumerate()
            .map(|(i, x)| {
                view! {
                    <details class="card settings-card" open=i == 0>
                        <summary class="row" style="justify-content:space-between"><h2 style="margin:0">{x.title.clone()}</h2><span class="row" style="gap:8px"><span class="badge">{x.version.clone()}</span><span class="chev" {..html(chevron())}>{""}</span></span></summary>
                        <div class="stack" style="gap:10px;margin-top:12px">{notes_view(&x.blocks)}</div>
                    </details>
                }
            })
            .collect::<Vec<_>>()
            .into_any()
    };
    view! {
        <div class="stack">
            <div class="card stack">
                <h2>{WHATS_NEW}</h2>
                <p class="small muted" style="margin:0">{settings_intro(&app, &answer.version)}</p>
                <label class="switch"><input type="checkbox" id="wn-on" checked=answer.popup on:change=move |e| save_popup(event_target_checked(&e))/><i></i><span>{SHOW_WHATS_NEW_AFTER_UPDATES}</span></label>
            </div>
            {list}
        </div>
    }
}

/// Settings → What's new: loads `GET /api/changelog` and draws [`ChangelogView`] (the error's sentence when the
/// call fails, as Encore's page shows it).
#[component]
pub fn ChangelogPage(
    /// The app's name.
    #[prop(into)]
    app: String,
) -> impl IntoView {
    let answer: RwSignal<Option<Result<ChangelogAnswer, String>>, LocalStorage> = RwSignal::new_local(None);
    crate::rt::spawn(async move {
        let r = api::get("/changelog").await.map(|v| serde_json::from_value(v).unwrap_or_default()).map_err(|e| e.message);
        answer.set(Some(r));
    });
    move || match answer.get() {
        None => view! { <div class="skeleton" style="height:200px"></div> }.into_any(),
        Some(Ok(a)) => view! { <ChangelogView app=app.clone() answer=a/> }.into_any(),
        Some(Err(e)) => view! { <div class="error-box">{e}</div> }.into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::esc;
    use crate::testing::{assert_same_html, render};
    use serde_json::{json, Value};

    // ---- Encore's functions (web-rs/src/whats_new.rs), as they build the HTML -------------------------------

    fn s(v: &Value, k: &str) -> String {
        v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
    }
    fn a(v: &Value, k: &str) -> Vec<Value> {
        v.get(k).and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn release_html(r: &Value) -> String {
        let summary = s(r, "summary");
        let highlights: String = a(r, "highlights").iter().map(|h| format!("<li>{}</li>", esc(h.as_str().unwrap_or("")))).collect();
        format!(
            r##"<div class="stack" style="gap:6px"><h3 style="margin:0">{}</h3>{}{}</div>"##,
            esc(&s(r, "title")),
            if summary.is_empty() { String::new() } else { format!(r##"<p class="muted" style="margin:0">{}</p>"##, esc(&summary)) },
            if highlights.is_empty() { String::new() } else { format!(r##"<ul style="margin:0;padding-left:20px">{highlights}</ul>"##) }
        )
    }
    fn block_html(b: &Value) -> String {
        match s(b, "k").as_str() {
            "h" => format!(r##"<h3 style="margin:10px 0 0">{}</h3>"##, esc(&s(b, "text"))),
            "p" => format!(r##"<p style="margin:0">{}</p>"##, esc(&s(b, "text"))),
            "li" => {
                let lead = s(b, "lead");
                format!(
                    r##"<li>{}{}</li>"##,
                    if lead.is_empty() { String::new() } else { format!("<strong>{}</strong> ", esc(&lead)) },
                    esc(&s(b, "text"))
                )
            }
            "row" => {
                let tag = if b.get("head").and_then(Value::as_bool).unwrap_or(false) { "th" } else { "td" };
                format!(
                    "<tr>{}</tr>",
                    a(b, "cells")
                        .iter()
                        .map(|c| format!(
                            r##"<{tag} style="text-align:left;padding:4px 10px 4px 0;vertical-align:top">{}</{tag}>"##,
                            esc(c.as_str().unwrap_or(""))
                        ))
                        .collect::<String>()
                )
            }
            _ => String::new(),
        }
    }
    fn notes_html(blocks: &[Value]) -> String {
        let mut out = String::new();
        let mut open = "";
        for b in blocks {
            let want = match s(b, "k").as_str() {
                "li" => "ul",
                "row" => "table",
                _ => "",
            };
            if want != open {
                if !open.is_empty() {
                    out.push_str(&format!("</{open}>"));
                }
                match want {
                    "ul" => out.push_str(r##"<ul style="margin:0;padding-left:20px" class="stack">"##),
                    "table" => out.push_str(r##"<table class="small" style="border-collapse:collapse">"##),
                    _ => {}
                }
                open = want;
            }
            out.push_str(&block_html(b));
        }
        if !open.is_empty() {
            out.push_str(&format!("</{open}>"));
        }
        out
    }
    fn render_html(r: &Value) -> String {
        let releases = a(r, "releases");
        let list: String = releases
            .iter()
            .enumerate()
            .map(|(i, x)| {
                format!(
                    r##"<details class="card settings-card" {open}>
          <summary class="row" style="justify-content:space-between"><h2 style="margin:0">{title}</h2><span class="row" style="gap:8px"><span class="badge">{ver}</span><span class="chev">{chev}</span></span></summary>
          <div class="stack" style="gap:10px;margin-top:12px">{notes}</div></details>"##,
                    open = if i == 0 { "open" } else { "" },
                    title = esc(&s(x, "title")),
                    ver = esc(&s(x, "version")),
                    chev = chevron(),
                    notes = notes_html(&a(x, "blocks"))
                )
            })
            .collect();
        format!(
            r##"<div class="stack">
          <div class="card stack">
            <h2>What's new</h2>
            <p class="small muted" style="margin:0">This server runs Encore {ver}. After an update, a short list of what changed shows once on the website and in the desktop app.</p>
            <label class="switch"><input type="checkbox" id="wn-on" {on}><i></i><span>Show what's new after updates</span></label>
          </div>
          {list}
        </div>"##,
            ver = esc(&s(r, "version")),
            on = if r.get("popup").and_then(Value::as_bool).unwrap_or(false) { "checked" } else { "" },
            list =
                if releases.is_empty() { r##"<div class="card muted">No release notes in this build.</div>"##.to_string() } else { list }
        )
    }

    fn answer() -> Value {
        json!({ "version": "0.36.4", "popup": true, "releases": [
            { "version": "0.36.0", "title": "Encore 0.36.0", "summary": "Clean up <your> library.", "highlights": ["Delete albums by type.", "See who requested what."],
              "blocks": [
                { "k": "p", "text": "Clean up your library & more." },
                { "k": "h", "text": "Library" },
                { "k": "li", "lead": "Delete albums by type.", "text": "Library → Albums has a Type filter." },
                { "k": "li", "text": "How the type is known." },
                { "k": "row", "cells": ["Setting", "Default"], "head": true },
                { "k": "row", "cells": ["a", "b"] },
                { "k": "quote", "text": "unknown" },
                { "k": "li", "text": "after an unknown block" }
              ] },
            { "version": "0.35.2", "title": "Encore 0.35.2", "summary": "", "highlights": [], "blocks": [] }
        ] })
    }

    #[test]
    fn the_changelog_page_is_encores() {
        let v = answer();
        let a: ChangelogAnswer = serde_json::from_value(v.clone()).unwrap();
        assert_same_html(&render(|| view! { <ChangelogView app="Encore" answer=a/> }), &render_html(&v));
        // no notes in this build, popup off
        let empty = json!({ "version": "1.0.0", "popup": false, "releases": [] });
        let a: ChangelogAnswer = serde_json::from_value(empty.clone()).unwrap();
        assert_same_html(&render(|| view! { <ChangelogView app="Encore" answer=a/> }), &render_html(&empty));
    }

    #[test]
    fn the_popup_is_encores() {
        let v = answer();
        let a: ChangelogAnswer = serde_json::from_value(v.clone()).unwrap();
        let want =
            format!(r##"<div class="stack" style="gap:18px">{}</div>"##, a_releases(&v).iter().map(release_html).collect::<String>());
        assert_same_html(&render(|| popup_body(&a.releases)), &want);
        let foot = r##"<label class="switch" style="margin-right:auto"><input type="checkbox" id="wn-on" checked><i></i><span>Show this after updates</span></label>
      <button class="btn" type="button" id="wn-full">Full changelog</button><button class="btn primary" type="button" data-close>Close</button>"##;
        assert_same_html(&render(|| popup_footer(|| {})), foot);
    }

    fn a_releases(v: &Value) -> Vec<Value> {
        a(v, "releases")
    }
}
