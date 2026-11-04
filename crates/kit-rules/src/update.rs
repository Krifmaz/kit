//! Reloading an open page into a new build of the web app by itself, without cutting off the music.
//!
//! From Encore's `web-rs/src/update.rs` (`Watch`, `Gate`, `waiting_for`, `build_in`, `check`, `new_build`,
//! `worker_ready`, `announce` and their constants) and `web-rs/src/sw.rs` (`Flow`, the service worker's side).
//! The browser calls (fetch, timers, sessionStorage, `navigator.serviceWorker`) stay in the UI crate; here are
//! the decisions, fed with answers and the time (`now_ms`) and returning what to do next. The contract is
//! `contracts/health.md`.
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
//! ([`ready_text`], [`RELOAD`]) and a reload once nothing has held the page for [`IDLE_FOR_MS`]. A reload that
//! did not bring the new build is not repeated ([`tried`], [`tried_key`]).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// How often the page asks `/api/health` while all is well. From `update.rs` `POLL_MS`.
pub const POLL_MS: u64 = 60 * 1000;
/// How often it asks while the server does not answer (a restart takes seconds). From `update.rs`
/// `DOWN_POLL_MS`.
pub const DOWN_POLL_MS: u64 = 3 * 1000;
/// After this long without an answer the server is down, not restarting … From `update.rs`
/// `DOWN_SLOW_AFTER_MS`.
pub const DOWN_SLOW_AFTER_MS: u64 = 2 * 60 * 1000;
/// … and it is asked this often. From `update.rs` `DOWN_SLOW_POLL_MS`.
pub const DOWN_SLOW_POLL_MS: u64 = 15 * 1000;
/// Coming back into view asks at most this often. From `update.rs` `FOCUS_GAP_MS`.
pub const FOCUS_GAP_MS: u64 = 15 * 1000;
/// The first question after the page loads (the boot has its own calls to make first). From `update.rs`
/// `FIRST_CHECK_MS`.
pub const FIRST_CHECK_MS: u64 = 10 * 1000;
/// `/api/health` gets this long to answer before it counts as no answer. From `update.rs` `health` (3 s abort).
pub const HEALTH_TIMEOUT_MS: u64 = 3000;
/// With a service worker in charge, how long a new build waits for its new worker before reloading anyway.
/// From `update.rs` `WORKER_WAIT_MS`.
pub const WORKER_WAIT_MS: u64 = 20 * 1000;
/// How often that wait looks whether the new worker is there. From `update.rs` `new_build` (`sleep(500)`).
pub const WORKER_POLL_MS: u64 = 500;
/// Nothing may have held the page for this long before it reloads by itself. From `update.rs` `IDLE_FOR_MS`.
pub const IDLE_FOR_MS: u64 = 5 * 1000;
/// How often a waiting reload looks again whether the page is free. From `update.rs` `TICK_MS`.
pub const TICK_MS: u64 = 1000;

/// `visibilitychange` → visible asks the browser for a new service worker at most this often. From `sw.rs`
/// `CHECK_EVERY_MS`.
pub const SW_CHECK_EVERY_MS: u64 = 10 * 60 * 1000;
/// While the tab stays in view, a service worker check this often (the same throttle applies). From `sw.rs`
/// `TIMER_MS`.
pub const SW_TIMER_MS: u64 = 60 * 60 * 1000;
/// After Reload the page reloads on `controllerchange`; this long without one, it reloads anyway. From `sw.rs`
/// `RELOAD_FALLBACK_MS`.
pub const RELOAD_FALLBACK_MS: u64 = 4000;

/// The update toast's button. From `update.rs` `RELOAD_LABEL`.
pub const RELOAD: &str = "Reload";
/// The toast's second line while music plays. From `update.rs` `waiting_for`.
pub const WAITS_FOR_MUSIC: &str = "It reloads when the music stops";
/// The toast's second line during an upload. From `update.rs` `waiting_for`.
pub const WAITS_FOR_UPLOAD: &str = "It reloads when the upload is done";
/// The toast's second line while songs are saved for offline. From `update.rs` `waiting_for`.
pub const WAITS_FOR_DOWNLOAD: &str = "It reloads when the songs are saved";
/// The toast's second line while a dialog is open. From `update.rs` `waiting_for`.
pub const WAITS_FOR_DIALOG: &str = "It reloads when this dialog is closed";

