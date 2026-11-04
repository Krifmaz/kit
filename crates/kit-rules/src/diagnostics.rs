//! Diagnostics: the page that runs every connection, folder and tool check and says what is wrong; its words,
//! how its checks are grouped and the plain-text report Copy report puts on the clipboard.
//!
//! From Encore's `web-rs/src/settings/diagnostics.rs` (`KIND`, `render`, `run`, `check_item`, `copy_report`)
//! and `contracts/diagnostics.md`. The answer is read as JSON (`serde_json::Value`) because servers add fields
//! per check (Encore's Soulseek check carries `soulseek`) and a client must keep what it does not know.

use serde_json::Value;

/// The check groups in page order: `kind` and its heading. From `diagnostics.rs` `KIND`.
pub const KINDS: [(&str, &str); 3] = [("service", "Connections"), ("path", "Folders & permissions"), ("tool", "Tools in the container")];
/// The page's heading. From `diagnostics.rs` `render`.
pub const TITLE: &str = "Diagnostics";
/// The button that copies the report. From `diagnostics.rs` `render`.
pub const COPY_REPORT: &str = "Copy report";
/// The button that runs the checks. From `diagnostics.rs` `render`.
pub const RUN_CHECKS: &str = "Run checks";
/// The line while the checks run. From `diagnostics.rs` `run`.
pub const CHECKING: &str = "Checking…";
/// The summary badge when every check passed. From `diagnostics.rs` `run`.
pub const ALL_GOOD: &str = "All good";
/// The fold with a check's raw answer. From `diagnostics.rs` `check_item`.
pub const RAW_RESPONSE: &str = "Raw response";
/// The settings table's fold. From `diagnostics.rs` `run`.
pub const SOURCES_TITLE: &str = "Where settings come from";
/// The settings table's column headings. From `diagnostics.rs` `run`.
pub const SOURCES_COLUMNS: [&str; 3] = ["Setting", "Source", "Value"];

/// The line under the heading for `app`. From `diagnostics.rs` `render`.
pub fn intro(app: &str) -> String {
    format!("Runs every connection, folder and tool check from inside the {app} container and shows the raw answers. Keys are hidden.")
}

/// The summary badge's words: [`ALL_GOOD`], "1 problem", "3 problems". From `diagnostics.rs` `run`.
pub fn problems(bad: usize) -> String {
    match bad {
        0 => ALL_GOOD.to_string(),
        1 => "1 problem".to_string(),
        n => format!("{n} problems"),
    }
}

/// The summary line ("Encore 0.36.4 · running as uid 1000 · 11/4/2026, 10:00:00 AM"); `time` is formatted by the
/// client. From `diagnostics.rs` `run`.
pub fn summary_line(app: &str, version: &str, uid: &str, time: &str) -> String {
    format!("{app} {version} · running as uid {uid} · {time}")
}

/// The settings fold's second line. From `diagnostics.rs` `run`.
pub fn sources_line(app: &str, count: usize) -> String {
    format!("Environment variables always win over values saved in {app} · {count} settings")
}

/// The badge of a setting saved in the app (not set by an environment variable). From `diagnostics.rs` `run`.
pub fn saved_in(app: &str) -> String {
    format!("saved in {app}")
}

/// `${obj[key]}` as JavaScript prints it: a missing key is `undefined`, `null` is `null`, a string as is.
/// From `diagnostics.rs` `js_str`.
pub fn js_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(x) => js_string(x),
    }
}

/// `String(v)` for a JSON value (a string without quotes, `[object Object]` for an object).
pub fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            _ => n.to_string(),
        },
        Value::Array(a) => a.iter().map(|x| if x.is_null() { String::new() } else { js_string(x) }).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// JavaScript's truthiness of `v[key]` (missing, null, false, 0, "" are false).
pub fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `String(obj[key] || '')`. From `diagnostics.rs` `or_empty`.
pub fn or_empty(v: &Value, key: &str) -> String {
    if truthy(v, key) {
        js_string(&v[key])
    } else {
        String::new()
    }
}

