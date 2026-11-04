//! The kit-leptos demo: every component on one page with fake data, served by `trunk serve` (see README.md).
//!
//! A fake [`Backend`] answers every call the components make, so the page needs no server. Once it is drawn the
//! page compares the DOM of the toasts, a dialog and the banners with Encore's HTML (parsed by the same browser,
//! so both sides are serialised alike) and writes the result into `<pre id="kit-check">` and
//! `document.body.dataset.kitCheck` ("pass" or "fail"), for the screenshot script to read.

use kit_leptos::api::{ApiError, ApiFuture, Backend};
use kit_leptos::diagnostics::DiagnosticsPage;
use kit_leptos::dropzone::DropZone;
use kit_leptos::html::esc;
use kit_leptos::icons::icon;
use kit_leptos::modal::{self, ModalOpts};
use kit_leptos::notice::{self, NoticeSource};
use kit_leptos::schema_form::schema::{visible_cards, Schema};
use kit_leptos::schema_form::SettingsCards;
use kit_leptos::toasts::{self, error_toast_html, toast, toast_action};
use kit_leptos::whats_new::{self, ChangelogPage};
use kit_leptos::{bug_report, details, dropzone, offline, rt};
use kit_rules::offline::mode::{server_down, BANNER};
use kit_rules::update::{ready_text, RELOAD};
use leptos::prelude::*;
use serde_json::{json, Value};
use std::rc::Rc;
use wasm_bindgen::JsCast;

const APP: &str = "Encore";
const SCHEMA: &str = include_str!("../../src/schema_form/fixtures/settings-schema.json");
const NOTICE: &str = "The downloads folder /downloads is read-only – check the disk (dmesg, fsck) or the mount";

/// Answers every call with fake data.
struct Fake;

fn releases() -> Value {
    json!([
        { "version": "0.36.4", "title": "Encore 0.36.4", "summary": "Smaller fixes after 0.36.",
          "highlights": ["Pause for everyone", "Delete albums by type"],
          "blocks": [
            { "k": "h", "text": "Listening" },
            { "k": "li", "lead": "Pause for everyone.", "text": "The host pauses the whole group." },
            { "k": "h", "text": "Library" },
            { "k": "li", "lead": "Delete albums by type.", "text": "Singles, EPs or live albums in one go." },
            { "k": "p", "text": "Thanks for the reports." }
          ] },
        { "version": "0.36.0", "title": "Encore 0.36.0", "summary": "Group listening.",
          "highlights": ["Listen together"], "blocks": [{ "k": "li", "text": "Listen together with friends." }] }
    ])
}

fn diagnostics() -> Value {
    json!({
        "version": "0.36.4", "uid": 1000, "time": 1_793_800_000,
        "checks": [
            { "name": "MusicBrainz", "kind": "service", "request": "GET https://musicbrainz.org/ws/2/?fmt=json", "ok": true, "detail": "200 OK", "status": 200, "ms": 180 },
            { "name": "Prowlarr", "kind": "service", "request": "GET http://prowlarr:9696/api", "ok": false, "detail": "connection refused", "status": 502, "body": "<html>Bad gateway</html>" },
            { "name": "Music library", "kind": "path", "ok": true, "detail": "/music · writable" },
            { "name": "ffmpeg", "kind": "tool", "ok": true, "detail": "6.1" }
        ],
        "settings": [
            { "key": "library_path", "value": "/music", "source": "env", "env": "ENCORE_LIBRARY_PATH" },
            { "key": "app_name", "value": "Encore", "source": "ui" },
            { "key": "session_days", "source": "default" }
        ]
    })
}

/// Every field of the schema with an empty value (a bool off, a select on its first option).
fn values(schema: &Schema) -> Value {
    let mut m = serde_json::Map::new();
    for f in schema.sections.iter().flat_map(|s| s.groups.iter()).flat_map(|g| g.fields.iter()) {
        let v = match f.kind.as_str() {
            "bool" => json!(false),
            "select" => json!(f.options.first().map(|o| o.0.clone()).unwrap_or_default()),
            "checks" => json!([]),
            _ => json!(""),
        };
        m.insert(f.key.clone(), v);
    }
    m.insert("app_name".into(), json!("Encore"));
    m.insert("_meta".into(), json!({ "env": {} }));
    Value::Object(m)
}

impl Backend for Fake {
    fn call(&self, method: &str, path: &str, _body: Option<Value>) -> ApiFuture {
        let (method, path) = (method.to_string(), path.to_string());
        Box::pin(async move {
            rt::sleep(150).await;
            match (method.as_str(), path.as_str()) {
                ("GET", "/changelog/new") => Ok(json!({ "show": true, "releases": releases() })),
                ("GET", "/changelog") => Ok(json!({ "version": "0.36.4", "popup": true, "releases": releases() })),
                ("GET", "/system/diagnostics") => Ok(diagnostics()),
                ("GET", "/requests/storage") | ("POST", "/requests/storage/check") => {
                    Ok(json!({ "downloads": { "ok": false, "text": NOTICE, "waiting": 3 } }))
                }
                (_, p) if p.starts_with("/system/test/") => Ok(json!({ "ok": true, "detail": "Connected (demo)" })),
                _ => Ok(json!({})),
            }
        })
    }

