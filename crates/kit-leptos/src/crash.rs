//! Crash logs for the web app: a panic, an uncaught error or an unhandled promise rejection is written down in
//! `localStorage` and sent to the server's crash reports (`POST <api>/crash-reports`, kit-server-bugreports),
//! now if it can be and on the next start if not. Admins read them on the server. A panic also puts a banner over
//! the page ([`CrashWords`]: what stopped it, and Reload), since the app does not run on after one.
//!
//! ```rust,ignore
//! kit_leptos::crash::install(); // once, after config::set
//! ```

use crate::api;
use serde_json::{json, Value};
use std::cell::Cell;
use wasm_bindgen::JsCast;

/// The most crash logs kept while the server can't be reached.
pub const MAX_PENDING: usize = 10;
/// The longest log text sent.
pub const MAX_TEXT: usize = 16_000;

thread_local! {
    static REPORTING: Cell<bool> = const { Cell::new(false) };
    static FLUSHING: Cell<bool> = const { Cell::new(false) };
}

fn key(name: &str) -> String {
    format!("{}.crash.{name}", crate::config::get().app.id)
}

fn local() -> Option<web_sys::Storage> {
    crate::rt::window()?.local_storage().ok().flatten()
}

/// A short hash of `s`, so the same crash is counted once (FNV-1a).
pub fn fingerprint(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// `text` cut to [`MAX_TEXT`] bytes on a character boundary.
pub fn clip(text: &str) -> &str {
    if text.len() <= MAX_TEXT {
        return text;
    }
    let mut end = MAX_TEXT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Where a crash happened.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Device {
    /// The app's version.
    pub version: String,
    /// The browser (its user agent).
    pub device: String,
    /// This browser's id, kept in `localStorage`.
    pub device_id: String,
    /// The platform.
    pub os: String,
}

/// The body of one crash log, in the server's shape. `time` is epoch seconds; `key` is the same for the same
/// crash in the same version, so the server keeps it once.
pub fn body(kind: &str, summary: &str, text: &str, d: &Device, time: f64) -> Value {
    let Device { version, device, device_id, os } = d;
    let summary = summary.lines().next().unwrap_or("").trim();
    let text = clip(text);
    let first_frame = text.lines().map(str::trim).find(|l| !l.is_empty() && *l != summary).unwrap_or("");
    json!({
        "kind": kind,
        "device": device,
        "device_id": device_id,
        "app_version": version,
        "os": os,
        "time": time,
        "summary": summary,
        "text": text,
        "key": fingerprint(&format!("{version}\n{summary}\n{first_frame}")),
    })
}

/// The logs not sent yet, oldest first.
pub fn pending() -> Vec<Value> {
    local().and_then(|s| s.get_item(&key("pending")).ok().flatten()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn set_pending(list: &[Value]) {
    if let Some(s) = local() {
        let _ = if list.is_empty() {
            s.remove_item(&key("pending"))
        } else {
            s.set_item(&key("pending"), &Value::from(list.to_vec()).to_string())
        };
    }
}

/// `list` with `item` added at the end, the oldest dropped past [`MAX_PENDING`] and a log with the same key
/// only once.
pub fn queued(mut list: Vec<Value>, item: Value) -> Vec<Value> {
    let k = item.get("key").cloned();
    list.retain(|v| v.get("key") != k.as_ref());
    list.push(item);
    let over = list.len().saturating_sub(MAX_PENDING);
    list.drain(..over);
    list
}

fn device_id() -> String {
    let Some(s) = local() else { return String::new() };
    if let Ok(Some(id)) = s.get_item(&key("device")) {
        return id;
    }
    let id = fingerprint(&format!("{}{}", js_sys::Math::random(), crate::rt::now_ms()));
    let _ = s.set_item(&key("device"), &id);
    id
}

/// The words of the banner a panic puts over the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrashWords {
    /// The banner's sentence: `{what}` is the panic's message, `{where}` " (file:line:col)" or nothing. Empty: no
    /// banner.
    pub stopped: &'static str,
    /// The banner's button, which reloads the page.
    pub reload: &'static str,
}

impl CrashWords {
    /// The kit's words.
    pub const DEFAULT: CrashWords =
        CrashWords { stopped: "This page hit an error and stopped: {what}{where}. The report was sent to the admin.", reload: "Reload" };

    /// The banner's sentence for a panic's message and place ("" when the app turned the banner off).
    pub fn stopped(&self, what: &str, place: &str) -> String {
        let place = if place.is_empty() { String::new() } else { format!(" ({place})") };
        self.stopped.replace("{what}", what).replace("{where}", &place)
    }
}

impl Default for CrashWords {
    fn default() -> Self {
        CrashWords::DEFAULT
    }
}

/// A panic's message and place, from the text of its `PanicHookInfo` ("panicked at src/a.rs:3:9:\nthe message"):
/// the first line that is not the "panicked at" line (or the whole text), cut to 200 characters, and
/// "src/a.rs:3:9" (or "").
pub fn panic_parts(text: &str) -> (String, String) {
    let what = text.lines().find(|l| !l.trim().is_empty() && !l.contains("panicked at")).unwrap_or(text).trim();
    let place = text.lines().find_map(|l| l.split_once("panicked at ").map(|(_, p)| p.trim().trim_end_matches(':').to_string()));
    (what.chars().take(200).collect(), place.unwrap_or_default())
}

/// The body of a crash log, written down to send (None while a report is already being written).
fn record(kind: &str, summary: &str, text: &str) -> Option<Value> {
    // A failure while reporting is not reported again.
    if REPORTING.with(|r| r.replace(true)) {
        return None;
    }
    let nav = crate::rt::window().map(|w| w.navigator());
    let device = nav.as_ref().and_then(|n| n.user_agent().ok()).unwrap_or_default();
    let os = nav.as_ref().and_then(|n| n.platform().ok()).unwrap_or_default();
    let version = crate::config::get().app.version.clone();
    let d = Device { version, device, device_id: device_id(), os };
    let b = body(kind, summary, text, &d, js_sys::Date::now() / 1000.0);
    set_pending(&queued(pending(), b.clone()));
    REPORTING.with(|r| r.set(false));
    Some(b)
}

/// Writes down a crash log and sends it.
pub fn report(kind: &str, summary: &str, text: &str) {
    if record(kind, summary, text).is_some() {
        crate::rt::spawn(flush());
    }
}

/// Posts a crash log at once with a plain `fetch` (no future to poll: after a panic the app's tasks may never
/// run again). It stays in the pending list too; the server keeps the same key once.
fn send_now(b: &Value) {
    let Some(w) = crate::rt::window() else { return };
    let c = crate::config::get();
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    init.set_credentials(web_sys::RequestCredentials::SameOrigin);
    // keepalive: the post goes out even when the reader reloads at once
    let _ = js_sys::Reflect::set(&init, &"keepalive".into(), &true.into());
    init.set_body(&wasm_bindgen::JsValue::from_str(&b.to_string()));
    if let Ok(h) = web_sys::Headers::new() {
        let _ = h.set("Content-Type", "application/json");
        let _ = h.set("X-Requested-With", &c.app.id.to_ascii_lowercase());
        init.set_headers(&h);
    }
    let p = w.fetch_with_str_and_init(&format!("{}{}", c.paths.api, c.paths.crash_reports), &init);
    // a failed send waits in the pending list; its rejection is no crash of its own (plain JS, no call back in)
    if let Ok(catch) = js_sys::Reflect::get(&p, &"catch".into()).and_then(|f| f.dyn_into::<js_sys::Function>()) {
        let _ = catch.call1(&p, &js_sys::Function::new_no_args(""));
    }
}

/// The banner over the page after a panic: what stopped it, and Reload. Plain DOM with its own inline styles and
/// a plain-JS button, since the app may not run again.
fn show_banner(text: &str, reload: &str) {
    let Some(doc) = crate::rt::document() else { return };
    if text.is_empty() || doc.get_element_by_id("kit-panic-banner").is_some() {
        return;
    }
    let (Ok(b), Ok(t), Ok(r)) = (doc.create_element("div"), doc.create_element("span"), doc.create_element("button")) else { return };
    b.set_id("kit-panic-banner");
    let _ = b.set_attribute("role", "alert");
    let _ = b.set_attribute(
        "style",
        "position:fixed;left:0;right:0;top:0;z-index:99999;padding:10px 14px;background:#7a1f1f;color:#fff;font:14px/1.4 system-ui,sans-serif;display:flex;gap:12px;align-items:center;flex-wrap:wrap",
    );
    t.set_text_content(Some(text));
    r.set_text_content(Some(reload));
    let _ =
        r.set_attribute("style", "padding:6px 12px;border:0;border-radius:8px;background:#fff;color:#7a1f1f;font:inherit;cursor:pointer");
    let _ = r.set_attribute("onclick", "location.reload()");
    let _ = b.append_child(&t);
    let _ = b.append_child(&r);
    let parent: Option<web_sys::Element> = doc.body().map(Into::into).or_else(|| doc.document_element());
    if let Some(p) = parent {
        let _ = p.append_child(&b);
    }
}

/// Sends the logs not sent yet; one the server can't take is dropped, one that didn't get there waits.
pub async fn flush() {
    // One at a time; a log written meanwhile goes with this one or the next start.
    if FLUSHING.with(|f| f.replace(true)) {
        return;
    }
    send_pending().await;
    FLUSHING.with(|f| f.set(false));
}

async fn send_pending() {
    let path = crate::config::get().paths.crash_reports.clone();
    for item in pending() {
        let r = api::post(&path, item.clone()).await;
        let reached = match &r {
            Ok(_) => true,
            // Not signed in: it waits for the next start.
            Err(e) => e.status != 0 && e.status != 401 && e.status < 500,
        };
        if !reached {
            return;
        }
        let k = item.get("key").cloned();
        set_pending(&pending().into_iter().filter(|v| v.get("key") != k.as_ref()).collect::<Vec<_>>());
    }
}

fn js_text(v: &wasm_bindgen::JsValue) -> (String, String) {
    if let Some(e) = v.dyn_ref::<js_sys::Error>() {
        let msg = String::from(e.to_string());
        let stack = js_sys::Reflect::get(e, &"stack".into()).ok().and_then(|s| s.as_string()).unwrap_or_default();
        return (msg.clone(), if stack.is_empty() { msg } else { stack });
    }
    let s = v.as_string().unwrap_or_else(|| format!("{v:?}"));
    (s.clone(), s)
}

/// Catches panics (also written to the console), uncaught errors and unhandled rejections, and sends the logs
/// left from an earlier visit.
pub fn install() {
    std::panic::set_hook(Box::new(move |info| {
        let said = info.to_string();
        web_sys::console::error_1(&said.clone().into());
        let summary = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| said.clone());
        // the JavaScript stack (the wasm frames) and the page, for the admin
        let stack = js_sys::Reflect::get(&js_sys::Error::new(""), &"stack".into()).ok().and_then(|v| v.as_string()).unwrap_or_default();
        let page = crate::rt::window().and_then(|w| w.location().href().ok()).unwrap_or_default();
        let text = format!("{said}\n\nStack:\n{stack}\n\npage: {page}");
        // the panic leaves the app's tasks stopped: send now and say so, without waiting for a future
        if let Some(b) = record("panic", &format!("panic: {summary}"), &text) {
            send_now(&b);
        }
        let w = crate::config::get().words.crash;
        let (what, place) = panic_parts(&said);
        show_banner(&w.stopped(&what, &place), w.reload);
    }));
    crate::rt::on_window("error", |e| {
        let Some(e) = e.dyn_ref::<web_sys::ErrorEvent>() else { return };
        let (msg, stack) = js_text(&e.error());
        let msg = if msg.is_empty() || msg == "undefined" { e.message() } else { msg };
        let place = format!("at {}:{}:{}", e.filename(), e.lineno(), e.colno());
        report("error", &msg, &format!("{stack}\n{place}"));
    });
    crate::rt::on_window("unhandledrejection", |e| {
        let Some(e) = e.dyn_ref::<web_sys::PromiseRejectionEvent>() else { return };
        let (msg, stack) = js_text(&e.reason());
        report("rejection", &format!("Unhandled rejection: {msg}"), &stack);
    });
    crate::rt::spawn(flush());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_the_servers_shape() {
        let d = |v: &str, id: &str| Device { version: v.into(), device: "UA".into(), device_id: id.into(), os: "Linux".into() };
        let b = body("error", "TypeError: x is undefined\nmore", "TypeError: x is undefined\n  at f (app.js:1:2)", &d("1.2.0", "d1"), 5.0);
        assert_eq!(b["summary"], "TypeError: x is undefined");
        assert_eq!((b["kind"].as_str(), b["app_version"].as_str(), b["time"].as_f64()), (Some("error"), Some("1.2.0"), Some(5.0)));
        let again = body("error", "TypeError: x is undefined", "TypeError: x is undefined\n  at f (app.js:1:2)", &d("1.2.0", "d2"), 9.0);
        assert_eq!(b["key"], again["key"], "the same crash, the same key");
        let other = body("error", "TypeError: x is undefined", "", &d("1.3.0", "d1"), 5.0);
        assert_ne!(b["key"], other["key"], "a new version counts again");
    }

    #[test]
    fn a_panic_says_its_message_and_place() {
        let (what, place) = panic_parts("panicked at src/pages/library.rs:120:9:\nindex out of bounds: the len is 0 but the index is 3");
        assert_eq!((what.as_str(), place.as_str()), ("index out of bounds: the len is 0 but the index is 3", "src/pages/library.rs:120:9"));
        assert_eq!(panic_parts("just words"), ("just words".to_string(), String::new()));
        assert_eq!(panic_parts(&"x".repeat(300)).0.len(), 200);
        let w = CrashWords::DEFAULT;
        assert_eq!(w.stopped(&what, &place), "This page hit an error and stopped: index out of bounds: the len is 0 but the index is 3 (src/pages/library.rs:120:9). The report was sent to the admin.");
        assert_eq!(w.stopped("boom", ""), "This page hit an error and stopped: boom. The report was sent to the admin.");
    }

    #[test]
    fn queue_keeps_one_of_each_and_the_newest() {
        let item = |k: usize| json!({"key": k.to_string()});
        let mut l = Vec::new();
        for k in 0..12 {
            l = queued(l, item(k));
        }
        assert_eq!(l.len(), MAX_PENDING);
        assert_eq!(l[0]["key"], "2");
        let l = queued(l, item(5));
        assert_eq!((l.len(), l.last().unwrap()["key"].as_str()), (MAX_PENDING, Some("5")));
    }

    #[test]
    fn clips_on_a_char_boundary() {
        let t = "é".repeat(MAX_TEXT);
        assert!(clip(&t).len() <= MAX_TEXT && clip(&t).is_char_boundary(clip(&t).len()));
        assert_eq!(clip("short"), "short");
    }
}
