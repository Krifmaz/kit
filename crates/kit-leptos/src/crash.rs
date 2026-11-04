//! Crash logs for the web app: a panic, an uncaught error or an unhandled promise rejection is written down in
//! `localStorage` and sent to the server's crash reports (`POST <api>/crash-reports`, kit-server-bugreports),
//! now if it can be and on the next start if not. Admins read them on the server.
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

/// Writes down a crash log and sends it.
pub fn report(kind: &str, summary: &str, text: &str) {
    // A failure while reporting is not reported again.
    if REPORTING.with(|r| r.replace(true)) {
        return;
    }
    let nav = crate::rt::window().map(|w| w.navigator());
    let device = nav.as_ref().and_then(|n| n.user_agent().ok()).unwrap_or_default();
    let os = nav.as_ref().and_then(|n| n.platform().ok()).unwrap_or_default();
    let version = crate::config::get().app.version.clone();
    let d = Device { version, device, device_id: device_id(), os };
    let b = body(kind, summary, text, &d, js_sys::Date::now() / 1000.0);
    set_pending(&queued(pending(), b));
    REPORTING.with(|r| r.set(false));
    crate::rt::spawn(flush());
}

/// Sends the logs not sent yet; one the server can't take is dropped, one that didn't get there waits.
pub async fn flush() {
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
        let text = info.to_string();
        web_sys::console::error_1(&text.clone().into());
        let summary = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| (*s).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| text.clone());
        report("panic", &format!("panic: {summary}"), &text);
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
