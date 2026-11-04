//! Reloading an open page into a new build of the web app by itself, without cutting off what the person is
//! doing.
//!
//! How a new build is noticed, two ways that meet in [`Gate::ready`]:
//! - `/api/health` names the build the server serves ([`Health::web`]); the page knows its own from the
//!   script it loaded ([`build_in`]). [`Watch::seen`] compares them. The page asks every minute
//!   ([`POLL_MS`]), when the tab comes back into view (at most every [`FOCUS_GAP_MS`]) and when the browser is
//!   online again ([`Checker`]). A server that does not answer is being restarted: ask every 3 s, after two
//!   minutes every 15 s ([`down_poll_ms`]).
//! - The service worker finds a new worker script and the new worker waits ([`Flow`]); [`worker_ready`] says
//!   whether that is a reload or a quiet takeover. With a worker in charge, a new build from `/api/health`
//!   first asks for the new worker and waits for it a while ([`new_build`], [`worker_wait`]).
//!
//! When to reload is [`Gate`]: at once when nothing holds the page ([`waiting_for`]); otherwise the toast
//! ([`ready_text`], [`RELOAD`]) and a reload once nothing has held the page for [`IDLE_FOR_MS`]. What holds a
//! page is the app's to say: each app passes its own reasons with their words ("It reloads when the upload is
//! done"); the kit knows only an open dialog ([`WAITS_FOR_DIALOG`]). A reload that did not bring the new build is
//! not repeated ([`tried`], [`tried_key`]). The words are [`UpdateWords`].

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How often the page asks `/api/health` while all is well.
pub const POLL_MS: u64 = 60 * 1000;
/// How often it asks while the server does not answer (a restart takes seconds).
pub const DOWN_POLL_MS: u64 = 3 * 1000;
/// After this long without an answer the server is down, not restarting …
pub const DOWN_SLOW_AFTER_MS: u64 = 2 * 60 * 1000;
/// … and it is asked this often.
pub const DOWN_SLOW_POLL_MS: u64 = 15 * 1000;
/// Coming back into view asks at most this often.
pub const FOCUS_GAP_MS: u64 = 15 * 1000;
/// The first question after the page loads (the boot has its own calls to make first).
pub const FIRST_CHECK_MS: u64 = 10 * 1000;
/// `/api/health` gets this long to answer before it counts as no answer.
pub const HEALTH_TIMEOUT_MS: u64 = 3000;
/// With a service worker in charge, how long a new build waits for its new worker before reloading anyway.
pub const WORKER_WAIT_MS: u64 = 20 * 1000;
/// How often that wait looks whether the new worker is there.
pub const WORKER_POLL_MS: u64 = 500;
/// Nothing may have held the page for this long before it reloads by itself.
pub const IDLE_FOR_MS: u64 = 5 * 1000;
/// How often a waiting reload looks again whether the page is free.
pub const TICK_MS: u64 = 1000;

/// `visibilitychange` → visible asks the browser for a new service worker at most this often.
pub const SW_CHECK_EVERY_MS: u64 = 10 * 60 * 1000;
/// While the tab stays in view, a service worker check this often (the same throttle applies).
pub const SW_TIMER_MS: u64 = 60 * 60 * 1000;
/// After Reload the page reloads on `controllerchange`; this long without one, it reloads anyway.
pub const RELOAD_FALLBACK_MS: u64 = 4000;

/// The update toast's button.
pub const RELOAD: &str = "Reload";
/// The toast's second line while a dialog is open (the one reason the kit knows itself).
pub const WAITS_FOR_DIALOG: &str = "It reloads when this dialog is closed";
/// The update toast's text, `{app}` filled in ([`ready_text`]).
pub const READY: &str = "A new version of {app} is ready";

/// The update toast's words. [`UpdateWords::DEFAULT`] holds the kit's; an app passes its own to change any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateWords {
    /// The toast's text, with `{app}` ([`READY`]).
    pub ready: &'static str,
    /// The toast's button ([`RELOAD`]).
    pub reload: &'static str,
    /// The second line while a dialog is open ([`WAITS_FOR_DIALOG`]).
    pub waits_for_dialog: &'static str,
}

