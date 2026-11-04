//! Reloading an open page into a new build of the web app by itself, without cutting off what holds it.
//!
//! - [`Watch`] compares this page's build (the hash in `<script>-<hash>.js`, the configured
//!   [`script`](crate::config::AppConfig::script) stem) with what `/api/health` names;
//! - [`Checker`] asks one check at a time: [`FIRST_CHECK_MS`] after start, every [`POLL_MS`], when the tab comes
//!   back into view (at most every [`FOCUS_GAP_MS`](kit_rules::update::FOCUS_GAP_MS)) and when the browser is
//!   online again; a server that does not answer is asked every 3 s, after two minutes every 15 s
//!   ([`down_poll_ms`](kit_rules::update::down_poll_ms));
//! - with a service worker in charge, a new build first waits for its new worker ([`new_build`], [`worker_wait`]);
//! - [`Gate`] reloads at once when nothing holds the page, else shows the update toast (`#sw-update`, drawn by
//!   [`crate::toasts`]) with the reason it waits and reloads once nothing has held the page for 5 s; a reload
//!   that did not bring the new build is not repeated ([`tried_key`]). What holds the page is the app's to say
//!   ([`UpdateConfig::hold`]); the kit itself knows only an open dialog;
//! - [`Flow`] is the service worker's side: `skip-waiting`, the reload on `controllerchange` (or after 4 s),
//!   `registration.update()` at most every 10 minutes.
//!
//! Start it once with [`start`] and, on https or localhost, [`register_worker`].

use kit_rules::update::{
    build_in, new_build, tried, tried_key, waiting_for, worker_ready, worker_wait, Act, CheckStep, Checker, Flow, Gate, Health, NewBuild,
    SwStep, Trigger, UpdateWords, Watch, WorkerReady, FIRST_CHECK_MS, HEALTH_TIMEOUT_MS, POLL_MS, RELOAD_FALLBACK_MS, SW_TIMER_MS, TICK_MS,
};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{ServiceWorker, ServiceWorkerContainer, ServiceWorkerRegistration, ServiceWorkerState};

/// One answer of `/api/health`, or None when none came (in time, or with `ok`).
pub type HealthFuture = Pin<Box<dyn Future<Output = Option<Health>>>>;

