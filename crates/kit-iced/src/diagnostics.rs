//! The diagnostics page, as Encore's desktop app draws it (`clients/desktop/src/pages/settings/diagnostics.rs`):
//! every check the server ran (`GET /api/diagnostics`, see `contracts/diagnostics.md`) in groups
//! by kind, each with its status dot, name, HTTP badge, time, request, what it said, how to fix it and
//! its folded raw answer; Copy on a check that failed, the "N problems" / "All good" summary, the
//! copyable whole report and Run checks / Checking….
//!
//! The component holds no network code. [`State`] keeps the last answer and what is unfolded,
//! the `view` functions draw it, and [`State::update`] returns the [`Action`] the app takes: run the
//! checks with its own client (and hand the answer to [`State::checked`]), build and copy the
//! [`report`], or copy one check's [`check_text`]. An app draws its own sections (Encore: Server,
//! Download folders, Where settings come from, Backups, App crashes) between and after the kit's
//! pieces, with [`facts`] and [`fold`] where they fit.

use crate::details::{self, Details};
use crate::settings::{help, link_muted};
use crate::theme::{self, Badge, Btn};
use crate::widgets::{self, badge, btn_icon, faint, small_muted, t, El, Icon};
use iced::widget::{column, container, row, space};
use iced::{Alignment, Color, Length};
use serde_json::Value;
use std::collections::HashSet;

/// The page's title.
pub const TITLE: &str = "Diagnostics";
/// The Copy report button.
pub const COPY_REPORT: &str = "Copy report";
/// The run button.
pub const RUN: &str = "Run checks";
/// The run button while the checks run.
pub const RUNNING: &str = "Checking…";
/// The summary when every check passed.
pub const ALL_GOOD: &str = "All good";
/// The summary of a check's folded raw answer.
pub const RAW: &str = "Raw response";

/// Encore's check groups: `(kind, heading)`, in the order the page shows them. A check of a kind
/// that is not listed is not drawn (it is still in the report).
pub const KINDS: [(&str, &str); 3] = [("service", "Connections"), ("path", "Folders & permissions"), ("tool", "Tools in the container")];

/// How a check came out: the contract's `status`, or – when a server leaves it out, as Encore's
/// does – `ok` → [`Level::Ok`], otherwise [`Level::Fail`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Passed (green).
    Ok,
    /// Works, but something needs a look (amber).
    Warn,
    /// Failed (red).
    Fail,
}

/// One check as the server sends it.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    /// What was checked, in plain words.
    pub name: String,
    /// `service`, `path` or `tool`.
    pub kind: String,
    /// Passed.
    pub ok: bool,
    /// How it came out (see [`Level`]).
    pub level: Level,
    /// The result in one plain sentence, as sent (the report uses this).
    pub detail: String,
    /// The result as the page and Copy show it: `detail` unless the app rewords it (Encore says its
    /// Soulseek check in the words of the Soulseek card).
    pub shown: String,
    /// What was asked, when it was a request ("" for none).
    pub request: String,
    /// The HTTP status of a request check (Encore's numeric `status`).
    pub http: Option<String>,
    /// How long it took, in milliseconds.
    pub ms: Option<String>,
    /// The raw answer ("" for none).
    pub body: String,
    /// How to fix it, in plain words ("" for none).
    pub fix: String,
    /// The check as sent, for an app's own fields (Encore's `soulseek`).
    pub raw: Value,
}

/// A field as text, as Encore's `fmt::s`: strings, numbers and booleans; "" for anything else.
fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Whether a field is set and not empty, zero or false (Encore's `fmt::truthy`).
fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(_)) => true,
    }
}

impl Check {
    /// A check from one entry of the answer's `checks`. A `status` of `ok`, `warn` or `fail` is the
    /// kit's [`Level`]; any other `status` (Encore's HTTP code) is the HTTP badge.
    pub fn from_value(c: &Value) -> Check {
        let ok = truthy(c, "ok");
        let level = match c["status"].as_str() {
            Some("ok") => Some(Level::Ok),
            Some("warn") => Some(Level::Warn),
            Some("fail") => Some(Level::Fail),
            _ => None,
        };
        let http = (level.is_none() && truthy(c, "status")).then(|| s(c, "status"));
        let detail = s(c, "detail");
        Check {
            name: s(c, "name"),
            kind: s(c, "kind"),
            ok,
            level: level.unwrap_or(if ok { Level::Ok } else { Level::Fail }),
            shown: detail.clone(),
            detail,
            request: s(c, "request"),
            http,
            ms: c.get("ms").is_some_and(|m| !m.is_null()).then(|| s(c, "ms")),
            body: s(c, "body"),
            fix: s(c, "fix"),
            raw: c.clone(),
        }
    }
}

