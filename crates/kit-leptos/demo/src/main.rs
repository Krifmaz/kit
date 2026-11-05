//! The kit-leptos demo: every component on one page with fake data, served by `trunk serve` (see README.md).
//!
//! The app is "Kit Demo" 1.4.0 ([`config`]); a fake [`Backend`] answers every call the components make, so the
//! page needs no server. Once it is drawn the page shows toasts, a confirm dialog and the banners through the
//! kit's API and compares each element with the kit's own expected markup (both parsed by the same browser,
//! comments and whitespace-only text left out, attributes in any order). The result goes into
//! `<pre id="kit-check">` and `document.body.dataset.kitCheck` ("pass" or "fail"), for the screenshot script to
//! read.

use kit_leptos::api::{ApiError, ApiFuture, Backend};
use kit_leptos::config::{self, AppConfig};
use kit_leptos::diagnostics::DiagnosticsPage;
use kit_leptos::dropzone::DropZone;
use kit_leptos::html::esc;
use kit_leptos::icons::icon;
use kit_leptos::modal::{self, ConfirmDialog};
use kit_leptos::notice::{self, NoticeSource};
use kit_leptos::palette::{self, Command, Palette, ACTIONS, GO_TO};
use kit_leptos::schema_form::schema::{visible_cards, Schema};
use kit_leptos::schema_form::{SettingsCards, SAMPLE_SCHEMA};
use kit_leptos::toasts::{self, error_toast_html, toast, toast_action};
use kit_leptos::update::{self, UpdateConfig};
use kit_leptos::whats_new::{self, ChangelogPage};
use kit_leptos::{bug_report, details, dropzone, offline, rt};
use kit_rules::app::App;
use kit_rules::dialogs::Confirm;
use kit_rules::notices::CheckWords;
use kit_rules::update::Health;
use leptos::prelude::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// The demo's logo: a rounded square in the theme's accent gradient.
const LOGO: &str = "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 32 32'%3E%3Cdefs%3E%3ClinearGradient id='g' x1='0' y1='0' x2='1' y2='1'%3E%3Cstop offset='0' stop-color='%238b5cf6'/%3E%3Cstop offset='1' stop-color='%23ec4899'/%3E%3C/linearGradient%3E%3C/defs%3E%3Crect width='32' height='32' rx='8' fill='url(%23g)'/%3E%3Cpath d='M10 9v14M10 16l8-7M12 15l7 8' stroke='white' stroke-width='3' fill='none' stroke-linecap='round'/%3E%3C/svg%3E";
/// The notice the fake server reports.
const NOTICE: &str = "The data folder /data is read-only – check the disk or the mount";
/// Check again's words for that notice.
const STORAGE_WORDS: CheckWords = CheckWords {
    failing: "The data folder still can't be written",
    checking: "Still checking the data folder",
    fixed: "The data folder can be written again",
};
/// What holds the demo page while its switch is on.
const HOLD_WORDS: &str = "It reloads when the demo's switch is off";
/// The themes of `design/out/tokens.css`.
const THEMES: [(&str, &str); 4] = [("encore", "Encore"), ("dark", "Dark"), ("oled", "OLED black"), ("light", "Light")];

thread_local! {
    /// The settings the fake server keeps.
    static SETTINGS: RefCell<Value> = const { RefCell::new(Value::Null) };
    /// How often Check again ran (the second one finds the notice fixed).
    static CHECKS: Cell<u32> = const { Cell::new(0) };
    /// The web build the fake server serves (Publish a new build changes it).
    static BUILD: RefCell<String> = RefCell::new("demo-1".into());
    /// The demo's switch that holds the page.
    static HOLD: Cell<bool> = const { Cell::new(true) };
}

/// Answers every call with fake data.
struct Fake;