/// How many checks failed (`ok` not truthy). From `diagnostics.rs` `run`.
pub fn bad_count(checks: &[Value]) -> usize {
    checks.iter().filter(|c| !truthy(c, "ok")).count()
}

/// The checks of each group that has any, in [`KINDS`] order: (heading, checks). From `diagnostics.rs` `run`.
pub fn groups(checks: &[Value]) -> Vec<(&'static str, Vec<&Value>)> {
    KINDS
        .iter()
        .map(|(k, title)| (*title, checks.iter().filter(|c| c.get("kind").and_then(Value::as_str) == Some(k)).collect::<Vec<_>>()))
        .filter(|(_, g)| !g.is_empty())
        .collect()
}

/// A check's `status` as an HTTP code ("HTTP 502" on the page), when it is one: Encore sends the code of the
/// answer as a number. The kit's `ok` / `warn` / `fail` (`contracts/diagnostics.md`) is not a code.
pub fn http_status(c: &Value) -> Option<String> {
    match c.get("status") {
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => Some(n.to_string()),
        Some(Value::String(s)) if !s.is_empty() && s.chars().all(|ch| ch.is_ascii_digit()) => Some(s.clone()),
        _ => None,
    }
}

/// A check's level for its dot: `ok`, `warn` or `err`. Encore's checks say only `ok`; a kit server may send
/// `status: "warn"` for a check that passed with a warning (`contracts/diagnostics.md`).
pub fn level(c: &Value) -> &'static str {
    match c.get("status").and_then(Value::as_str) {
        Some("warn") => "warn",
        Some("fail") => "err",
        _ if truthy(c, "ok") => "ok",
        _ => "err",
    }
}