    fn post_form(&self, path: &str, _form: web_sys::FormData) -> ApiFuture {
        let path = path.to_string();
        Box::pin(async move {
            rt::sleep(300).await;
            if path == "/bug-reports" {
                Ok(json!({ "id": 42 }))
            } else {
                Err(ApiError { status: 404, message: "Not found".into(), data: Value::Null })
            }
        })
    }
}

fn navigate() -> Rc<dyn Fn(&str)> {
    Rc::new(|p: &str| toast(&format!("Would open {p}"), ""))
}

/// The buttons that open the dialogs and show the toasts and banners.
#[component]
fn Buttons() -> impl IntoView {
    let b = |label: &'static str, f: Rc<dyn Fn()>| view! { <button class="btn" on:click=move |_| f()>{label}</button> };
    view! {
        <div class="row wrap" style="gap:8px">
            {b("Toast", Rc::new(|| toast("Saved", "ok")))}
            {b("Error toast", Rc::new(|| toast("import failed: <Band> & \"Co\"", "err")))}
            {b("Undo toast", Rc::new(|| toast_action("Up next cleared", "Undo", 6000, || toast("Restored", "ok"))))}
            {b("Update toast", Rc::new(|| { toasts::show_update(&ready_text(APP), RELOAD, Rc::new(|| toast("Would reload", ""))); }))}
            {b("Confirm", Rc::new(|| rt::spawn(async {
                let ok = modal::confirm_dialog("Delete album?", "Its files are removed from the disk.", "Delete", true).await;
                toast(if ok { "Deleted" } else { "Kept" }, "");
            })))}
            {b("Details", Rc::new(|| { details::show("Prowlarr", "GET http://prowlarr:9696/api\n502 Bad gateway\n<html>Bad gateway</html>"); }))}
            {b("What's new", Rc::new(|| { whats_new::popup(serde_json::from_value(releases()).unwrap_or_default(), navigate()); }))}
            {b("Bug report", Rc::new(|| { bug_report::open(bug_report::SheetOpts::new(APP, "#/library", "0.36.4")); }))}
            {b("Offline banner", Rc::new(|| offline::show_banner(false, Rc::new(|| offline::set_trying(true)))))}
            {b("Connection banner", Rc::new(|| offline::conn_banner(true, &server_down(APP))))}
        </div>
    }
}

#[component]
fn Demo() -> impl IntoView {
    let schema = Schema::from_json(&serde_json::from_str(SCHEMA).unwrap_or_default());
    let values = values(&schema);
    let cards = schema.section("general").map(|s| visible_cards(&s.groups, &values)).unwrap_or_default();
    view! {
        <h1>"kit-leptos"</h1>
        <p class="muted">"Every component of the kit with fake data. The check below compares the DOM with Encore's HTML."</p>
        <pre id="kit-check" class="small mono">"checking…"</pre>
        <section><h2>"Toasts, dialogs and banners"</h2><Buttons/></section>
        <section>
            <h2>"Drop zone"</h2>
            <DropZone input="demo-files" class="card flat">
                <p class="muted">"Drop files here, or "<label class="btn sm">"pick them"<input type="file" id="demo-files" multiple hidden/></label></p>
            </DropZone>
        </section>
        <section><h2>"Settings form (Encore's schema, page General)"</h2><SettingsCards cards=cards values=values/></section>
        <section><DiagnosticsPage app=APP/></section>
        <section><h2>"What's new"</h2><ChangelogPage app=APP/></section>
    }
}