/// One thing that can hold the page: whether it does now, and the toast's second line while it does.
pub type Hold = (Rc<dyn Fn() -> bool>, &'static str);

/// How the page finds out about new builds.
#[derive(Clone)]
pub struct UpdateConfig {
    /// The app's name as people read it: the toast's words.
    pub app: String,
    /// The app's id: the storage key of the loop guard (`<id>.update.tried`) and the service worker's messages.
    pub id: String,
    /// The stem of the page's script name (`<id>-web`), whose hash is this page's build.
    pub script: String,
    /// The toast's words.
    pub words: UpdateWords,
    /// What holds the page, in order ([`UpdateConfig::hold`]); an open dialog comes after them.
    pub holds: Vec<Hold>,
    /// Asks `/api/health` (the default: [`health`]).
    pub health: Rc<dyn Fn() -> HealthFuture>,
    /// This page's build; the default reads it from the page's `<head>` ([`build_in`]).
    pub build: Option<String>,
}

impl UpdateConfig {
    /// The configured app ([`crate::config::get`]), named `app` in the toast.
    pub fn new(app: &str) -> Self {
        UpdateConfig { app: app.to_string(), ..Self::from_config() }
    }

    /// The configured app's id, name, script stem and words, nothing of the app's own holding the page.
    pub fn from_config() -> Self {
        let c = crate::config::get();
        UpdateConfig {
            app: c.app.name.clone(),
            id: c.app.id.clone(),
            script: c.script.clone(),
            words: c.words.update,
            holds: Vec::new(),
            health: Rc::new(|| Box::pin(health())),
            build: None,
        }
    }

    /// Also waits while `held()` is true, saying `words` on the toast's second line ("It reloads when the upload
    /// is done"). The first hold that holds is the one the toast names.
    pub fn hold(mut self, words: &'static str, held: impl Fn() -> bool + 'static) -> Self {
        self.holds.push((Rc::new(held), words));
        self
    }

    /// What holds the page right now, as the toast's second line says it; None when nothing does.
    pub fn held(&self) -> Option<&'static str> {
        let mut now: Vec<(bool, &'static str)> = self.holds.iter().map(|(f, w)| (f(), *w)).collect();
        now.push((dialog_open(), self.words.waits_for_dialog));
        waiting_for(&now)
    }
}

/// Whether a dialog is open (a `.modal-bg` on the page).
pub fn dialog_open() -> bool {
    crate::rt::qs(".modal-bg").is_some()
}

struct State {
    cfg: UpdateConfig,
    watch: Watch,
    gate: Gate,
    checker: Checker,
    flow: Flow,
    target: Option<String>,
    reg: Option<ServiceWorkerRegistration>,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
}

fn with<T>(f: impl FnOnce(&mut State) -> T) -> Option<T> {
    STATE.with(|s| s.borrow_mut().as_mut().map(f))
}

/// `GET /api/health` (the configured [`health`](crate::config::Paths::health) path) without the app's connection
/// events (the banner would flash); None when it does not answer within 3 s or not with `ok`.
pub async fn health() -> Option<Health> {
    let ctl = web_sys::AbortController::new().ok()?;
    let signal = ctl.signal();
    let abort = gloo_timers::callback::Timeout::new(HEALTH_TIMEOUT_MS as u32, move || ctl.abort());
    let url = {
        let c = crate::config::get();
        c.paths.full(&c.paths.health)
    };
    let req = gloo_net::http::Request::get(&url).cache(web_sys::RequestCache::NoStore).abort_signal(Some(&signal)).build().ok()?;
    let r = req.send().await.ok()?;
    drop(abort);
    if !r.ok() {
        return None;
    }
    Health::parse(&r.text().await.ok()?)
}

/// Starts watching for new builds (once, at start-up).
pub fn start(cfg: UpdateConfig) {
    let mine =
        cfg.build.clone().or_else(|| crate::rt::document().and_then(|d| d.head()).and_then(|h| build_in(&h.inner_html(), &cfg.script)));
    let flow = Flow::new(&cfg.id);
    STATE.with(|s| {
        *s.borrow_mut() =
            Some(State { cfg, watch: Watch::new(mine), gate: Gate::default(), checker: Checker::default(), flow, target: None, reg: None })
    });
    crate::rt::after(FIRST_CHECK_MS, || crate::rt::spawn(check(Trigger::Timer)));
    crate::rt::every(POLL_MS, || crate::rt::spawn(check(Trigger::Timer)));
    crate::rt::on_document("visibilitychange", |_| {
        if !crate::rt::document().is_some_and(|d| d.hidden()) {
            crate::rt::spawn(check(Trigger::Visible));
        }
    });
    crate::rt::on_window("online", |_| crate::rt::spawn(check(Trigger::Online)));
}

/// Asks the server now (as the timer does): Settings → Diagnostics after an update, or a demo.
pub fn check_now() {
    crate::rt::spawn(check(Trigger::Timer));
}

fn ask_health() -> Option<HealthFuture> {
    with(|s| (s.cfg.health)())
}

/// Asks the server once; when it does not answer, keeps asking until it does (a restart), then compares.
async fn check(trigger: Trigger) {
    let now = crate::rt::now_ms();
    if with(|s| {
        let ready = s.gate.is_ready();
        s.checker.begin(trigger, ready, now)
    }) != Some(true)
    {
        return;
    }
    loop {
        let Some(f) = ask_health() else { return };
        let h = f.await;
        let now = crate::rt::now_ms();
        let Some(step) = with(|s| {
            let seen = s.watch.seen(h.as_ref());
            s.checker.answer(seen, now)
        }) else {
            return;
        };
        match step {
            CheckStep::Done => return,
            CheckStep::AskAfter(ms) => crate::rt::sleep(ms).await,
            CheckStep::NewBuild(id) => {
                on_new_build(id).await;
                with(|s| s.checker.finish());
                return;
            }
        }
    }
}

/// The server serves build `id`: with a service worker in charge, have its new worker installed first.
async fn on_new_build(id: String) {
    if new_build(controlled(), waiting()) == NewBuild::WaitForServiceWorker {
        worker_check_now().await;
        let started = crate::rt::now_ms();
        while let Some(ms) = worker_wait(started, waiting(), crate::rt::now_ms()) {
            crate::rt::sleep(ms).await;
        }
    }
    announce(id);
}

/// A new version (build `id`) is ready: reload now, or show the toast and wait for the page to be free.
pub fn announce(id: String) {
    let Some(cfg) = with(|s| s.cfg.clone()) else { return };
    let stored = crate::rt::session_storage().and_then(|s| s.get_item(&tried_key(&cfg.id)).ok().flatten());
    let was_tried = tried(stored.as_deref(), &id);
    let held = cfg.held();
    let now = crate::rt::now_ms();
    let act = with(|s| {
        s.target = Some(id);
        s.gate.ready(held.is_some(), was_tried, now)
    });
    match act {
        Some(Act::Reload) => reload(),
        Some(Act::ShowToast) => {
            crate::toasts::show_update(
                &cfg.words.ready(&cfg.app),
                cfg.words.reload,
                Rc::new(|| {
                    if with(|s| s.gate.pressed()) == Some(Act::Reload) {
                        reload();
                    }
                }),
            );
            if with(|s| s.gate.is_manual()) == Some(false) {
                crate::toasts::set_update_reason(held);
            }
            crate::rt::every(TICK_MS, move || {
                let held = cfg.held();
                if with(|s| s.gate.is_manual()) == Some(false) {
                    crate::toasts::set_update_reason(held);
                }
                let now = crate::rt::now_ms();
                if with(|s| s.gate.tick(held.is_some(), now)) == Some(Act::Reload) {
                    reload();
                }
            });
        }
        _ => {}
    }
}

/// Reloads into the new version, remembering which build it was for (the loop guard).
fn reload() {
    if let Some((app_id, Some(id))) = with(|s| (s.cfg.id.clone(), s.target.clone().filter(|i| !i.is_empty()))) {
        if let Some(st) = crate::rt::session_storage() {
            let _ = st.set_item(&tried_key(&app_id), &id);
        }
    }
    apply_update();
}

// ---- the service worker ----------------------------------------------------------------------------

/// `navigator.serviceWorker`, when this browser has it here.
fn container() -> Option<ServiceWorkerContainer> {
    let nav = crate::rt::window()?.navigator();
    let v = js_sys::Reflect::get(&nav, &JsValue::from_str("serviceWorker")).ok()?;
    if v.is_undefined() || v.is_null() {
        return None;
    }
    v.dyn_into::<ServiceWorkerContainer>().ok()
}

/// Whether a service worker controls this page.
pub fn controlled() -> bool {
    container().is_some_and(|c| c.controller().is_some())
}

/// Whether a new worker has installed and waits to take over.
pub fn waiting() -> bool {
    with(|s| s.reg.as_ref().is_some_and(|r| r.waiting().is_some())).unwrap_or(false)
}

/// The shell cache the worker serves this page from (`<id>-shell-<hash>`), once it has said so.
pub fn shell_cache() -> Option<String> {
    with(|s| s.flow.cache().map(str::to_string)).flatten()
}

fn post(w: &ServiceWorker, msg: &serde_json::Value) -> bool {
    // a plain JS object, as the worker reads `d.type`
    let Ok(js) = js_sys::JSON::parse(&msg.to_string()) else { return false };
    w.post_message(&js).is_ok()
}

fn state_name(s: ServiceWorkerState) -> &'static str {
    match s {
        ServiceWorkerState::Parsed => "parsed",
        ServiceWorkerState::Installing => "installing",
        ServiceWorkerState::Installed => "installed",
        ServiceWorkerState::Activating => "activating",
        ServiceWorkerState::Activated => "activated",
        ServiceWorkerState::Redundant => "redundant",
        _ => "",
    }
}