fn releases() -> Value {
    json!([
        { "version": "1.4.0", "title": "Kit Demo 1.4.0", "summary": "Share folders with a link.",
          "highlights": ["Share a folder by link", "Links that expire"],
          "blocks": [
            { "k": "h", "text": "Sharing" },
            { "k": "li", "lead": "Share a folder by link.", "text": "Folders → Share makes a link anyone can open." },
            { "k": "li", "lead": "Links that expire.", "text": "Pick a day and the link stops working." },
            { "k": "h", "text": "Settings" },
            { "k": "row", "cells": ["Setting", "Default"], "head": true },
            { "k": "row", "cells": ["Link lifetime", "30 days"] },
            { "k": "p", "text": "Thanks for the reports." }
          ] },
        { "version": "1.3.0", "title": "Kit Demo 1.3.0", "summary": "Notes on every file.",
          "highlights": ["Notes"], "blocks": [{ "k": "li", "text": "Write a note on any file." }] }
    ])
}

fn diagnostics() -> Value {
    json!({
        "version": "1.4.0", "uid": 1000, "time": 1_793_800_000,
        "checks": [
            { "name": "Mail server", "kind": "service", "request": "EHLO mail.example.org:587", "ok": true, "detail": "250 OK", "status": 200, "ms": 84 },
            { "name": "Search index", "kind": "service", "request": "GET http://search:7700/health", "ok": false, "detail": "connection refused", "status": 502, "body": "<html>Bad gateway</html>" },
            { "name": "Data folder", "kind": "path", "ok": true, "detail": "/data · writable" },
            { "name": "Disk space", "kind": "path", "ok": true, "status": "warn", "detail": "92% full", "fix": "Free some space or grow the volume" },
            { "name": "convert", "kind": "tool", "ok": true, "detail": "7.1" },
            { "name": "Job queue", "kind": "queue", "ok": true, "detail": "0 waiting · 3 workers" }
        ],
        "settings": [
            { "key": "data_dir", "value": "/data", "source": "env", "env": "APP_DATA_DIR" },
            { "key": "app_name", "value": "Kit Demo", "source": "ui" },
            { "key": "session_days", "value": 30, "source": "default" }
        ]
    })
}