/// The checks of a diagnostics answer (`{checks: [...]}`), in the server's order.
pub fn checks(answer: &Value) -> Vec<Check> {
    answer["checks"].as_array().map(|a| a.iter().map(Check::from_value).collect()).unwrap_or_default()
}

/// One check in full for Copy and its dialog, as the web's dialog has it: the name, the request,
/// what it said, how to fix it and the whole raw answer (the report keeps 300 characters of it).
pub fn check_text(c: &Check) -> String {
    [c.name.as_str(), c.request.as_str(), c.shown.as_str(), c.fix.as_str(), c.body.as_str()]
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The details dialog of one check (the web opens it from the cut-off request line): the check's
/// name over its [`check_text`].
pub fn details<P: Clone>(c: &Check) -> Details<P> {
    Details::new(c.name.clone(), check_text(c), None, None)
}

/// A block of the report after the checks: an empty line, `title:`, then its lines as they are
/// (Encore: Download folders, Server, Settings).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// The block's title, without the colon.
    pub title: String,
    /// Its lines, indented by the app.
    pub lines: Vec<String>,
}

/// One check's lines in the report: `OK  `, `WARN` or `FAIL`, the name, the request, the HTTP status
/// and the time, then what it said, how to fix it and (when it did not pass) the raw answer's first
/// 300 characters.
fn report_check(c: &Check) -> String {
    let mark = match c.level {
        Level::Ok => "OK  ",
        Level::Warn => "WARN",
        Level::Fail => "FAIL",
    };
    let status = c.http.as_ref().map(|h| format!(" → HTTP {h}")).unwrap_or_default();
    let ms = c.ms.as_ref().map(|m| format!(" ({m} ms)")).unwrap_or_default();
    let fix = if c.fix.is_empty() { String::new() } else { format!("\n     {}", c.fix) };
    let body = if !c.body.is_empty() && c.level != Level::Ok {
        format!("\n     body: {}", c.body.chars().take(300).collect::<String>())
    } else {
        String::new()
    };
    format!("{mark} {} – {}{status}{ms}\n     {}{fix}{body}", c.name, c.request, c.detail)
}

/// The plain-text report Copy report puts on the clipboard: `first` (Encore: "Encore 0.27.0
/// diagnostics (uid 1000)"), every check, then the app's [`Section`]s.
pub fn report<'c>(first: &str, checks: impl IntoIterator<Item = &'c Check>, sections: &[Section]) -> String {
    let mut lines = vec![first.to_string()];
    lines.extend(checks.into_iter().map(report_check));
    for s in sections {
        lines.push(String::new());
        lines.push(format!("{}:", s.title));
        lines.extend(s.lines.iter().cloned());
    }
    lines.join("\n")
}

/// `(label, value)` facts as report lines: `  label: value` (Encore's Server block).
pub fn fact_lines<K: AsRef<str>, V: AsRef<str>>(rows: &[(K, V)]) -> Vec<String> {
    rows.iter().map(|(k, v)| format!("  {}: {}", k.as_ref(), v.as_ref())).collect()
}

/// The summary badge: "N problem(s)" in red, or "All good" in green.
pub fn problems(bad: usize) -> (String, Badge) {
    if bad > 0 {
        (format!("{bad} problem{}", if bad > 1 { "s" } else { "" }), Badge::Err)
    } else {
        (ALL_GOOD.to_string(), Badge::Ok)
    }
}

