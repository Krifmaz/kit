//! Diagnostics: the page that runs every check an app's server knows and says what is wrong; its words, how its
//! checks are grouped and the plain-text report Copy report puts on the clipboard.
//!
//! The checks are the app's (`contracts/diagnostics.md`): each says its `kind`, and the page draws one group
//! per kind, the kit's three ([`KINDS`]: connections, folders, tools) first and any other kind after them, in
//! the order the server sent them ([`groups`]). The answer is read as JSON (`serde_json::Value`) because servers
//! add fields per check and a client must keep what it does not know. The words are [`DiagWords`].

use serde_json::Value;

/// The kit's check groups in page order: `kind` and its heading. Other kinds follow them ([`groups`]).
pub const KINDS: [(&str, &str); 3] = [("service", "Connections"), ("path", "Folders & permissions"), ("tool", "Tools in the container")];
/// The page's heading.
pub const TITLE: &str = "Diagnostics";
/// The button that copies the report.
pub const COPY_REPORT: &str = "Copy report";
/// The button that runs the checks.
pub const RUN_CHECKS: &str = "Run checks";
/// The line while the checks run.
pub const CHECKING: &str = "Checking…";
/// The summary badge when every check passed.
pub const ALL_GOOD: &str = "All good";
/// The fold with a check's raw answer.
pub const RAW_RESPONSE: &str = "Raw response";
/// The settings table's fold.
pub const SOURCES_TITLE: &str = "Where settings come from";
/// The settings table's column headings.
pub const SOURCES_COLUMNS: [&str; 3] = ["Setting", "Source", "Value"];

/// The line under the heading, `{app}` filled in ([`intro`]).
pub const INTRO: &str = "Runs every check from inside the {app} server and shows the raw answers. Keys are hidden.";

/// The Diagnostics page's words. [`DiagWords::DEFAULT`] holds the kit's; an app passes its own to change any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiagWords {
    /// The page's heading ([`TITLE`]).
    pub title: &'static str,
    /// The line under it, with `{app}` ([`INTRO`]).
    pub intro: &'static str,
    /// The button that copies the report ([`COPY_REPORT`]).
    pub copy_report: &'static str,
    /// The button that runs the checks ([`RUN_CHECKS`]).
    pub run_checks: &'static str,
    /// The line while the checks run ([`CHECKING`]).
    pub checking: &'static str,
    /// The summary badge when every check passed ([`ALL_GOOD`]).
    pub all_good: &'static str,
    /// The fold with a check's raw answer ([`RAW_RESPONSE`]).
    pub raw_response: &'static str,
    /// The settings table's fold ([`SOURCES_TITLE`]).
    pub sources_title: &'static str,
    /// The settings table's column headings ([`SOURCES_COLUMNS`]).
    pub sources_columns: [&'static str; 3],
    /// The check groups' headings by kind ([`KINDS`]).
    pub kinds: [(&'static str, &'static str); 3],
}

impl DiagWords {
    /// The kit's words.
    pub const DEFAULT: DiagWords = DiagWords {
        title: TITLE,
        intro: INTRO,
        copy_report: COPY_REPORT,
        run_checks: RUN_CHECKS,
        checking: CHECKING,
        all_good: ALL_GOOD,
        raw_response: RAW_RESPONSE,
        sources_title: SOURCES_TITLE,
        sources_columns: SOURCES_COLUMNS,
        kinds: KINDS,
    };

    /// The line under the heading for `app`.
    pub fn intro(&self, app: &str) -> String {
        crate::app::fill(self.intro, &[("app", app)])
    }
}

impl Default for DiagWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The line under the heading for `app`.
pub fn intro(app: &str) -> String {
    DiagWords::DEFAULT.intro(app)
}

/// The summary badge's words: [`ALL_GOOD`], "1 problem", "3 problems".
pub fn problems(bad: usize) -> String {
    match bad {
        0 => ALL_GOOD.to_string(),
        1 => "1 problem".to_string(),
        n => format!("{n} problems"),
    }
}