/// The sample schema's fields at their defaults, a few of them set elsewhere.
fn settings() -> Value {
    let schema: Value = serde_json::from_str(SAMPLE_SCHEMA).unwrap_or_default();
    let mut values = serde_json::Map::new();
    let mut meta = serde_json::Map::new();
    let groups = schema["sections"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .flat_map(|s| s["groups"].as_array().cloned().unwrap_or_default());
    for f in groups.flat_map(|g| g["fields"].as_array().cloned().unwrap_or_default()) {
        let key = f["key"].as_str().unwrap_or("").to_string();
        values.insert(key.clone(), f["default"].clone());
        meta.insert(key, json!({ "env": f["env"], "source": "default" }));
    }
    values.insert("smtp_host".into(), json!("mail.example.org"));
    meta.insert("smtp_host".into(), json!({ "env": "APP_SMTP_HOST", "source": "env" }));
    values.insert("smtp_password".into(), json!("•••• (saved)"));
    meta.insert("smtp_password".into(), json!({ "env": "APP_SMTP_PASSWORD", "source": "ui" }));
    values.insert("_meta".into(), Value::Object(meta));
    Value::Object(values)
}

fn notices() -> Value {
    let fixed = CHECKS.with(Cell::get) >= 2;
    json!({ "notices": [
        { "id": "storage", "ok": fixed, "text": if fixed { Value::Null } else { json!(NOTICE) }, "severity": "warn" },
        { "id": "mail", "ok": true }
    ] })
}

impl Backend for Fake {
    fn call(&self, method: &str, path: &str, body: Option<Value>) -> ApiFuture {
        let (method, path) = (method.to_string(), path.to_string());
        Box::pin(async move {
            rt::sleep(150).await;
            match (method.as_str(), path.as_str()) {
                ("GET", "/changelog/new") => Ok(json!({ "show": true, "releases": releases() })),
                ("GET", "/changelog") => Ok(json!({ "version": "1.4.0", "popup": true, "releases": releases() })),
                ("POST", "/changelog/seen") | ("PUT", "/auth/prefs") => Ok(json!({ "ok": true })),
                ("GET", "/diagnostics") => Ok(diagnostics()),
                ("GET", "/kit/suite/sidebar") => Ok(json!({ "items": SIDEBAR.with(|s| s.borrow().clone()) })),
                ("PUT", "/kit/suite/sidebar") => Ok(SIDEBAR.with(|s| {
                    // the demo keeps the order it was sent and marks the hidden entries
                    let p: kit_rules::suite::SidebarPrefs = body.and_then(|b| serde_json::from_value(b).ok()).unwrap_or_default();
                    let mut items: Vec<kit_rules::suite::SidebarItem> = serde_json::from_value(s.borrow().clone()).unwrap_or_default();
                    items.sort_by_key(|i| p.order.iter().position(|k| *k == i.key).unwrap_or(usize::MAX));
                    for i in &mut items {
                        i.hidden = p.hidden.contains(&i.key);
                    }
                    *s.borrow_mut() = serde_json::to_value(&items).unwrap_or_default();
                    json!({ "items": items })
                })),
                ("GET", p) if p.starts_with("/kit/suite/search") => Ok(json!({ "groups": [
                    { "app": "demo", "app_name": "Demo", "icon": DEMO_ICON, "local": true, "results": [{ "title": "First thing", "subtitle": "Things", "url": "#things/1" }] },
                    { "app": "other", "app_name": "Other demo", "icon": OTHER_ICON, "local": false, "results": [{ "title": "A related item", "subtitle": "Items", "url": "#items/1" }] },
                    { "app": "down", "app_name": "Third demo", "icon": OTHER_ICON, "local": false, "results": [], "error": "http://third.test can't be reached. Check the address and that the app is running." }
                ] })),
                ("GET", "/notices") => Ok(notices()),
                ("POST", "/notices/check") => {
                    CHECKS.with(|c| c.set(c.get() + 1));
                    Ok(notices())
                }
                ("GET", "/settings") => Ok(SETTINGS.with(|s| s.borrow().clone())),
                ("PUT", "/settings") => Ok(SETTINGS.with(|s| {
                    let mut s = s.borrow_mut();
                    if let (Some(o), Some(Value::Object(b))) = (s.as_object_mut(), body) {
                        o.extend(b);
                    }
                    s.clone()
                })),
                ("GET", p) if p.starts_with("/settings/secret/") => Ok(json!({ "value": "demo-secret-value" })),
                ("POST", "/diagnostics/mail/test") => {
                    Ok(json!({ "ok": false, "message": "535 Authentication failed: the password was refused" }))
                }
                _ => Err(ApiError { status: 404, message: format!("The demo has no {method} {path}"), data: Value::Null }),
            }
        })
    }

    fn post_form(&self, path: &str, _form: web_sys::FormData) -> ApiFuture {
        let path = path.to_string();
        Box::pin(async move {
            rt::sleep(300).await;
            if path == "/bug-reports" {
                Ok(json!({ "ok": true, "id": 42 }))
            } else {
                Err(ApiError { status: 404, message: "Not found".into(), data: Value::Null })
            }
        })
    }
}

fn navigate() -> Rc<dyn Fn(&str)> {
    Rc::new(|p: &str| toast(&format!("Would open {p}"), "info"))
}

fn name() -> String {
    config::app_name()
}

/// Sets `<html data-theme>`.
fn set_theme(theme: &str) {
    if let Some(html) = rt::document().and_then(|d| d.document_element()) {
        let _ = html.set_attribute("data-theme", theme);
    }
}

/// The head: the logo, the app's name and version, and the theme switcher.
#[component]
fn Head() -> impl IntoView {
    let c = config::get();
    let opts = THEMES.iter().map(|(id, label)| view! { <option value=*id>{*label}</option> }).collect_view();
    view! {
        <header class="row between wrap demo-head">
            <div class="row">
                <img src=c.logo.clone().unwrap_or_default() alt="" width="36" height="36"/>
                <div><h1 style="margin:0">{c.app.name.clone()}</h1><div class="small muted">{format!("kit-leptos demo · version {}", c.app.version)}</div></div>
            </div>
            <label class="row small muted">"Theme"
                <select class="input" id="theme" style="width:auto" on:change=move |e| set_theme(&event_target_value(&e))>{opts}</select>
            </label>
        </header>
    }
}

fn button(label: &'static str, f: impl Fn() + 'static) -> impl IntoView {
    view! { <button class="btn" on:click=move |_| f()>{label}</button> }
}

/// Toasts: each kind, an error with Copy, the same error merged, Undo and the stack's cap.
#[component]
fn ToastButtons() -> impl IntoView {
    view! {
        <div class="row wrap">
            {button("Saved", || toast("Settings saved", "ok"))}
            {button("Info", || toast("Two files were left out: they are larger than 8 MB", "info"))}
            {button("Warning", || toast("The disk is 92% full", "warn"))}
            {button("Error with Copy", || toast("upload failed: mkdir /data/<team> & \"co\": permission denied (os error 13)", "err"))}
            {button("Same error twice", || { toast("The server refused the file", "err"); toast("The server refused the file", "err"); })}
            {button("Five errors (three stay)", || for i in 1..=5 { toast(&format!("Error {i} of 5"), "err"); })}
            {button("Undo", || toast_action("Note deleted", "Undo", 6000, || toast("Note restored", "ok")))}
            {button("Clear", toasts::clear)}
        </div>
    }
}

/// Dialogs: the confirm dialog (the async call and the component), the details dialog and What's new.
#[component]
fn DialogButtons() -> impl IntoView {
    let asking = RwSignal::new(false);
    let ask = Confirm::new("Archive this project?").text("Its files stay, but nobody can change them.").ok("Archive");
    view! {
        <div class="row wrap">
            {button("Confirm (danger)", || rt::spawn(async {
                let yes = modal::confirm(Confirm::new("Delete this note?").text("It is gone for good.").ok("Delete").danger(true)).await;
                toast(if yes { "Deleted" } else { "Kept" }, if yes { "ok" } else { "info" });
            }))}
            {button("Confirm component", move || asking.set(true))}
            {button("Details", || { details::show("Search index", "GET http://search:7700/health\n502 Bad gateway\n<html>Bad gateway</html>"); })}
            {button("What's new popup", || { whats_new::popup(serde_json::from_value(releases()).unwrap_or_default(), navigate()); })}
            {button("New report", || {
                let mut o = bug_report::SheetOpts::from_config("#/demo");
                o.on_sent = Some(Rc::new(|id| toast(&format!("Report #{id} is in the list"), "info")));
                bug_report::open(o);
            })}
        </div>
        <Show when=move || asking.get()>
            <ConfirmDialog confirm=ask.clone() on_answer=Callback::new(move |yes: bool| {
                asking.set(false);
                toast(if yes { "Archived" } else { "Not archived" }, if yes { "ok" } else { "info" });
            })/>
        </Show>
    }
}

/// Self-update: the toast, and the real flow against the fake server.
#[component]
fn UpdateButtons() -> impl IntoView {
    view! {
        <div class="row wrap">
            {button("Update toast", || {
                let c = config::get();
                toasts::show_update(&c.words.update.ready(&c.app.name), c.words.update.reload, Rc::new(|| toast("Would reload", "info")));
                toasts::set_update_reason(Some(HOLD_WORDS));
            })}
            {button("Publish a new build", || {
                BUILD.with(|b| *b.borrow_mut() = "demo-2".into());
                update::check_now();
            })}
            <label class="switch"><input type="checkbox" checked on:change=move |e| HOLD.with(|h| h.set(event_target_checked(&e)))/><i></i><span>"Hold the page (a reload waits)"</span></label>
        </div>
    }
}

/// The offline banner (offline mode, with Retry) and the connection banner.
#[component]
fn Banners() -> impl IntoView {
    view! {
        <div class="row wrap">
            {button("Offline banner", || offline::show_banner(false, Rc::new(retry)))}
            {button("Offline, grace ended", || {
                if let Some(b) = rt::qs("body > #offline-banner") {
                    b.remove();
                }
                offline::show_banner(true, Rc::new(retry));
            })}
            {button("Connection banner", || offline::conn_banner(true, &config::get().words.offline.server_down(&name())))}
            {button("Hide the connection banner", || offline::conn_banner(false, ""))}
        </div>
        <p class="small muted">"Offline mode has its own banner, so the connection banner stays away while it is on."</p>
    }
}

/// Offline mode's Retry: tries for a moment, the server still does not answer.
fn retry() {
    offline::set_trying(true);
    rt::after(1200, || {
        offline::set_trying(false);
        toast(config::get().words.offline.still_offline, "warn");
    });
}

/// The palette's commands: a few actions and places, and fake sites loaded on opening.
fn commands() -> Palette {
    let jump = |label: &'static str| move || toast(&format!("Would go to {label}"), "info");
    Palette::new(vec![
        Command::new(ACTIONS, "Show a toast", || toast("Hello from the palette", "ok")).icon("check"),
        Command::new(ACTIONS, "Report a bug", || {
            bug_report::open(bug_report::SheetOpts::from_config("#/demo"));
        })
        .icon("bug"),
        Command::new(GO_TO, "Settings", jump("Settings")).hint("/settings").icon("settings"),
        Command::new(GO_TO, "Diagnostics", jump("Diagnostics")).hint("/diagnostics").icon("alert"),
        Command::new(GO_TO, "Changelog", jump("Changelog")).hint("/changelog"),
    ])
    .source(|| async {
        rt::sleep(150).await;
        (1..=8)
            .map(|n| {
                let host = format!("site{n}.example");
                Command::new("Sites", format!("Site {n}"), move || toast(&format!("Would open {host}"), "info"))
                    .hint(format!("site{n}.example"))
                    .icon("external")
            })
            .collect()
    })
}

/// The demo app's icon (SVG path data).
const DEMO_ICON: &str = "M4 4h16v16H4z";
/// The other demo app's icon.
const OTHER_ICON: &str = "M12 3l9 9-9 9-9-9z";

thread_local! {
    /// The demo's shared sidebar: its own section and another app's.
    static SIDEBAR: std::cell::RefCell<Value> = std::cell::RefCell::new(json!([
        { "key": "demo/things", "app": "demo", "app_name": "Demo", "section": "things", "title": "Things", "icon": DEMO_ICON, "url": "#things", "path": "#things", "hidden": false, "local": true },
        { "key": "other/items", "app": "other", "app_name": "Other demo", "section": "items", "title": "Items", "icon": OTHER_ICON, "url": "#items", "path": "/items", "hidden": false, "local": false },
        { "key": "other/more", "app": "other", "app_name": "Other demo", "section": "more", "title": "More", "icon": "", "url": "#more", "path": "/more", "hidden": true, "local": false }
    ]));
}

#[component]
fn Demo() -> impl IntoView {
    let schema = Schema::from_json(&serde_json::from_str(SAMPLE_SCHEMA).unwrap_or_default());
    let values = SETTINGS.with(|s| s.borrow().clone());
    let general = schema.section("general").map(|s| visible_cards(&s.groups, &values)).unwrap_or_default();
    let mail = schema.section("mail").map(|s| visible_cards(&s.groups, &values)).unwrap_or_default();
    let v2 = values.clone();
    view! {
        <Head/>
        <p class="muted">"Every kit component with fake data. The check below compares the page with the kit's own markup."</p>
        <pre id="kit-check" class="code small">"checking…"</pre>
        <section><h2>"Toasts"</h2><ToastButtons/></section>
        <section><h2>"Dialogs and the bug report sheet"</h2><DialogButtons/></section>
        <section><h2>"Self-update"</h2><UpdateButtons/></section>
        <section><h2>"Offline and connection banners"</h2><Banners/></section>
        <section>
            <h2>"Command palette"</h2>
            <p class="muted">"Ctrl+K or ⌘K opens it; the sites are loaded each time it opens (eight, six show until you type)."</p>
            <div class="row wrap">{button("Open the command palette", palette::open)}</div>
        </section>
        <section>
            <h2>"Drop zone"</h2>
            <DropZone input="demo-files" class="card">
                <p class="muted" style="margin:0">"Drop files here, or "
                    <label class="btn sm">"pick them"<input type="file" id="demo-files" multiple hidden on:change=move |e| {
                        let Some(i) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return };
                        let names: Vec<String> = i.files().map(|l| (0..l.length()).filter_map(|n| l.get(n)).map(|f| f.name()).collect()).unwrap_or_default();
                        toast(&format!("Picked {}", names.join(", ")), "ok");
                        i.set_value("");
                    }/></label>
                </p>
            </DropZone>
        </section>
        <section><h2>"Settings from a schema: General"</h2><SettingsCards cards=general values=values/></section>
        <section><h2>"Settings from a schema: Mail"</h2><SettingsCards cards=mail values=v2/></section>
        <section><h2>"Suite: the shared sidebar"</h2><div class="card nav"><kit_leptos::suite::SuiteNav/></div></section>
        <section><h2>"Suite: search across apps"</h2><kit_leptos::suite::SuiteSearch initial="thing"/></section>
        <section><DiagnosticsPage/></section>
        <section><ChangelogPage/></section>
    }
}