/// The whole text a check's request line opens (its name, request, detail and raw answer, the empty ones left
/// out). From `diagnostics.rs` `check_item`.
pub fn whole_text(c: &Value, detail: &str) -> String {
    [js_string(c.get("name").unwrap_or(&Value::Null)), or_empty(c, "request"), detail.to_string(), or_empty(c, "body")]
        .into_iter()
        .filter(|x| !x.is_empty() && x != "null")
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first `n` UTF-16 units of `s`, as JavaScript's `s.slice(0, n)` cuts it (a cut surrogate pair is dropped).
pub fn js_slice(s: &str, n: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().take(n).collect();
    String::from_utf16_lossy(&units).trim_end_matches('\u{FFFD}').to_string()
}

/// The plain-text report Copy report puts on the clipboard, for `app`'s answer `last`. `extra` are the app's own
/// sections (Encore's download folders and Server block), each a list of lines, put before the settings.
///
/// From `diagnostics.rs` `copy_report`.
pub fn report_text(app: &str, last: &Value, extra: &[Vec<String>]) -> String {
    let mut lines = vec![format!("{app} {} diagnostics (uid {})", js_str(last, "version"), js_str(last, "uid"))];
    let empty = vec![];
    for c in last.get("checks").and_then(Value::as_array).unwrap_or(&empty) {
        let ok = truthy(c, "ok");
        let status = if truthy(c, "status") { format!(" → HTTP {}", js_str(c, "status")) } else { String::new() };
        let ms = if c.get("ms").is_some_and(|m| !m.is_null()) { format!(" ({} ms)", js_str(c, "ms")) } else { String::new() };
        let body = if truthy(c, "body") && !ok { format!("\n     body: {}", js_slice(&js_string(&c["body"]), 300)) } else { String::new() };
        lines.push(format!(
            "{} {} – {}{status}{ms}\n     {}{body}",
            if ok { "OK  " } else { "FAIL" },
            js_str(c, "name"),
            or_empty(c, "request"),
            or_empty(c, "detail")
        ));
    }
    for section in extra {
        lines.push(String::new());
        lines.extend(section.iter().cloned());
    }
    lines.push(String::new());
    lines.push("Settings:".into());
    for s in last.get("settings").and_then(Value::as_array).unwrap_or(&empty) {
        let source =
            if s.get("source").and_then(Value::as_str) == Some("env") { format!("env {}", js_str(s, "env")) } else { "saved".to_string() };
        lines.push(format!("  {} = {}  [{source}]", js_str(s, "key"), js_str(s, "value")));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_words_take_the_apps_name() {
        assert_eq!(
            intro("Encore"),
            "Runs every connection, folder and tool check from inside the Encore container and shows the raw answers. Keys are hidden."
        );
        assert_eq!((problems(0), problems(1), problems(3)), ("All good".into(), "1 problem".into(), "3 problems".into()));
        assert_eq!(summary_line("Encore", "0.36.4", "1000", "today"), "Encore 0.36.4 · running as uid 1000 · today");
        assert_eq!(sources_line("Encore", 12), "Environment variables always win over values saved in Encore · 12 settings");
        assert_eq!(saved_in("Encore"), "saved in Encore");
    }

    #[test]
    fn checks_group_by_kind_in_page_order() {
        let checks = vec![
            json!({ "name": "ffmpeg", "kind": "tool", "ok": true }),
            json!({ "name": "MusicBrainz", "kind": "service", "ok": false }),
            json!({ "name": "Music", "kind": "path", "ok": true }),
            json!({ "name": "odd", "kind": "other", "ok": true }),
        ];
        let g = groups(&checks);
        assert_eq!(
            g.iter().map(|(t, c)| (*t, c.len())).collect::<Vec<_>>(),
            [("Connections", 1), ("Folders & permissions", 1), ("Tools in the container", 1)]
        );
        assert_eq!(bad_count(&checks), 1);
    }

    #[test]
    fn status_is_a_code_or_a_level() {
        assert_eq!(http_status(&json!({ "status": 502 })).as_deref(), Some("502"));
        assert_eq!(http_status(&json!({ "status": "warn" })), None);
        assert_eq!(http_status(&json!({})), None);
        assert_eq!(level(&json!({ "ok": true })), "ok");
        assert_eq!(level(&json!({ "ok": false, "status": 500 })), "err");
        assert_eq!(level(&json!({ "ok": true, "status": "warn" })), "warn");
    }

    #[test]
    fn the_report_reads_like_encores() {
        let last = json!({
            "version": "0.36.4", "uid": 1000,
            "checks": [
                { "name": "MusicBrainz", "kind": "service", "request": "GET https://musicbrainz.org/ws/2", "ok": true, "detail": "200 OK", "status": 200, "ms": 180 },
                { "name": "Prowlarr", "kind": "service", "ok": false, "detail": "refused", "body": "é".repeat(400) }
            ],
            "settings": [{ "key": "library_path", "value": "/music", "source": "env", "env": "ENCORE_LIBRARY" }, { "key": "app_name", "value": null, "source": "ui" }]
        });
        let r = report_text("Encore", &last, &[vec!["Server:".into(), "  CPUs: 4".into()]]);
        let want = format!(
            "Encore 0.36.4 diagnostics (uid 1000)\nOK   MusicBrainz – GET https://musicbrainz.org/ws/2 → HTTP 200 (180 ms)\n     200 OK\nFAIL Prowlarr – \n     refused\n     body: {}\n\nServer:\n  CPUs: 4\n\nSettings:\n  library_path = /music  [env ENCORE_LIBRARY]\n  app_name = null  [saved]",
            "é".repeat(300)
        );
        assert_eq!(r, want);
        assert_eq!(js_str(&json!({}), "x"), "undefined");
        assert_eq!(js_slice("a😀b", 2), "a");
        assert_eq!(
            (js_string(&json!(2.0)), js_string(&json!(0.5)), js_string(&json!(384000))),
            ("2".into(), "0.5".into(), "384000".into())
        );
    }
}