/// The diagnostics page's state: the last answer, its checks, the app's own checks, the run and
/// what is unfolded.
#[derive(Debug, Default)]
pub struct State {
    /// The last answer, `None` before the first one came.
    answer: Option<Value>,
    /// The answer's checks.
    checks: Vec<Check>,
    /// Checks the app adds from its own requests (Encore's VPN for Soulseek), after the server's.
    extra: Vec<Check>,
    /// The checks are running.
    running: bool,
    /// Why the last run failed ("" when it did not).
    error: String,
    /// The [`fold`]s that are open, by key (the checks use `check-N`).
    open: HashSet<String>,
}

/// What the page's buttons send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// Run checks.
    Run,
    /// Copy report.
    CopyReport,
    /// Open or close a [`fold`].
    Toggle(String),
    /// Copy one check (or any text) in full.
    Copy(String),
}

/// What the app does after a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Run the checks (Encore: `GET /diagnostics` and `/system/vpn`) and hand the answer to
    /// [`State::checked`].
    Run,
    /// Build the [`report`] (with the app's sections) from [`State::answer`] and [`State::all`],
    /// copy it and say [`details::COPIED`].
    CopyReport,
    /// Copy this text.
    Copy(String),
}

impl State {
    /// The page opens: everything is forgotten and the checks run.
    pub fn open(&mut self) -> Action {
        *self = State { running: true, ..State::default() };
        Action::Run
    }

    /// Apply a message; `Some(action)` is what the app does next.
    pub fn update(&mut self, msg: Msg) -> Option<Action> {
        match msg {
            Msg::Run => {
                self.running = true;
                self.error.clear();
                Some(Action::Run)
            }
            Msg::CopyReport => self.answer.is_some().then_some(Action::CopyReport),
            Msg::Toggle(k) => {
                if !self.open.remove(&k) {
                    self.open.insert(k);
                }
                None
            }
            Msg::Copy(text) => Some(Action::Copy(text)),
        }
    }

    /// The checks' answer came: the run is over; an error keeps the last answer and is shown above it.
    pub fn checked(&mut self, answer: Result<Value, String>) {
        self.running = false;
        match answer {
            Ok(a) => {
                self.checks = checks(&a);
                self.answer = Some(a);
            }
            Err(e) => self.error = e,
        }
    }

    /// Set the app's own checks (empty to drop them), shown after the server's of the same kind.
    pub fn set_extra(&mut self, extra: Vec<Check>) {
        self.extra = extra;
    }

    /// The last answer, for the app's own sections.
    pub fn answer(&self) -> Option<&Value> {
        self.answer.as_ref()
    }

    /// The server's checks, for an app to reword ([`Check::shown`]).
    pub fn checks_mut(&mut self) -> &mut [Check] {
        &mut self.checks
    }

    /// Every check: the server's, then the app's.
    pub fn all(&self) -> impl Iterator<Item = &Check> {
        self.checks.iter().chain(self.extra.iter())
    }

    /// Whether the checks are running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Why the last run failed ("" when it did not).
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Whether the [`fold`] with this key is open.
    pub fn is_open(&self, key: &str) -> bool {
        self.open.contains(key)
    }
}

/// The colour of a [`Level`]: green, amber or red.
pub fn colour(level: Level) -> Color {
    let p = theme::look();
    match level {
        Level::Ok => p.ok,
        Level::Warn => p.warn,
        Level::Fail => p.err,
    }
}

/// A round 8px status dot in `colour`.
pub fn dot_in<'a, M: 'a>(colour: Color) -> El<'a, M> {
    container(space().width(8).height(8))
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(colour)),
            border: iced::Border { color: Color::TRANSPARENT, width: 0.0, radius: 99.0.into() },
            ..container::Style::default()
        })
        .into()
}

/// The status dot of a [`Level`].
pub fn dot<'a, M: 'a>(level: Level) -> El<'a, M> {
    dot_in(colour(level))
}

/// A folded block ("Raw response", "Full report"): `▸ summary`, and when open `▾ summary` over
/// the text in a monospace panel.
pub fn fold<'a>(state: &State, key: String, summary: &str, text: &'a str) -> El<'a, Msg> {
    let open = state.is_open(&key);
    let mut c = column![link_muted(if open { format!("▾ {summary}") } else { format!("▸ {summary}") }, Msg::Toggle(key))].spacing(6);
    if open {
        c = c.push(container(t(text).size(12).font(theme::mono())).padding(10).width(Length::Fill).style(theme::panel2));
    }
    c.into()
}