/// The summary line ("Kit Demo 1.4.0 · running as uid 1000 · 11/4/2026, 10:00:00 AM"); `time` is formatted by the
/// client.
pub fn summary_line(app: &str, version: &str, uid: &str, time: &str) -> String {
    format!("{app} {version} · running as uid {uid} · {time}")
}

/// The settings fold's second line.
pub fn sources_line(app: &str, count: usize) -> String {
    format!("Environment variables always win over values saved in {app} · {count} settings")
}

/// The badge of a setting saved in the app (not set by an environment variable).
pub fn saved_in(app: &str) -> String {
    format!("saved in {app}")
}

/// `${obj[key]}` as JavaScript prints it: a missing key is `undefined`, `null` is `null`, a string as is.
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

/// `String(obj[key] || '')`.
pub fn or_empty(v: &Value, key: &str) -> String {
    if truthy(v, key) {
        js_string(&v[key])
    } else {
        String::new()
    }
}

/// How many checks failed (`ok` not truthy).
pub fn bad_count(checks: &[Value]) -> usize {
    checks.iter().filter(|c| !truthy(c, "ok")).count()
}

/// The checks of each group that has any: the kit's kinds ([`KINDS`]) first, in that order, then every other
/// kind in the order it first appears, headed by the kind itself with a capital (`queue` → "Queue"); a check
/// without a kind goes under "Other". Returns (heading, checks).
pub fn groups(checks: &[Value]) -> Vec<(String, Vec<&Value>)> {
    groups_with(checks, &KINDS)
}

/// [`groups`] with the app's own headings for the kinds it knows (`kinds`, in page order).
pub fn groups_with<'a>(checks: &'a [Value], kinds: &[(&str, &str)]) -> Vec<(String, Vec<&'a Value>)> {
    let kind_of = |c: &Value| c.get("kind").and_then(Value::as_str).filter(|k| !k.is_empty()).unwrap_or("other").to_string();
    let mut out: Vec<(String, Vec<&Value>)> =
        kinds.iter().map(|(k, title)| (title.to_string(), checks.iter().filter(|c| kind_of(c) == *k).collect::<Vec<_>>())).collect();
    let mut others: Vec<(String, Vec<&Value>)> = Vec::new();
    for c in checks {
        let k = kind_of(c);
        if kinds.iter().any(|(known, _)| *known == k) {
            continue;
        }
        match others.iter_mut().find(|(kk, _)| *kk == k) {
            Some((_, g)) => g.push(c),
            None => others.push((k, vec![c])),
        }
    }
    out.extend(others.into_iter().map(|(k, g)| (heading_of(&k), g)));
    out.retain(|(_, g)| !g.is_empty());
    out
}

/// A kind as a heading: its first letter a capital, `_` and `-` as spaces (`disk_space` → "Disk space").
pub fn heading_of(kind: &str) -> String {
    let words = kind.replace(['_', '-'], " ");
    let mut c = words.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// A check's `status` as an HTTP code ("HTTP 502" on the page), when it is one: a check of a service may send
/// the code of the answer as a number. The kit's `ok` / `warn` / `fail` (`contracts/diagnostics.md`) is not a
/// code.
pub fn http_status(c: &Value) -> Option<String> {
    match c.get("status") {
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => Some(n.to_string()),
        Some(Value::String(s)) if !s.is_empty() && s.chars().all(|ch| ch.is_ascii_digit()) => Some(s.clone()),
        _ => None,
    }
}

/// A check's level for its dot: `ok`, `warn` or `err`, from `status` (`warn`, `fail`) when it says so, else
/// from `ok` (`contracts/diagnostics.md`).
pub fn level(c: &Value) -> &'static str {
    match c.get("status").and_then(Value::as_str) {
        Some("warn") => "warn",
        Some("fail") => "err",
        _ if truthy(c, "ok") => "ok",
        _ => "err",
    }
}