impl UpdateWords {
    /// The kit's words.
    pub const DEFAULT: UpdateWords = UpdateWords { ready: READY, reload: RELOAD, waits_for_dialog: WAITS_FOR_DIALOG };

    /// The toast's text for `app`.
    pub fn ready(&self, app: &str) -> String {
        crate::app::fill(self.ready, &[("app", app)])
    }
}

impl Default for UpdateWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The update toast's text for `app` ("A new version of Kit Demo is ready").
pub fn ready_text(app: &str) -> String {
    UpdateWords::DEFAULT.ready(app)
}

/// The sessionStorage key that keeps the build this tab last reloaded for, the loop guard: `<app id>.update.tried`
/// (the id in lower case).
pub fn tried_key(app_id: &str) -> String {
    crate::app::key(&app_id.to_ascii_lowercase(), "update.tried")
}

/// Whether this tab already reloaded once for build `id` and it did not come (`stored` is the value under
/// [`tried_key`]). An announcement without an id (the worker found a new shell while offline) never counts.
pub fn tried(stored: Option<&str>, id: &str) -> bool {
    !id.is_empty() && stored == Some(id)
}

/// The answer of `GET /api/health` (`contracts/health.md`). Unknown fields are ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Health {
    /// Always true when the server answers.
    pub ok: bool,
    /// The server's version (`MAJOR.MINOR.PATCH`, or `MAJOR.MINOR.PATCH.N` for a hotfix); `null` reads as "".
    #[serde(deserialize_with = "null_as_empty")]
    pub version: String,
    /// The server's implementation (`"rust"`).
    pub server: Option<String>,
    /// The build of the web app it serves (the hash in the script's name, `<stem>-<hash>.js`), or None.
    pub web: Option<String>,
}

/// `null` (or a missing field) as "", the way JavaScript's `String(v ?? '')` reads a string.
fn null_as_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

impl Health {
    /// The answer's body, when it is one with `ok`; None otherwise (it then counts as no answer).
    pub fn parse(body: &str) -> Option<Health> {
        serde_json::from_str::<Health>(body).ok().filter(|h| h.ok)
    }
}

/// What one answer of `/api/health` says about the build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// The server serves the build this page runs.
    Same,
    /// The server serves another build; its id (the hash, or the version when it names no build).
    New(String),
    /// No answer: the server is restarting or down.
    Down,
}

/// The comparison of this page's build with the server's, apart from the network.
#[derive(Debug, Default, Clone)]
pub struct Watch {
    /// This page's build (the hash of its script), when it could tell.
    mine: Option<String>,
    /// The first version the server named (the fallback when a side has no build id).
    version: Option<String>,
}

impl Watch {
    /// A watch for a page running build `mine` ([`build_in`]).
    pub fn new(mine: Option<String>) -> Self {
        Watch { mine, version: None }
    }

    /// One answer of `/api/health` (None: none came, or not with `ok`). A server that names no build (or a
    /// page that could not tell its own) is compared by `version` against the first one this page saw.
    pub fn seen(&mut self, health: Option<&Health>) -> Seen {
        let Some(h) = health else { return Seen::Down };
        let web = h.web.as_deref().unwrap_or("");
        if let (Some(mine), false) = (&self.mine, web.is_empty()) {
            return if mine == web { Seen::Same } else { Seen::New(web.to_string()) };
        }
        // one side has no build id: the server's version against the first one this page saw
        let v = &h.version;
        if v.is_empty() {
            return Seen::Same;
        }
        match &self.version {
            None => {
                self.version = Some(v.clone());
                Seen::Same
            }
            Some(first) if first != v => Seen::New(v.clone()),
            Some(_) => Seen::Same,
        }
    }
}

/// What the page does next about a ready update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Nothing.
    Nothing,
    /// Reload into the new version now (with a service worker: [`Flow::reload_pressed`]), first keeping the
    /// build under [`tried_key`].
    Reload,
    /// Show the toast (once): something holds the page, or the last reload did not bring the new build.
    ShowToast,
}

/// When to reload once a new version is ready.
#[derive(Debug, Default, Clone)]
pub struct Gate {
    ready: bool,
    manual: bool,
    toast: bool,
    reloading: bool,
    idle_since: Option<u64>,
}