/// One list row with a status dot: the dot beside `body`, then a divider (a check, Encore's
/// download folders and app crashes).
pub fn dot_row<'a, M: 'a>(colour: Color, body: impl Into<El<'a, M>>) -> El<'a, M> {
    column![
        container(row![container(dot_in(colour)).padding([6, 0]), body.into()].spacing(12).align_y(Alignment::Start)).padding([10, 14]),
        widgets::divider(),
    ]
    .into()
}

/// One check of a group, the `i`-th on the page: the dot, the name with the HTTP badge, the time
/// and (when it did not pass) Copy; the request, what it said (red when it failed, amber for a
/// warning), `more` (an app's own line, Encore: the Soulseek session's queue), how to fix it, and
/// the folded raw answer.
pub fn check_item<'a>(state: &State, c: &'a Check, i: usize, more: Option<String>) -> El<'a, Msg> {
    let p = theme::look();
    let mut head = row![t(c.name.as_str()).size(14).font(theme::bold())].spacing(8).align_y(Alignment::Center);
    if let Some(h) = &c.http {
        head = head.push(badge(format!("HTTP {h}"), if c.ok { Badge::Ok } else { Badge::Err }));
    }
    if let Some(ms) = &c.ms {
        head = head.push(faint(format!("{ms} ms")).size(12));
    }
    if c.level != Level::Ok {
        head = head.push(details::copy_btn(Msg::Copy(check_text(c))));
    }
    let mut body = column![head].spacing(3).width(Length::Fill);
    if !c.request.is_empty() {
        body = body.push(faint(c.request.as_str()).size(12).font(theme::mono()));
    }
    if !c.shown.is_empty() {
        let colour = match c.level {
            Level::Ok => p.muted,
            Level::Warn => p.warn,
            Level::Fail => p.err,
        };
        body = body.push(t(c.shown.as_str()).size(12.5).color(colour));
    }
    if let Some(m) = more.filter(|m| !m.is_empty()) {
        body = body.push(faint(m).size(12));
    }
    if !c.fix.is_empty() {
        body = body.push(help(c.fix.as_str()));
    }
    if c.raw["body"].as_str().is_some_and(|b| !b.is_empty()) {
        body = body.push(fold(state, format!("check-{i}"), RAW, c.body.as_str()));
    }
    dot_row(colour(c.level), body)
}

/// The check groups, each a heading over a list (a group without checks is left out); push them
/// onto the page one by one. `kinds` is `(kind, heading)` in order ([`KINDS`] for Encore's);
/// the app's own checks come last in their kind's group; `more` gives a check an app's own line.
pub fn groups<'a>(state: &'a State, kinds: &[(&str, &str)], more: impl Fn(&Check) -> Option<String>) -> Vec<El<'a, Msg>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    for (kind, title) in kinds {
        let group: Vec<&'a Check> = state.all().filter(|c| c.kind == *kind).collect();
        if group.is_empty() {
            continue;
        }
        out.push(widgets::section_head(title.to_string(), None));
        let mut list = column![];
        for c in group {
            list = list.push(check_item(state, c, i, more(c)));
            i += 1;
        }
        out.push(widgets::card_flat(list).width(Length::Fill).into());
    }
    out
}

/// The page's head: "Diagnostics" over the `help` line, Copy report (once there is an answer) and
/// Run checks (Checking… while they run).
pub fn head<'a>(state: &State, help_line: &'a str) -> El<'a, Msg> {
    widgets::page_head(
        TITLE,
        Some(help(help_line).into()),
        vec![
            btn_icon(Icon::Copy, COPY_REPORT, Btn::Default, state.answer.as_ref().map(|_| Msg::CopyReport)).into(),
            btn_icon(Icon::Refresh, if state.running { RUNNING } else { RUN }, Btn::Primary, (!state.running).then_some(Msg::Run)).into(),
        ],
    )
}

/// The summary card: "N problems" or "All good" (every check counts, the app's too) beside
/// `about` (Encore: "Encore 0.27.0 · running as uid 1000 · <when>").
pub fn summary<'a>(state: &State, about: String) -> El<'a, Msg> {
    let (label, kind) = problems(state.all().filter(|c| !c.ok).count());
    widgets::card(row![badge(label, kind), help(about)].spacing(12).align_y(Alignment::Center)).width(Length::Fill).into()
}