/// The whole text a check's request line opens (its name, request, detail and raw answer, the empty ones left
/// out).
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
/// sections (a server block, the app's folders), each a list of lines, put before the settings.
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
        assert_eq!(intro("Kit Demo"), "Runs every check from inside the Kit Demo server and shows the raw answers. Keys are hidden.");
        assert_eq!((problems(0), problems(1), problems(3)), ("All good".into(), "1 problem".into(), "3 problems".into()));
        assert_eq!(summary_line("Kit Demo", "1.4.0", "1000", "today"), "Kit Demo 1.4.0 · running as uid 1000 · today");
        assert_eq!(sources_line("Kit Demo", 12), "Environment variables always win over values saved in Kit Demo · 12 settings");
        assert_eq!(saved_in("Kit Demo"), "saved in Kit Demo");
        let mine = DiagWords { intro: "Prüft {app}", ..DiagWords::DEFAULT };
        assert_eq!(mine.intro("Filebrary"), "Prüft Filebrary");
        assert_eq!(DiagWords::default(), DiagWords::DEFAULT);
    }

    #[test]
    fn checks_group_by_kind_the_kits_first_then_the_apps() {
        let checks = vec![
            json!({ "name": "ffmpeg", "kind": "tool", "ok": true }),
            json!({ "name": "Mail server", "kind": "service", "ok": false }),
            json!({ "name": "Mail queue", "kind": "queue", "ok": true }),
            json!({ "name": "Data folder", "kind": "path", "ok": true }),
            json!({ "name": "Free space", "kind": "disk_space", "ok": true }),
            json!({ "name": "Job queue", "kind": "queue", "ok": true }),
            json!({ "name": "odd" }),
        ];
        let g = groups(&checks);
        assert_eq!(
            g.iter().map(|(t, c)| (t.as_str(), c.len())).collect::<Vec<_>>(),
            [
                ("Connections", 1),
                ("Folders & permissions", 1),
                ("Tools in the container", 1),
                ("Queue", 2),
                ("Disk space", 1),
                ("Other", 1)
            ]
        );
        assert_eq!(bad_count(&checks), 2);
        let own = groups_with(&checks, &[("queue", "Queues")]);
        assert_eq!(own[0].0, "Queues");
        assert_eq!(own.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["Queues", "Tool", "Service", "Path", "Disk space", "Other"]);
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
    fn the_report_lists_every_check_and_setting() {
        let last = json!({
            "version": "1.4.0", "uid": 1000,
            "checks": [
                { "name": "Mail server", "kind": "service", "request": "GET https://mail.example.com/health", "ok": true, "detail": "200 OK", "status": 200, "ms": 180 },
                { "name": "Search index", "kind": "service", "ok": false, "detail": "refused", "body": "é".repeat(400) }
            ],
            "settings": [{ "key": "data_path", "value": "/data", "source": "env", "env": "APP_DATA_PATH" }, { "key": "app_name", "value": null, "source": "ui" }]
        });
        let r = report_text("Kit Demo", &last, &[vec!["Server:".into(), "  CPUs: 4".into()]]);
        let want = format!(
            "Kit Demo 1.4.0 diagnostics (uid 1000)\nOK   Mail server – GET https://mail.example.com/health → HTTP 200 (180 ms)\n     200 OK\nFAIL Search index – \n     refused\n     body: {}\n\nServer:\n  CPUs: 4\n\nSettings:\n  data_path = /data  [env APP_DATA_PATH]\n  app_name = null  [saved]",
            "é".repeat(300)
        );
        assert_eq!(r, want);
        assert_eq!(js_str(&json!({}), "x"), "undefined");
        assert_eq!(js_slice("a😀b", 2), "a");
        assert_eq!(
            (js_string(&json!(2.0)), js_string(&json!(0.5)), js_string(&json!(384000))),
            ("2".into(), "0.5".into(), "384000".into())
        );
        assert_eq!(heading_of(""), "");
    }
}