impl Gate {
    /// A new version is ready at `now_ms`. `held`: something holds the page right now ([`waiting_for`]);
    /// `tried`: this tab already reloaded once for this very build, and it did not come ([`tried`]).
    pub fn ready(&mut self, held: bool, tried: bool, now_ms: u64) -> Act {
        if self.ready || self.reloading {
            return Act::Nothing;
        }
        self.ready = true;
        self.manual = tried;
        if !held && !tried {
            self.reloading = true;
            return Act::Reload;
        }
        if !held {
            self.idle_since = Some(now_ms);
        }
        self.toast = true;
        Act::ShowToast
    }

    /// The tick (every [`TICK_MS`]) while a reload waits: reload once nothing has held the page for
    /// [`IDLE_FOR_MS`] (so a short pause between two things that hold it does not count as free).
    pub fn tick(&mut self, held: bool, now_ms: u64) -> Act {
        if !self.ready || self.reloading || self.manual {
            return Act::Nothing;
        }
        if held {
            self.idle_since = None;
            return Act::Nothing;
        }
        let since = *self.idle_since.get_or_insert(now_ms);
        if now_ms.saturating_sub(since) < IDLE_FOR_MS {
            return Act::Nothing;
        }
        self.reloading = true;
        Act::Reload
    }

    /// Reload pressed in the toast: reloads even while something holds the page.
    pub fn pressed(&mut self) -> Act {
        if self.reloading {
            return Act::Nothing;
        }
        self.reloading = true;
        Act::Reload
    }

    /// Whether a new version is ready (the tick runs, the toast may show; no more health checks).
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Whether only Reload in the toast reloads (the toast's second line then stays empty).
    pub fn is_manual(&self) -> bool {
        self.manual
    }

    /// Whether the toast was asked for.
    pub fn has_toast(&self) -> bool {
        self.toast
    }
}

/// What holds the page from reloading, as the toast's second line says it; None when nothing does. `holds`
/// are the app's reasons in order, each whether it holds the page now and its words ("It reloads when the
/// upload is done"); the first that holds counts.
pub fn waiting_for<'a>(holds: &[(bool, &'a str)]) -> Option<&'a str> {
    holds.iter().find(|(on, _)| *on).map(|(_, words)| *words)
}

/// The hash of the web app's script named in `html` (`<stem>-<hash>.js` or `<stem>-<hash>_bg.wasm`; `stem` is
/// the name trunk gives the script, the web crate's binary name, in any case): the build this page runs.
/// Trunk drops leading zeros, so 12 hex digits or more count.
pub fn build_in(html: &str, stem: &str) -> Option<String> {
    let prefix = format!("{}-", stem.to_ascii_lowercase());
    let mut rest = html;
    while let Some(i) = rest.find(&prefix) {
        rest = &rest[i + prefix.len()..];
        let hash: String = rest.chars().take_while(char::is_ascii_hexdigit).collect();
        let after = rest[hash.len()..].chars().next();
        if hash.len() >= 12 && matches!(after, Some('.') | Some('_')) {
            return Some(hash);
        }
    }
    None
}

/// How long to wait before asking a server that did not answer again, `waited_ms` after the check began: every
/// 3 s, after two minutes every 15 s.
pub fn down_poll_ms(waited_ms: u64) -> u64 {
    if waited_ms < DOWN_SLOW_AFTER_MS {
        DOWN_POLL_MS
    } else {
        DOWN_SLOW_POLL_MS
    }
}

/// Why the page asks `/api/health`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// [`FIRST_CHECK_MS`] after the page loaded, or every [`POLL_MS`].
    Timer,
    /// The tab came back into view (`visibilitychange`).
    Visible,
    /// The browser is online again (`online`).
    Online,
}

/// What to do after an answer of `/api/health`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStep {
    /// The server serves this build: the check is over.
    Done,
    /// No answer: ask again after this many ms.
    AskAfter(u64),
    /// The server serves build `id`: go on with [`new_build`]; call [`Checker::finish`] after announcing it.
    NewBuild(String),
}

/// One check at a time, the back-off while the server restarts and the focus throttle.
#[derive(Debug, Default, Clone)]
pub struct Checker {
    /// When the check under way began.
    since: Option<u64>,
    /// When the page last asked because it came into view.
    last_focus: Option<u64>,
}