/// The update toast's text for `app` ("A new version of Encore is ready"). From `update.rs` `READY_TEXT`.
pub fn ready_text(app: &str) -> String {
    format!("A new version of {app} is ready")
}

/// The sessionStorage key that keeps the build this tab last reloaded for, the loop guard (`app` in lower
/// case: `encore.update.tried`). From `update.rs` `TRIED_KEY`.
pub fn tried_key(app: &str) -> String {
    format!("{}.update.tried", app.to_ascii_lowercase())
}

/// Whether this tab already reloaded once for build `id` and it did not come (`stored` is the value under
/// [`tried_key`]). An announcement without an id (the worker found a new shell while offline) never counts.
///
/// From `update.rs` `announce`.
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
    /// The build of the web app it serves (the hash in `<app>-web-<hash>.js`), or None.
    pub web: Option<String>,
}

/// `null` (or a missing field) as "", the way Encore's `J::s` reads a string.
fn null_as_empty<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

impl Health {
    /// The answer's body, when it is one with `ok`; None otherwise (it then counts as no answer).
    ///
    /// From `update.rs` `health` (`v.truthy("ok").then_some(v)`).
    pub fn parse(body: &str) -> Option<Health> {
        serde_json::from_str::<Health>(body).ok().filter(|h| h.ok)
    }
}

/// What one answer of `/api/health` says about the build.
///
/// From `update.rs` `Seen`.
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
///
/// From `update.rs` `Watch`.
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
///
/// From `update.rs` `Act`.
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
///
/// From `update.rs` `Gate`.
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
    /// [`IDLE_FOR_MS`] (so the short gap between two songs does not count as stopped).
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

    /// Reload pressed in the toast: reloads even while music plays.
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

/// What holds the page from reloading, as the toast's second line says it; None when nothing does. The first
/// one counts: music, then an upload, then saving for offline, then an open dialog.
///
/// From `update.rs` `waiting_for`.
pub fn waiting_for(playing: bool, uploading: bool, downloading: bool, dialog: bool) -> Option<&'static str> {
    if playing {
        Some(WAITS_FOR_MUSIC)
    } else if uploading {
        Some(WAITS_FOR_UPLOAD)
    } else if downloading {
        Some(WAITS_FOR_DOWNLOAD)
    } else if dialog {
        Some(WAITS_FOR_DIALOG)
    } else {
        None
    }
}