fn apply(step: Option<SwStep>) {
    match step {
        Some(SwStep::Ready) => on_worker_ready(),
        Some(SwStep::Reload) => crate::rt::reload(),
        Some(SwStep::Posted) => crate::rt::after(RELOAD_FALLBACK_MS, || apply(with(|s| s.flow.reload_timeout()))),
        _ => {}
    }
}

/// A new worker waits: a reload when the server serves another build than this page's, a quiet takeover when it
/// already runs it.
fn on_worker_ready() {
    crate::rt::spawn(async {
        let Some(f) = ask_health() else { return };
        let h = f.await;
        let Some(seen) = with(|s| s.watch.seen(h.as_ref())) else { return };
        match worker_ready(seen) {
            WorkerReady::ActivateQuietly => activate_quietly(),
            WorkerReady::Announce(id) => announce(id),
        }
    });
}

/// Reloads into the new version: through the waiting worker when there is one, otherwise `location.reload()`.
pub fn apply_update() {
    let waiting = with(|s| s.reg.as_ref().and_then(|r| r.waiting())).flatten();
    let posted = waiting.as_ref().map(|w| post(w, &Flow::skip_waiting_message()));
    let step = with(|s| s.flow.reload_pressed(posted));
    if step.is_none() {
        crate::rt::reload();
    }
    apply(step);
}

/// Lets a waiting worker take over without reloading this page.
pub fn activate_quietly() {
    if let Some(w) = with(|s| s.reg.as_ref().and_then(|r| r.waiting())).flatten() {
        let _ = post(&w, &Flow::skip_waiting_message());
    }
}