/// `(label, value)` facts in a flat card, the label muted in a 220px column (Encore's Server block).
pub fn facts<'a, M: 'a>(rows: Vec<(String, String)>) -> El<'a, M> {
    let mut c = column![];
    for (k, v) in rows {
        c = c.push(container(row![small_muted(k).width(220), t(v).size(13)].spacing(10).align_y(Alignment::Start)).padding([8, 14]));
    }
    widgets::card_flat(c).width(Length::Fill).into()
}

/// The whole page for an app that has only the checks: the [`head`], the error, then the
/// [`summary`] and the [`groups`] ([`KINDS`]), or grey rows while the first run runs.
pub fn view<'a>(state: &'a State, help_line: &'a str, about: String) -> El<'a, Msg> {
    let mut page = column![head(state, help_line)].spacing(16).width(Length::Fill);
    if !state.error.is_empty() {
        page = page.push(widgets::error_box(&state.error));
    }
    if state.answer.is_some() {
        page = page.push(summary(state, about));
        page = page.extend(groups(state, &KINDS, |_| None));
    } else if state.running {
        page = page.push(widgets::skeleton_rows(5));
    }
    page.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn checks_parse_as_the_server_sends_them() {
        // Encore's shape: no level, a numeric HTTP status
        let a = json!({ "checks": [
            { "name": "MusicBrainz", "kind": "service", "ok": false, "request": "GET x", "detail": "timeout", "status": 504, "ms": 30, "body": "bad" },
            { "name": "Music library", "kind": "path", "ok": true, "detail": "Writable" },
        ] });
        let c = checks(&a);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].level, c[0].http.as_deref(), c[0].ms.as_deref()), (Level::Fail, Some("504"), Some("30")));
        assert_eq!((c[0].body.as_str(), c[0].fix.as_str()), ("bad", ""));
        assert_eq!((c[1].level, c[1].http.as_deref(), c[1].ms.as_deref()), (Level::Ok, None, None));
        assert_eq!(c[1].shown, "Writable");
        // the kit's fields: the level and the fix
        let k = Check::from_value(
            &json!({ "name": "Disk", "kind": "path", "ok": true, "detail": "90% full", "status": "warn", "fix": "Free some space" }),
        );
        assert_eq!((k.level, k.http, k.fix.as_str()), (Level::Warn, None, "Free some space"));
        assert_eq!(Check::from_value(&json!({ "ok": true, "status": "fail" })).level, Level::Fail);
        assert_eq!(Check::from_value(&json!({ "ok": false, "status": 0, "ms": null })).http, None);
        assert!(checks(&json!({})).is_empty());
    }

    #[test]
    fn the_report_and_check_text() {
        let d = json!({ "checks": [{ "name": "MusicBrainz", "ok": false, "request": "GET x", "detail": "timeout", "status": 504, "ms": 30, "body": "bad" }] });
        let rows = vec![("CPUs", "4".to_string()), ("Memory", "8.0 GB (container limit)".to_string())];
        let sections = vec![
            Section { title: "Server".into(), lines: fact_lines(&rows) },
            Section { title: "Settings".into(), lines: vec!["  app_name = Encore  [env ENCORE_APP_NAME]".into()] },
        ];
        let r = report("Encore 0.27.0 diagnostics (uid 1000)", &checks(&d), &sections);
        assert_eq!(
            r,
            "Encore 0.27.0 diagnostics (uid 1000)\nFAIL MusicBrainz – GET x → HTTP 504 (30 ms)\n     timeout\n     body: bad\n\nServer:\n  CPUs: 4\n  Memory: 8.0 GB (container limit)\n\nSettings:\n  app_name = Encore  [env ENCORE_APP_NAME]"
        );
        // a check that passed: no body; the raw answer is cut at 300 characters
        let ok =
            Check::from_value(&json!({ "name": "Tidal", "ok": true, "request": "GET t", "detail": "OK", "status": 200, "body": "fine" }));
        assert_eq!(report("x", [&ok], &[]), "x\nOK   Tidal – GET t → HTTP 200\n     OK");
        let long = "x".repeat(900);
        let c = Check::from_value(&json!({ "name": "MusicBrainz", "ok": false, "request": "GET x", "detail": "timeout", "body": long }));
        assert!(report("x", [&c], &[]).ends_with(&format!("body: {}", "x".repeat(300))));
        // Copy takes one check whole, the raw answer included
        assert_eq!(check_text(&c), format!("MusicBrainz\nGET x\ntimeout\n{long}"));
        // the kit's warn level and fix
        let w = Check::from_value(&json!({ "name": "Disk", "ok": true, "detail": "90% full", "status": "warn", "fix": "Free some space" }));
        assert_eq!(check_text(&w), "Disk\n90% full\nFree some space");
        assert_eq!(report("x", [&w], &[]), "x\nWARN Disk – \n     90% full\n     Free some space");
        // the page's words go to Copy, the server's to the report
        let mut s = Check::from_value(&json!({ "name": "Soulseek", "ok": true, "detail": "a" }));
        s.shown = "b".into();
        assert_eq!(check_text(&s), "Soulseek\nb");
        assert!(report("x", [&s], &[]).ends_with("     a"));
        assert_eq!(problems(0), ("All good".to_string(), Badge::Ok));
        assert_eq!(problems(1), ("1 problem".to_string(), Badge::Err));
        assert_eq!(problems(3).0, "3 problems");
        let dlg: Details<()> = details(&c);
        assert_eq!((dlg.title.as_str(), dlg.text.as_str()), ("MusicBrainz", check_text(&c).as_str()));
    }

    #[test]
    fn update_says_what_the_app_does() {
        let mut st = State::default();
        assert_eq!(st.open(), Action::Run);
        assert!(st.running());
        // nothing to copy before the first answer
        assert_eq!(st.update(Msg::CopyReport), None);
        st.checked(Ok(json!({ "checks": [{ "name": "A", "kind": "service", "ok": false }] })));
        assert!(!st.running());
        assert_eq!(st.update(Msg::CopyReport), Some(Action::CopyReport));
        assert_eq!(st.update(Msg::Copy("A".into())), Some(Action::Copy("A".into())));
        assert_eq!(st.update(Msg::Toggle("check-0".into())), None);
        assert!(st.is_open("check-0"));
        st.update(Msg::Toggle("check-0".into()));
        assert!(!st.is_open("check-0"));
        // a run that fails keeps the last answer and says why
        assert_eq!(st.update(Msg::Run), Some(Action::Run));
        st.checked(Err("Server unreachable".into()));
        assert_eq!((st.error(), st.answer().is_some(), st.all().count()), ("Server unreachable", true, 1));
        assert_eq!(st.update(Msg::Run), Some(Action::Run));
        assert_eq!(st.error(), "");
        // the app's own checks come after the server's
        st.set_extra(vec![Check::from_value(&json!({ "name": "VPN for Soulseek", "kind": "service", "ok": true }))]);
        assert_eq!(st.all().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["A", "VPN for Soulseek"]);
        st.update(Msg::Toggle("x".into()));
        st.open();
        assert_eq!((st.answer(), st.all().count(), st.is_open("x")), (None, 0, false));
    }

    /// The page drawn headless: a check's name and its Copy, which sends the check in full.
    #[test]
    fn the_page_shows_a_check_and_its_copy() {
        let mut st = State::default();
        st.checked(Ok(
            json!({ "checks": [{ "name": "MusicBrainz", "kind": "service", "ok": false, "request": "GET x", "detail": "timeout" }] }),
        ));
        let mut ui = iced_test::simulator(view(&st, "Runs every check.", "Encore 0.27.0".into()));
        assert!(ui.find("MusicBrainz").is_ok());
        assert!(ui.find("Connections").is_ok());
        assert!(ui.find("1 problem").is_ok());
        assert!(ui.find(COPY_REPORT).is_ok());
        assert!(ui.click(details::COPY).is_ok());
        let sent: Vec<Msg> = ui.into_messages().collect();
        assert_eq!(sent, vec![Msg::Copy("MusicBrainz\nGET x\ntimeout".into())]);
    }
}