/// The kit's markup for what the check looks at.
mod want {
    use super::*;

    pub fn plain(msg: &str, kind: &str) -> String {
        format!(r#"<div class="toast {kind}">{}</div>"#, esc(msg))
    }
    pub fn error(msg: &str) -> String {
        format!(r#"<div class="toast err stays" role="alert" data-msg="{}">{}</div>"#, esc(msg), error_toast_html(msg))
    }
    pub fn action(msg: &str, label: &str) -> String {
        format!(
            r#"<div class="toast act"><div class="toast-text">{}</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-toast-action>{}</button></div></div>"#,
            esc(msg),
            esc(label)
        )
    }
    pub fn update(text: &str, why: &str, label: &str) -> String {
        format!(
            r#"<div id="sw-update" class="toast" role="status" style="display:flex;align-items:center;gap:12px"><span class="grow">{text}<span class="small muted" data-update-why style="display:block">{why}</span></span><button class="btn sm primary" type="button">{label}</button></div>"#
        )
    }
    pub fn confirm(title: &str, text: &str, ok: &str, style: &str) -> String {
        format!(
            r#"<div class="modal-bg"><div class="modal" role="dialog" aria-modal="true"><div class="modal-head"><h2>{title}</h2><button class="btn ghost icon" data-close aria-label="Close">{x}</button></div><div class="modal-body"><p class="confirm-text muted">{text}</p></div><div class="modal-foot"><button class="btn" data-close>Cancel</button><button class="btn {style}" data-ok>{ok}</button></div></div></div>"#,
            x = icon("x")
        )
    }
    pub fn notice(id: &str, text: &str) -> String {
        format!(
            r#"<div class="notice-banner warn" role="status" data-notice="{id}">{}<span>{}</span><button class="btn sm" data-notice-check>{}Check again</button></div>"#,
            icon("alert"),
            esc(text),
            icon("refresh")
        )
    }
    pub fn offline(text: &str) -> String {
        format!(
            r#"<div id="offline-banner" class="offline-banner" role="status">{}<span>{}</span><button class="btn sm" type="button" id="offline-retry">Retry</button></div>"#,
            icon("offline"),
            esc(text)
        )
    }
    pub fn conn(text: &str) -> String {
        format!(r#"<div id="conn-banner">{}<span>{}</span></div>"#, icon("x"), esc(text))
    }
}

/// `node` without comments and whitespace-only text, its text runs joined: the DOM the eye sees.
fn tidy(node: &web_sys::Node) {
    let kids = node.child_nodes();
    let list: Vec<web_sys::Node> = (0..kids.length()).filter_map(|i| kids.item(i)).collect();
    for k in list {
        let blank = k.node_type() == web_sys::Node::TEXT_NODE && k.text_content().unwrap_or_default().trim().is_empty();
        if k.node_type() == web_sys::Node::COMMENT_NODE || blank {
            let _ = node.remove_child(&k);
        } else {
            tidy(&k);
        }
    }
    node.normalize();
}

/// The first element `html` makes in this browser, tidied.
fn parsed(html: &str) -> Option<web_sys::Node> {
    let div = rt::document()?.create_element("div").ok()?;
    div.set_inner_html(html);
    let first: web_sys::Node = div.first_element_child()?.into();
    tidy(&first);
    Some(first)
}

/// One comparison: the element against the kit's markup.
fn same(name: &str, got: Option<web_sys::Element>, want: &str, out: &mut Vec<(String, bool, String)>) {
    let got_node = got.as_ref().and_then(|e| e.clone_node_with_deep(true).ok());
    if let Some(n) = &got_node {
        tidy(n);
    }
    let want_node = parsed(want);
    let ok = matches!((&got_node, &want_node), (Some(g), Some(w)) if g.is_equal_node(Some(w)));
    let why = if ok {
        String::new()
    } else {
        let html = |n: &Option<web_sys::Node>| {
            n.as_ref().and_then(|n| n.dyn_ref::<web_sys::Element>().map(|e| e.outer_html())).unwrap_or_default()
        };
        format!("\n  got  {}\n  want {}", html(&got_node), html(&want_node))
    };
    out.push((name.to_string(), ok, why));
}

fn toast_els() -> Vec<web_sys::Element> {
    rt::by_id("toasts")
        .map(|h| {
            let c = h.children();
            (0..c.length()).filter_map(|i| c.item(i)).collect()
        })
        .unwrap_or_default()
}

/// Shows toasts, a confirm dialog and the banners through the kit's API and compares each with the kit's markup.
/// Leptos draws a change on its next tick, so the check waits a moment after each step.
async fn check() {
    let mut out = Vec::new();
    let c = config::get();
    let tick = || rt::sleep(60);
    // the stack: three errors stay, the same one again is shown once
    toasts::clear();
    for m in ["a", "b", "a", "c", "d"] {
        toast(m, "err");
    }
    tick().await;
    let errs: Vec<String> = toast_els().iter().map(|e| e.get_attribute("data-msg").unwrap_or_default()).collect();
    out.push(("three errors stay, the same one is shown once".to_string(), errs == ["b", "c", "d"], format!(" ({errs:?})")));
    toasts::clear();
    toast("Settings saved", "ok");
    toast("upload failed: <a.png> & \"b\"", "err");
    toast("Two files were left out", "");
    toast_action("Note deleted", "Undo", 60_000, || {});
    toasts::show_update(&c.words.update.ready(&c.app.name), c.words.update.reload, Rc::new(|| {}));
    toasts::set_update_reason(Some(HOLD_WORDS));
    tick().await;
    let only_elements = rt::by_id("toasts").is_some_and(|h| h.child_nodes().length() == h.children().length());
    out.push(("#toasts holds only the toasts".to_string(), only_elements, String::new()));
    let kids = toast_els();
    same("toast ok", kids.first().cloned(), &want::plain("Settings saved", "ok"), &mut out);
    same("error toast with Copy", kids.get(1).cloned(), &want::error("upload failed: <a.png> & \"b\""), &mut out);
    same("toast with no kind", kids.get(2).cloned(), &want::plain("Two files were left out", ""), &mut out);
    same("Undo toast", kids.get(3).cloned(), &want::action("Note deleted", "Undo"), &mut out);
    same("update toast", kids.get(4).cloned(), &want::update("A new version of Kit Demo is ready", HOLD_WORDS, "Reload"), &mut out);
    // the confirm dialog, then a new page closes it: no
    let answer: Rc<Cell<Option<bool>>> = Rc::default();
    let a2 = answer.clone();
    rt::spawn(async move {
        a2.set(Some(modal::confirm(Confirm::new("Delete this note?").text("It is gone for good.").ok("Delete").danger(true)).await));
    });
    tick().await;
    same(
        "confirm dialog",
        rt::qs("#modal-root .modal-bg"),
        &want::confirm("Delete this note?", "It is gone for good.", "Delete", "danger"),
        &mut out,
    );
    modal::close_all();
    tick().await;
    out.push(("a dialog closed another way answers no".to_string(), answer.get() == Some(false), format!(" ({:?})", answer.get())));
    // the connection banner first: offline mode (the banner below) keeps it away
    let down = c.words.offline.server_down(&c.app.name);
    offline::conn_banner(true, &down);
    tick().await;
    same("connection banner", rt::qs("body > #conn-banner"), &want::conn(&down), &mut out);
    offline::show_banner(false, Rc::new(retry));
    tick().await;
    out.push(("offline mode takes the connection banner away".to_string(), rt::qs("body > #conn-banner").is_none(), String::new()));
    same("offline banner", rt::qs("body > #offline-banner"), &want::offline(c.words.offline.banner), &mut out);
    same("notice banner", rt::qs("#server-notice .notice-banner"), &want::notice("storage", NOTICE), &mut out);
    let pass = out.iter().all(|(_, ok, _)| *ok);
    let text: String = out.iter().map(|(n, ok, why)| format!("{} {n}{why}\n", if *ok { "PASS" } else { "FAIL" })).collect();
    if let Some(pre) = rt::by_id("kit-check") {
        pre.set_text_content(Some(&text));
    }
    // leave a few toasts for the screenshot
    toasts::clear();
    toast("upload failed: mkdir /data/<team>: permission denied (os error 13)", "err");
    toast_action("Note deleted", "Undo", 600_000, || toast("Note restored", "ok"));
    toast("Settings saved", "ok");
    tick().await;
    if let Some(body) = rt::document().and_then(|d| d.body()) {
        let _ = body.set_attribute("data-kit-check", if pass { "pass" } else { "fail" });
    }
}

fn main() {
    let mut cfg = AppConfig::new(App::named("Kit Demo").with_version("1.4.0"));
    cfg.logo = Some(LOGO.into());
    config::set(cfg);
    SETTINGS.with(|s| *s.borrow_mut() = settings());
    kit_leptos::api::set_backend(Fake);
    toasts::mount();
    details::init();
    dropzone::init();
    notice::start(NoticeSource::default().words_for("storage", STORAGE_WORDS), true);
    notice::mount();
    let mut up = UpdateConfig::from_config().hold(HOLD_WORDS, || HOLD.with(Cell::get));
    up.build = Some("demo-1".into());
    up.health = Rc::new(|| -> update::HealthFuture {
        Box::pin(async {
            let web = BUILD.with(|b| b.borrow().clone());
            Health::parse(&json!({ "ok": true, "version": "1.4.0", "web": web }).to_string())
        })
    });
    update::start(up);
    palette::install(commands());
    if let Some(el) = rt::by_id("demo").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        leptos::mount::mount_to(el, Demo).forget();
    }
    // the notices' first answer comes after the fake backend's delay
    rt::after(600, || rt::spawn(check()));
}