/// `registration.update()` now, past the throttle, and waits for the browser to have fetched the worker script.
async fn worker_check_now() {
    let now = crate::rt::now_ms();
    with(|s| s.flow.registered(now));
    let Some(reg) = with(|s| s.reg.clone()).flatten() else { return };
    if let Ok(p) = reg.update() {
        let _ = wasm_bindgen_futures::JsFuture::from(p).await;
    }
}

fn watch_worker(w: ServiceWorker) {
    let w2 = w.clone();
    crate::rt::listen_forever(&w, "statechange", move |_| {
        let has_controller = controlled();
        apply(with(|s| s.flow.worker_state(state_name(w2.state()), has_controller)));
    });
}

fn check_for_update() {
    let now = crate::rt::now_ms();
    if with(|s| s.flow.should_check(now)) != Some(true) {
        return;
    }
    let Some(reg) = with(|s| s.reg.clone()).flatten() else { return };
    if let Ok(p) = reg.update() {
        crate::rt::spawn(async move {
            let _ = wasm_bindgen_futures::JsFuture::from(p).await;
        });
    }
}

fn ask_version(c: &ServiceWorkerContainer) {
    if let Some(ctrl) = c.controller() {
        let _ = post(&ctrl, &Flow::version_question());
    }
}

/// Registers the service worker at `url` (`/sw.js`, the configured [`worker`](crate::config::Paths::worker)) and
/// follows its updates; call after [`start`], on https or localhost.
pub fn register_worker(url: &str) {
    let Some(c) = container() else { return };
    crate::rt::listen_forever(&c, "controllerchange", |_| {
        apply(with(|s| s.flow.controller_changed()));
        if let Some(c) = container() {
            ask_version(&c);
        }
    });
    crate::rt::listen_forever(&c, "message", |e| {
        let Some(me) = e.dyn_ref::<web_sys::MessageEvent>() else { return };
        let data = js_sys::JSON::stringify(&me.data())
            .ok()
            .and_then(|s| s.as_string())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(serde_json::Value::Null);
        with(|s| s.flow.message(&data));
    });
    crate::rt::on_document("visibilitychange", |_| {
        if !crate::rt::document().is_some_and(|d| d.hidden()) {
            check_for_update();
        }
    });
    crate::rt::every(SW_TIMER_MS, || {
        if !crate::rt::document().is_some_and(|d| d.hidden()) {
            check_for_update();
        }
    });
    let promise = c.register(url);
    crate::rt::spawn(async move {
        let Ok(reg) = wasm_bindgen_futures::JsFuture::from(promise).await else { return };
        let Ok(reg) = reg.dyn_into::<ServiceWorkerRegistration>() else { return };
        let now = crate::rt::now_ms();
        with(|s| {
            s.flow.registered(now);
            s.reg = Some(reg.clone());
        });
        // a worker that installed during an earlier visit and still waits
        if reg.waiting().is_some() {
            let has = c.controller().is_some();
            apply(with(|s| s.flow.worker_state("installed", has)));
        }
        if let Some(w) = reg.installing() {
            watch_worker(w);
        }
        let reg2 = reg.clone();
        crate::rt::listen_forever(&reg, "updatefound", move |_| {
            if let Some(w) = reg2.installing() {
                watch_worker(w);
            }
        });
        ask_version(&c);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_name_the_configured_app() {
        let cfg = UpdateConfig::from_config();
        assert_eq!((cfg.app.as_str(), cfg.id.as_str(), cfg.script.as_str()), ("App", "app", "app-web"));
        // off the browser no dialog is open, so nothing holds the page
        assert_eq!(cfg.held(), None);
        assert_eq!(cfg.words.ready(&cfg.app), "A new version of App is ready");
        assert_eq!(tried_key(&cfg.id), "app.update.tried");
        let named = UpdateConfig::new("Kit Demo");
        assert_eq!(named.words.ready(&named.app), "A new version of Kit Demo is ready");
    }

    #[test]
    fn the_app_says_what_holds_the_page() {
        let busy = Rc::new(std::cell::Cell::new(false));
        let b = busy.clone();
        let cfg = UpdateConfig::from_config()
            .hold("It reloads when the upload is done", move || b.get())
            .hold("It reloads when the call ends", || true);
        assert_eq!(cfg.held(), Some("It reloads when the call ends"));
        busy.set(true);
        assert_eq!(cfg.held(), Some("It reloads when the upload is done"), "the first hold that holds is named");
    }
}