impl Checker {
    /// Whether to ask now: not while a check runs, not once an update is ready (`ready`: [`Gate::is_ready`]),
    /// and not for a tab that came into view within [`FOCUS_GAP_MS`] of the last time. A yes starts the check.
    pub fn begin(&mut self, trigger: Trigger, ready: bool, now_ms: u64) -> bool {
        if trigger == Trigger::Visible {
            if self.last_focus.is_some_and(|t| now_ms.saturating_sub(t) < FOCUS_GAP_MS) {
                return false;
            }
            self.last_focus = Some(now_ms);
        }
        if ready || self.since.is_some() {
            return false;
        }
        self.since = Some(now_ms);
        true
    }

    /// The answer arrived (`seen` from [`Watch::seen`]) at `now_ms`.
    pub fn answer(&mut self, seen: Seen, now_ms: u64) -> CheckStep {
        match seen {
            Seen::Same => {
                self.since = None;
                CheckStep::Done
            }
            Seen::Down => CheckStep::AskAfter(down_poll_ms(now_ms.saturating_sub(self.since.unwrap_or(now_ms)))),
            Seen::New(id) => CheckStep::NewBuild(id),
        }
    }

    /// The new build was announced: the next check may run.
    pub fn finish(&mut self) {
        self.since = None;
    }

    /// A check is under way.
    pub fn checking(&self) -> bool {
        self.since.is_some()
    }
}

/// What to do once `/api/health` names a new build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewBuild {
    /// Ask the browser for the new service worker (`registration.update()`), then wait with [`worker_wait`].
    WaitForServiceWorker,
    /// Announce it now ([`Gate::ready`]).
    Announce,
}

/// With a service worker in charge (`controlled`) and no new worker waiting yet, have it installed first, so
/// the reload lands in the new shell and not the old one.
pub fn new_build(controlled: bool, waiting: bool) -> NewBuild {
    if controlled && !waiting {
        NewBuild::WaitForServiceWorker
    } else {
        NewBuild::Announce
    }
}

/// While a new build waits for its worker (since `started_ms`): Some(ms) to look again after that long, None
/// to announce now (the worker waits, or [`WORKER_WAIT_MS`] has passed).
pub fn worker_wait(started_ms: u64, waiting: bool, now_ms: u64) -> Option<u64> {
    (!waiting && now_ms.saturating_sub(started_ms) < WORKER_WAIT_MS).then_some(WORKER_POLL_MS)
}

/// What a waiting new service worker means, from the health answer asked right then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerReady {
    /// This page already runs the build the server serves: the worker takes over quietly
    /// ([`Flow::skip_waiting_message`], no reload).
    ActivateQuietly,
    /// A new version: announce build `id` (`""` when the server did not answer – the worker has the new shell
    /// all the same).
    Announce(String),
}

/// A new worker waits ([`SwStep::Ready`]): a reload when the server serves another build than this page's, a
/// quiet takeover when this page already runs it.
pub fn worker_ready(seen: Seen) -> WorkerReady {
    match seen {
        Seen::Same => WorkerReady::ActivateQuietly,
        Seen::New(id) => WorkerReady::Announce(id),
        Seen::Down => WorkerReady::Announce(String::new()),
    }
}

/// What the page does after a service worker event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwStep {
    /// Nothing.
    Nothing,
    /// A new worker waits: [`worker_ready`] decides.
    Ready,
    /// `{type: 'skip-waiting'}` went to the waiting worker; the reload follows on `controllerchange` (or after
    /// [`RELOAD_FALLBACK_MS`]).
    Posted,
    /// `location.reload()` now.
    Reload,
}

/// The page's side of a service worker update.
#[derive(Debug, Default, Clone)]
pub struct Flow {
    app: String,
    ready: bool,
    asked: bool,
    reloading: bool,
    last_check: Option<u64>,
    cache: Option<String>,
}

impl Flow {
    /// The flow for the app with id `app_id` (its worker answers `<id>:version` and names `<id>-shell-…`
    /// caches, the id in lower case).
    pub fn new(app_id: &str) -> Self {
        Flow { app: app_id.to_ascii_lowercase(), ..Flow::default() }
    }

    /// The message that lets a waiting worker take over.
    pub fn skip_waiting_message() -> Value {
        json!({ "type": "skip-waiting" })
    }