/// The hash of the web app's script named in `html` (`<app>-web-<hash>.js` or `…_bg.wasm`, `app` in lower
/// case): the build this page runs. Trunk drops leading zeros, so 12 hex digits or more count.
///
/// From `update.rs` `build_in` (where the prefix is `encore-web-`).
pub fn build_in(html: &str, app: &str) -> Option<String> {
    let prefix = format!("{}-web-", app.to_ascii_lowercase());
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
///
/// From `update.rs` `check`.
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
///
/// From `update.rs` `check` / `start` (`CHECKING`, `LAST_FOCUS`).
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
///
/// From `update.rs` `new_build`.
pub fn new_build(controlled: bool, waiting: bool) -> NewBuild {
    if controlled && !waiting {
        NewBuild::WaitForServiceWorker
    } else {
        NewBuild::Announce
    }
}

/// While a new build waits for its worker (since `started_ms`): Some(ms) to look again after that long, None
/// to announce now (the worker waits, or [`WORKER_WAIT_MS`] has passed).
///
/// From `update.rs` `new_build`.
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
///
/// From `update.rs` `worker_ready`.
pub fn worker_ready(seen: Seen) -> WorkerReady {
    match seen {
        Seen::Same => WorkerReady::ActivateQuietly,
        Seen::New(id) => WorkerReady::Announce(id),
        Seen::Down => WorkerReady::Announce(String::new()),
    }
}

/// What the page does after a service worker event.
///
/// From `sw.rs` `Step`.
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
///
/// From `sw.rs` `Flow`.
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
    /// The flow for `app` (its worker answers `<app>:version` and names `<app>-shell-…` caches, in lower case).
    pub fn new(app: &str) -> Self {
        Flow { app: app.to_ascii_lowercase(), ..Flow::default() }
    }

    /// The message that lets a waiting worker take over. From `sw.rs` `skip_waiting_message`.
    pub fn skip_waiting_message() -> Value {
        json!({ "type": "skip-waiting" })
    }

    /// The question the worker answers with `{type: '<app>:version', cache}`. From `sw.rs` `version_question`.
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

    /// The page reloads into the new version (Reload pressed, or the music stopped). `posted`: whether
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

    /// Ported from `update.rs` `the_build_is_read_from_the_pages_script`.
    #[test]
    fn the_build_is_read_from_the_pages_script() {
        let head = r#"<link rel="modulepreload" href="/encore-web-f92ab5b84b305a59.js" crossorigin="anonymous">
            <link rel="preload" href="/encore-web-f92ab5b84b305a59_bg.wasm" as="fetch" type="application/wasm">"#;
        assert_eq!(build_in(head, "Encore").as_deref(), Some("f92ab5b84b305a59"));
        assert_eq!(build_in("import init from '/encore-web-59d563296e62da3.js';", "encore").as_deref(), Some("59d563296e62da3"));
        assert_eq!(build_in(r#"href="/encore-web-f92ab5b84b305a59_bg.wasm""#, "encore").as_deref(), Some("f92ab5b84b305a59"));
        assert_eq!(
            build_in("encore-web-app encore-web-12ab.js /encore-web-0123456789abcdef.js", "encore").as_deref(),
            Some("0123456789abcdef")
        );
        for none in [
            "",
            "<title>Encore</title>",
            "encore-web-",
            "encore-web-f92ab5b8.js",
            "encore-web-f92ab5b84b305a59x.js",
            "encore-web-f92ab5b84b305a59",
        ] {
            assert_eq!(build_in(none, "encore"), None, "{none}");
        }
        assert_eq!(build_in("/encore-web-f92ab5b84b305a59.js", "other"), None);
    }

    /// Ported from `update.rs` `another_build_on_the_server_is_new`.
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

    /// Ported from `update.rs` `without_a_build_id_the_version_decides`.
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

    /// Ported from `update.rs` `nothing_playing_reloads_at_once`.
    #[test]
    fn nothing_playing_reloads_at_once() {
        let mut g = Gate::default();
        assert!(!g.is_ready());
        assert_eq!(g.ready(false, false, 0), Act::Reload);
        assert_eq!(g.ready(false, false, 1), Act::Nothing);
        assert_eq!(g.tick(false, 10_000), Act::Nothing);
        assert_eq!(g.pressed(), Act::Nothing);
    }

    /// Ported from `update.rs` `music_waits_until_it_has_stopped_for_a_while` (times in whole ms).
    #[test]
    fn music_waits_until_it_has_stopped_for_a_while() {
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

    /// Ported from `update.rs` `reload_in_the_toast_reloads_even_while_playing`.
    #[test]
    fn reload_in_the_toast_reloads_even_while_playing() {
        let mut g = Gate::default();
        assert_eq!(g.ready(true, false, 0), Act::ShowToast);
        assert_eq!(g.pressed(), Act::Reload);
        assert_eq!(g.pressed(), Act::Nothing);
        assert_eq!(g.tick(false, 60_000), Act::Nothing);
    }

    /// Ported from `update.rs` `a_reload_that_did_not_bring_the_build_is_not_repeated`.
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
        assert_eq!(tried_key("Encore"), "encore.update.tried");
    }

    /// Ported from `update.rs` `what_holds_the_page_comes_in_order`.
    #[test]
    fn what_holds_the_page_comes_in_order() {
        assert_eq!(waiting_for(false, false, false, false), None);
        assert_eq!(waiting_for(true, true, true, true), Some("It reloads when the music stops"));
        assert_eq!(waiting_for(false, true, true, true), Some("It reloads when the upload is done"));
        assert_eq!(waiting_for(false, false, true, true), Some("It reloads when the songs are saved"));
        assert_eq!(waiting_for(false, false, false, true), Some("It reloads when this dialog is closed"));
        assert_eq!(ready_text("Encore"), "A new version of Encore is ready");
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

    /// Ported from `sw.rs` `a_new_worker_is_reported_once_and_only_as_an_update`.
    #[test]
    fn a_new_worker_is_reported_once_and_only_as_an_update() {
        let mut f = Flow::new("Encore");
        assert_eq!(f.worker_state("installing", false), SwStep::Nothing);
        assert_eq!(f.worker_state("installed", false), SwStep::Nothing);
        assert!(!f.ready_seen());
        assert_eq!(f.worker_state("installing", true), SwStep::Nothing);
        assert_eq!(f.worker_state("installed", true), SwStep::Ready);
        assert!(f.ready_seen());
        assert_eq!(f.worker_state("installed", true), SwStep::Nothing);
        assert_eq!(f.worker_state("activated", true), SwStep::Nothing);
    }

    /// Ported from `sw.rs` `reload_posts_skip_waiting_then_reloads_on_controllerchange`.
    #[test]
    fn reload_posts_skip_waiting_then_reloads_on_controllerchange() {
        let mut f = Flow::new("encore");
        f.worker_state("installed", true);
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Posted);
        assert_eq!(Flow::skip_waiting_message(), json!({ "type": "skip-waiting" }));
        assert_eq!(f.controller_changed(), SwStep::Reload);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        assert_eq!(f.reload_timeout(), SwStep::Nothing);
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Nothing);
    }

    /// Ported from `sw.rs` `the_fallback_timer_reloads_when_controllerchange_never_comes`.
    #[test]
    fn the_fallback_timer_reloads_when_controllerchange_never_comes() {
        let mut f = Flow::new("encore");
        assert_eq!(f.reload_pressed(Some(true)), SwStep::Posted);
        assert_eq!(f.reload_timeout(), SwStep::Reload);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
    }

    /// Ported from `sw.rs` `controllerchange_without_reload_pressed_keeps_the_page` and
    /// `a_refused_message_reloads_at_once`.
    #[test]
    fn controllerchange_without_reload_pressed_keeps_the_page() {
        let mut f = Flow::new("encore");
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        f.worker_state("installed", true);
        assert_eq!(f.controller_changed(), SwStep::Nothing);
        assert_eq!(f.reload_pressed(None), SwStep::Reload);
        let mut g = Flow::new("encore");
        assert_eq!(g.reload_pressed(Some(false)), SwStep::Reload, "a refused message reloads at once");
    }

    /// Ported from `sw.rs` `update_checks_are_at_least_ten_minutes_apart`.
    #[test]
    fn update_checks_are_at_least_ten_minutes_apart() {
        let mut f = Flow::new("encore");
        let t0 = 1_700_000_000_000;
        assert!(f.should_check(t0));
        assert!(!f.should_check(t0 + 60_000));
        assert!(!f.should_check(t0 + SW_CHECK_EVERY_MS - 1));
        assert!(f.should_check(t0 + SW_CHECK_EVERY_MS));
        let mut g = Flow::new("encore");
        g.registered(t0);
        assert!(!g.should_check(t0 + 5 * 60_000));
        assert!(g.should_check(t0 + 11 * 60_000));
    }

    /// Ported from `sw.rs` `the_version_answer_is_remembered`.
    #[test]
    fn the_version_answer_is_remembered() {
        let mut f = Flow::new("Encore");
        assert_eq!(f.message(&json!({ "type": "other", "cache": "encore-shell-abc" })), None);
        assert_eq!(f.message(&json!({ "type": "encore:version" })), None);
        assert_eq!(f.message(&json!({ "type": "encore:version", "cache": "encore-offline-v1" })), None);
        assert_eq!(f.message(&Value::Null), None);
        assert_eq!(f.cache(), None);
        assert_eq!(
            f.message(&json!({ "type": "encore:version", "cache": "encore-shell-e56412d8e3fb9a38" })).as_deref(),
            Some("encore-shell-e56412d8e3fb9a38")
        );
        assert_eq!(f.cache(), Some("encore-shell-e56412d8e3fb9a38"));
        assert_eq!(Flow::version_question()["type"], "version?");
    }
}