/// Encore's HTML for what the check looks at (`ui.rs` `toast`, `error_toast`, `toast_action`, `modal`;
/// `update.rs` `show_toast`; `notice.rs` `banner_html`; `mode.rs` `banner`; `app.rs` `conn_banner`).
mod encore {
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
    pub fn update(text: &str, label: &str) -> String {
        format!(
            r#"<div id="sw-update" class="toast" role="status" style="display:flex;align-items:center;gap:12px"><span class="grow">{text}<span class="small muted" data-update-why style="display:block"></span></span><button class="btn sm primary" type="button">{label}</button></div>"#
        )
    }
    pub fn modal(title: &str, body: &str, footer: &str) -> String {
        format!(
            r#"<div class="modal-bg"><div class="modal " role="dialog" aria-modal="true">
    <div class="modal-head"><h2>{title}</h2><button class="btn ghost icon" data-close aria-label="Close">{}</button></div>
    <div class="modal-body">{body}</div>
    <div class="modal-foot">{footer}</div>
  </div></div>"#,
            icon("x")
        )
    }
    pub fn notice(text: &str) -> String {
        format!(
            r#"<div class="storage-banner" role="status">{}<span>{}</span><button class="btn sm" data-notice-check>{}Check again</button></div>"#,
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

/// The HTML `html` makes in this browser (so it is serialised the way the browser serialises the kit's DOM).
fn parsed(html: &str) -> String {
    let Some(d) = rt::document() else { return String::new() };
    let Ok(div) = d.create_element("div") else { return String::new() };
    div.set_inner_html(html);
    div.inner_html()
}

/// One comparison: the element's `outerHTML` against Encore's HTML.
fn same(name: &str, got: Option<web_sys::Element>, want: &str, out: &mut Vec<(String, bool, String)>) {
    let got = got.map(|e| e.outer_html()).unwrap_or_default();
    let want = parsed(want);
    let ok = got == want;
    out.push((name.to_string(), ok, if ok { String::new() } else { format!("\n  got  {got}\n  want {want}") }));
}

/// Draws the toasts, a dialog and the banners through the kit's API and compares each with Encore's HTML.
fn check() {
    toasts::clear();
    toast("Saved", "ok");
    toast("import failed: <Band> & \"Co\"", "err");
    toast("Some files were left out", "");
    toast_action("Up next cleared", "Undo", 60_000, || {});
    toasts::show_update(&ready_text(APP), RELOAD, Rc::new(|| {}));
    let m = modal::open(
        ModalOpts::new("Delete album?", "<p class=\"muted\" style=\"margin:0\">Gone for good.</p>")
            .footer("<button class=\"btn\" data-close>Cancel</button><button class=\"btn danger\" data-ok>Delete</button>"),
    );
    let mut out = Vec::new();
    // the connection banner first: offline mode (the banner below) keeps it away, as in Encore
    offline::conn_banner(true, &server_down(APP));
    same("connection banner", rt::by_id("conn-banner"), &encore::conn(&server_down(APP)), &mut out);
    offline::show_banner(false, Rc::new(|| {}));
    out.push(("offline mode takes the connection banner away".to_string(), rt::by_id("conn-banner").is_none(), String::new()));
    let host = rt::by_id("toasts");
    let kids: Vec<web_sys::Element> = host
        .as_ref()
        .map(|h| {
            let c = h.children();
            (0..c.length()).filter_map(|i| c.item(i)).collect()
        })
        .unwrap_or_default();
    let only_elements = host.as_ref().is_some_and(|h| h.child_nodes().length() == h.children().length());
    out.push(("#toasts holds only the toasts (no markers)".to_string(), only_elements, String::new()));
    same("toast ok", kids.first().cloned(), &encore::plain("Saved", "ok"), &mut out);
    same("error toast", kids.get(1).cloned(), &encore::error("import failed: <Band> & \"Co\""), &mut out);
    same("toast with no kind", kids.get(2).cloned(), &encore::plain("Some files were left out", ""), &mut out);
    same("action toast", kids.get(3).cloned(), &encore::action("Up next cleared", "Undo"), &mut out);
    same("update toast", kids.get(4).cloned(), &encore::update(&ready_text(APP), RELOAD), &mut out);
    same(
        "dialog",
        m.bg(),
        &encore::modal(
            "Delete album?",
            "<p class=\"muted\" style=\"margin:0\">Gone for good.</p>",
            "<button class=\"btn\" data-close>Cancel</button><button class=\"btn danger\" data-ok>Delete</button>",
        ),
        &mut out,
    );
    same("notice banner", rt::qs("#server-notice .storage-banner"), &encore::notice(NOTICE), &mut out);
    same("offline banner", rt::by_id("offline-banner"), &encore::offline(BANNER), &mut out);
    m.close();
    let pass = out.iter().all(|(_, ok, _)| *ok);
    let text: String = out.iter().map(|(n, ok, why)| format!("{} {n}{why}\n", if *ok { "PASS" } else { "FAIL" })).collect();
    if let Some(pre) = rt::by_id("kit-check") {
        pre.set_text_content(Some(&text));
    }
    if let Some(body) = rt::document().and_then(|d| d.body()) {
        let _ = body.set_attribute("data-kit-check", if pass { "pass" } else { "fail" });
    }
    // the offline banner sits over the page's top: take it away again, keep the rest for the screenshot (the
    // page stays in offline mode, so the Connection banner button shows nothing from here on)
    if let Some(b) = rt::by_id("offline-banner") {
        b.remove();
    }
}

fn main() {
    kit_leptos::api::set_backend(Fake);
    toasts::mount();
    details::init();
    dropzone::init();
    notice::start(NoticeSource::encore_downloads(), true);
    notice::mount();
    if let Some(el) = rt::by_id("demo").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
        leptos::mount::mount_to(el, Demo).forget();
    }
    // the notice's first answer comes after the fake backend's delay
    rt::after(600, check);
}