    /// The question the worker answers with `{type: '<app>:version', cache}`.
    pub fn version_question() -> Value {
        json!({ "type": "version?" })
    }

    /// The worker was registered at `now_ms`: the browser has just looked for a new one itself.
    pub fn registered(&mut self, now_ms: u64) {
        self.last_check = Some(now_ms);
    }

    /// A worker of this registration reached `state` (`installed`, `activated`, …). Reaching `installed` while
    /// the page has a controller is an update, reported once; without one it is the browser's first worker.
    pub fn worker_state(&mut self, state: &str, has_controller: bool) -> SwStep {
        if state == "installed" && has_controller && !self.ready {
            self.ready = true;
            return SwStep::Ready;
        }
        SwStep::Nothing
    }

    /// The page reloads into the new version (Reload pressed, or the page became free). `posted`: whether
    /// [`Flow::skip_waiting_message`] went to a waiting worker (None: no worker waits).
    pub fn reload_pressed(&mut self, posted: Option<bool>) -> SwStep {
        if self.reloading {
            return SwStep::Nothing;
        }
        self.asked = true;
        if posted == Some(true) {
            return SwStep::Posted;
        }
        // nothing waits any more (another tab switched already) or the message did not go out
        self.reloading = true;
        SwStep::Reload
    }

    /// `controllerchange`: a reload only when this page asked for the new version (the first worker claiming
    /// the page fires it too, and so does another tab's Reload).
    pub fn controller_changed(&mut self) -> SwStep {
        if self.asked && !self.reloading {
            self.reloading = true;
            return SwStep::Reload;
        }
        SwStep::Nothing
    }

    /// The fallback timer after Reload: reload when `controllerchange` has not done it.
    pub fn reload_timeout(&mut self) -> SwStep {
        if self.reloading {
            return SwStep::Nothing;
        }
        self.reloading = true;
        SwStep::Reload
    }

    /// The tab came into view (or the hourly timer fired) at `now_ms`: whether to call
    /// `registration.update()` (at most every [`SW_CHECK_EVERY_MS`]).
    pub fn should_check(&mut self, now_ms: u64) -> bool {
        if self.last_check.is_some_and(|t| now_ms.saturating_sub(t) < SW_CHECK_EVERY_MS) {
            return false;
        }
        self.last_check = Some(now_ms);
        true
    }

    /// A message from the worker; `{type: '<app>:version', cache: '<app>-shell-…'}` is remembered and
    /// returned.
    pub fn message(&mut self, data: &Value) -> Option<String> {
        if data.get("type").and_then(Value::as_str) != Some(format!("{}:version", self.app).as_str()) {
            return None;
        }
        let shell = format!("{}-shell-", self.app);
        let cache = data.get("cache").and_then(Value::as_str).filter(|c| c.starts_with(&shell))?.to_string();
        self.cache = Some(cache.clone());
        Some(cache)
    }

    /// The shell cache the controlling worker named, once it answered.
    pub fn cache(&self) -> Option<&str> {
        self.cache.as_deref()
    }

    /// Whether [`SwStep::Ready`] was reported.
    pub fn ready_seen(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(version: &str, web: Option<&str>) -> Health {
        Health { ok: true, version: version.into(), server: Some("rust".into()), web: web.map(String::from) }
    }

    #[test]
    fn the_build_is_read_from_the_pages_script() {
        let head = r#"<link rel="modulepreload" href="/kit-demo-web-f92ab5b84b305a59.js" crossorigin="anonymous">
            <link rel="preload" href="/kit-demo-web-f92ab5b84b305a59_bg.wasm" as="fetch" type="application/wasm">"#;
        assert_eq!(build_in(head, "Kit-Demo-Web").as_deref(), Some("f92ab5b84b305a59"));
        assert_eq!(build_in("import init from '/kit-demo-web-59d563296e62da3.js';", "kit-demo-web").as_deref(), Some("59d563296e62da3"));
        assert_eq!(build_in(r#"href="/kit-demo-web-f92ab5b84b305a59_bg.wasm""#, "kit-demo-web").as_deref(), Some("f92ab5b84b305a59"));
        assert_eq!(
            build_in("kit-demo-web-app kit-demo-web-12ab.js /kit-demo-web-0123456789abcdef.js", "kit-demo-web").as_deref(),
            Some("0123456789abcdef")
        );
        for none in [
            "",
            "<title>Kit Demo</title>",
            "kit-demo-web-",
            "kit-demo-web-f92ab5b8.js",
            "kit-demo-web-f92ab5b84b305a59x.js",
            "kit-demo-web-f92ab5b84b305a59",
        ] {
            assert_eq!(build_in(none, "kit-demo-web"), None, "{none}");
        }
        assert_eq!(build_in("/kit-demo-web-f92ab5b84b305a59.js", "other-web"), None);
        assert_eq!(build_in("/photos-3f9a0c1b2d4e5f60.js", "photos").as_deref(), Some("3f9a0c1b2d4e5f60"), "any stem");
    }

    #[test]
    fn another_build_on_the_server_is_new() {
        let mut w = Watch::new(Some("aaaaaaaaaaaa1111".into()));
        assert_eq!(w.seen(Some(&h("0.35.2", Some("aaaaaaaaaaaa1111")))), Seen::Same);
        assert_eq!(w.seen(None), Seen::Down);
        assert_eq!(w.seen(Some(&h("0.35.2.18", Some("bbbbbbbbbbbb2222")))), Seen::New("bbbbbbbbbbbb2222".into()));
        // the build decides, not the version
        let mut w = Watch::new(Some("aaaaaaaaaaaa1111".into()));
        assert_eq!(w.seen(Some(&h("0.36.0", Some("aaaaaaaaaaaa1111")))), Seen::Same);
    }

    #[test]
    fn without_a_build_id_the_version_decides() {
        let mut w = Watch::new(Some("aaaaaaaaaaaa1111".into()));
        assert_eq!(w.seen(Some(&h("0.35.2", None))), Seen::Same, "the first version seen is this page's");
        assert_eq!(w.seen(Some(&Health::parse(r#"{"ok":true,"version":"0.35.2","web":null}"#).unwrap())), Seen::Same);
        assert_eq!(w.seen(None), Seen::Down);
        assert_eq!(w.seen(Some(&h("0.35.3", None))), Seen::New("0.35.3".into()));
        let mut w = Watch::new(None);
        assert_eq!(w.seen(Some(&h("1.0.0", Some("bbbbbbbbbbbb2222")))), Seen::Same);
        assert_eq!(w.seen(Some(&h("1.0.1", Some("cccccccccccc3333")))), Seen::New("1.0.1".into()));
        let mut w = Watch::new(None);
        assert_eq!(w.seen(Some(&Health::parse(r#"{"ok":true}"#).unwrap())), Seen::Same);
    }

    #[test]
    fn only_an_ok_answer_counts() {
        assert_eq!(
            Health::parse(r#"{"ok":true,"version":"0.36.4","server":"rust","web":"3f9a0c1b2d4e","extra":1}"#).unwrap().web.as_deref(),
            Some("3f9a0c1b2d4e")
        );
        assert_eq!(Health::parse(r#"{"ok":false,"version":"1"}"#), None);
        assert_eq!(Health::parse("<html>502</html>"), None);
        assert_eq!(Health::parse(r#"{"ok":true,"version":null}"#).unwrap().version, "");
    }

    #[test]
    fn nothing_holding_the_page_reloads_at_once() {
        let mut g = Gate::default();
        assert!(!g.is_ready());
        assert_eq!(g.ready(false, false, 0), Act::Reload);
        assert_eq!(g.ready(false, false, 1), Act::Nothing);
        assert_eq!(g.tick(false, 10_000), Act::Nothing);
        assert_eq!(g.pressed(), Act::Nothing);
    }

    #[test]
    fn a_held_page_waits_until_it_has_been_free_for_a_while() {
        let mut g = Gate::default();
        assert_eq!(g.ready(true, false, 0), Act::ShowToast);
        assert!(g.has_toast());
        assert_eq!(g.ready(true, false, 1), Act::Nothing);
        assert_eq!(g.tick(true, 1_000), Act::Nothing);
        assert_eq!(g.tick(false, 2_000), Act::Nothing);
        assert_eq!(g.tick(false, 2_000 + IDLE_FOR_MS - 1), Act::Nothing);
        assert_eq!(g.tick(true, 2_000 + IDLE_FOR_MS), Act::Nothing);
        assert_eq!(g.tick(false, 10_000), Act::Nothing, "the quiet time starts again");
        assert_eq!(g.tick(false, 10_000 + IDLE_FOR_MS), Act::Reload);
        assert_eq!(g.tick(false, 20_000), Act::Nothing);
    }

    #[test]
    fn reload_in_the_toast_reloads_even_while_held() {
        let mut g = Gate::default();
        assert_eq!(g.ready(true, false, 0), Act::ShowToast);
        assert_eq!(g.pressed(), Act::Reload);
        assert_eq!(g.pressed(), Act::Nothing);
        assert_eq!(g.tick(false, 60_000), Act::Nothing);
    }

    #[test]
    fn a_reload_that_did_not_bring_the_build_is_not_repeated() {
        let mut g = Gate::default();
        assert_eq!(g.ready(false, true, 0), Act::ShowToast);
        assert!(g.is_manual());
        assert_eq!(g.tick(false, 60_000), Act::Nothing);
        assert_eq!(g.pressed(), Act::Reload);
        assert!(tried(Some("abc"), "abc"));
        assert!(!tried(Some("abc"), "def"));
        assert!(!tried(Some(""), ""), "an announcement without a build never counts");
        assert!(!tried(None, "abc"));
        assert_eq!(tried_key("Kit-Demo"), "kit-demo.update.tried");
    }

    #[test]
    fn what_holds_the_page_comes_in_order() {
        const UPLOAD: &str = "It reloads when the upload is done";
        const EDIT: &str = "It reloads when you have saved";
        let holds = |upload, edit, dialog| [(upload, UPLOAD), (edit, EDIT), (dialog, WAITS_FOR_DIALOG)];
        assert_eq!(waiting_for(&holds(false, false, false)), None);
        assert_eq!(waiting_for(&holds(true, true, true)), Some(UPLOAD));
        assert_eq!(waiting_for(&holds(false, true, true)), Some(EDIT));
        assert_eq!(waiting_for(&holds(false, false, true)), Some("It reloads when this dialog is closed"));
        assert_eq!(waiting_for(&[]), None, "an app with no reasons is never held");
        assert_eq!(ready_text("Kit Demo"), "A new version of Kit Demo is ready");
        let mine = UpdateWords { ready: "{app} hat eine neue Version", ..UpdateWords::DEFAULT };
        assert_eq!(mine.ready("Filebrary"), "Filebrary hat eine neue Version");
        assert_eq!(UpdateWords::default().reload, "Reload");
    }

    #[test]
    fn a_restarting_server_is_asked_every_3_s_then_every_15_s() {
        let mut c = Checker::default();
        assert!(c.begin(Trigger::Timer, false, 1_000));
        assert!(!c.begin(Trigger::Online, false, 1_500), "one check at a time");
        assert_eq!(c.answer(Seen::Down, 1_000), CheckStep::AskAfter(3_000));
        assert_eq!(c.answer(Seen::Down, 1_000 + DOWN_SLOW_AFTER_MS - 1), CheckStep::AskAfter(3_000));
        assert_eq!(c.answer(Seen::Down, 1_000 + DOWN_SLOW_AFTER_MS), CheckStep::AskAfter(15_000));
        assert_eq!(c.answer(Seen::Same, 200_000), CheckStep::Done);
        assert!(!c.checking());
        assert!(c.begin(Trigger::Timer, false, 300_000));
        assert_eq!(c.answer(Seen::New("b".into()), 300_100), CheckStep::NewBuild("b".into()));
        assert!(c.checking(), "until the new build is announced");
        c.finish();
        assert!(!c.begin(Trigger::Timer, true, 400_000), "an update is ready: no more checks");
    }

    #[test]
    fn coming_into_view_asks_at_most_every_15_s() {
        let mut c = Checker::default();
        assert!(c.begin(Trigger::Visible, false, 100_000));
        c.answer(Seen::Same, 100_001);
        assert!(!c.begin(Trigger::Visible, false, 100_000 + FOCUS_GAP_MS - 1));
        assert!(c.begin(Trigger::Visible, false, 100_000 + FOCUS_GAP_MS));
    }

    #[test]
    fn a_new_build_waits_for_its_service_worker_a_while() {
        assert_eq!(new_build(true, false), NewBuild::WaitForServiceWorker);
        assert_eq!(new_build(true, true), NewBuild::Announce);
        assert_eq!(new_build(false, false), NewBuild::Announce);
        assert_eq!(worker_wait(0, false, 0), Some(WORKER_POLL_MS));
        assert_eq!(worker_wait(0, false, WORKER_WAIT_MS - 1), Some(500));
        assert_eq!(worker_wait(0, false, WORKER_WAIT_MS), None);
        assert_eq!(worker_wait(0, true, 10), None);
        assert_eq!(worker_ready(Seen::Same), WorkerReady::ActivateQuietly);
        assert_eq!(worker_ready(Seen::New("b".into())), WorkerReady::Announce("b".into()));
        assert_eq!(worker_ready(Seen::Down), WorkerReady::Announce(String::new()));
    }

    #[test]
    fn a_new_worker_is_reported_once_and_only_as_an_update() {
        let mut f = Flow::new("Kit-Demo");
        assert_eq!(f.worker_state("installing", false), SwStep::Nothing);
        assert_eq!(f.worker_state("installed", false), SwStep::Nothing);
        assert!(!f.ready_seen());
        assert_eq!(f.worker_state("installing", true), SwStep::Nothing);
        assert_eq!(f.worker_state("installed", true), SwStep::Ready);
        assert!(f.ready_seen());
        assert_eq!(f.worker_state("installed", true), SwStep::Nothing);
        assert_eq!(f.worker_state("activated", true), SwStep::Nothing);
    }

    #[test]
    fn reload_posts_skip_waiting_then_reloads_on_controllerchange() {
        let mut f = Flow::new("kit-demo");
        f.worker_state("installed", true);
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Posted);
        assert_eq!(Flow::skip_waiting_message(), json!({ "type": "skip-waiting" }));
        assert_eq!(f.controller_changed(), SwStep::Reload);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        assert_eq!(f.reload_timeout(), SwStep::Nothing);
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Nothing);
    }

    #[test]
    fn the_fallback_timer_reloads_when_controllerchange_never_comes() {
        let mut f = Flow::new("kit-demo");
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Posted);
        assert_eq!(f.reload_timeout(), SwStep::Reload);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
    }

    #[test]
    fn controllerchange_without_reload_pressed_keeps_the_page() {
        let mut f = Flow::new("kit-demo");
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        f.worker_state("installed", true);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        assert_eq!(f.reload_pressed(None), SwStep::Reload);
        let mut g = Flow::new("kit-demo");
        assert_eq!(g.reload_pressed(Some(false)), SwStep::Reload, "a refused message reloads at once");
    }

    #[test]
    fn update_checks_are_at_least_ten_minutes_apart() {
        let mut f = Flow::new("kit-demo");
        let t0 = 1_700_000_000_000;
        assert!(f.should_check(t0));
        assert!(!f.should_check(t0 + 60_000));
        assert!(!f.should_check(t0 + SW_CHECK_EVERY_MS - 1));
        assert!(f.should_check(t0 + SW_CHECK_EVERY_MS));
        let mut g = Flow::new("kit-demo");
        g.registered(t0);
        assert!(!g.should_check(t0 + 5 * 60_000));
        assert!(g.should_check(t0 + 11 * 60_000));
    }

    #[test]
    fn the_version_answer_is_remembered() {
        let mut f = Flow::new("Kit-Demo");
        assert_eq!(f.message(&json!({ "type": "other", "cache": "kit-demo-shell-abc" })), None);
        assert_eq!(f.message(&json!({ "type": "kit-demo:version" })), None);
        assert_eq!(f.message(&json!({ "type": "kit-demo:version", "cache": "kit-demo-offline-v1" })), None);
        assert_eq!(f.message(&Value::Null), None);
        assert_eq!(f.cache(), None);
        assert_eq!(
            f.message(&json!({ "type": "kit-demo:version", "cache": "kit-demo-shell-e56412d8e3fb9a38" })).as_deref(),
            Some("kit-demo-shell-e56412d8e3fb9a38")
        );
        assert_eq!(f.cache(), Some("kit-demo-shell-e56412d8e3fb9a38"));
        assert_eq!(Flow::version_question()["type"], "version?");
    }
}
