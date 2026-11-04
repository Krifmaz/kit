//! The browser glue the components share: the clock, timers, tasks, the document and event listeners.
//!
//! Encore's `web-rs/src/dom.rs` holds the same helpers for its hand-written pages. Off the browser (the markup
//! tests run natively) the clock reads the system time and timers and listeners do nothing, so a component can
//! be rendered to a string without a window.

use std::cell::RefCell;
use std::future::Future;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, EventTarget, Window};

/// Whether this runs in a browser (wasm32): timers, listeners and the DOM exist.
pub const IN_BROWSER: bool = cfg!(target_arch = "wasm32");

/// Milliseconds since the epoch (`Date.now()`).
pub fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
    }
}

/// Runs `f` once after `ms` (a `setTimeout` that is never cancelled). Nothing off the browser.
pub fn after(ms: u64, f: impl FnOnce() + 'static) {
    if IN_BROWSER {
        gloo_timers::callback::Timeout::new(ms.min(u32::MAX as u64) as u32, f).forget();
    } else {
        drop(f);
    }
}

/// Runs `f` every `ms` for the rest of the page's life. Nothing off the browser.
pub fn every(ms: u64, f: impl FnMut() + 'static) {
    if IN_BROWSER {
        gloo_timers::callback::Interval::new(ms.min(u32::MAX as u64) as u32, f).forget();
    } else {
        drop(f);
    }
}

/// Waits `ms` (off the browser it returns at once).
pub async fn sleep(ms: u64) {
    if IN_BROWSER {
        gloo_timers::future::TimeoutFuture::new(ms.min(u32::MAX as u64) as u32).await;
    }
}

/// Runs a task on the page (`spawn_local`). Nothing off the browser.
pub fn spawn(f: impl Future<Output = ()> + 'static) {
    if IN_BROWSER {
        wasm_bindgen_futures::spawn_local(f);
    } else {
        drop(f);
    }
}

/// The window, in the browser.
pub fn window() -> Option<Window> {
    if IN_BROWSER {
        web_sys::window()
    } else {
        None
    }
}

/// The document, in the browser.
pub fn document() -> Option<Document> {
    window().and_then(|w| w.document())
}

/// `document.getElementById(id)`.
pub fn by_id(id: &str) -> Option<Element> {
    document()?.get_element_by_id(id)
}

/// `document.querySelector(sel)`.
pub fn qs(sel: &str) -> Option<Element> {
    document()?.query_selector(sel).ok().flatten()
}

/// `root.querySelector(sel)`.
pub fn qs_in(root: &Element, sel: &str) -> Option<Element> {
    root.query_selector(sel).ok().flatten()
}

/// `root.querySelectorAll(sel)` as elements.
pub fn qsa_in(root: &Element, sel: &str) -> Vec<Element> {
    let Ok(list) = root.query_selector_all(sel) else { return vec![] };
    (0..list.length()).filter_map(|i| list.get(i)?.dyn_into::<Element>().ok()).collect()
}

/// `document.querySelectorAll(sel)` as elements.
pub fn qsa(sel: &str) -> Vec<Element> {
    let Some(doc) = document() else { return vec![] };
    let Ok(list) = doc.query_selector_all(sel) else { return vec![] };
    (0..list.length()).filter_map(|i| list.get(i)?.dyn_into::<Element>().ok()).collect()
}

/// An event listener that is removed when dropped.
pub struct Listener {
    target: EventTarget,
    kind: String,
    f: Closure<dyn FnMut(Event)>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback(&self.kind, self.f.as_ref().unchecked_ref());
    }
}

/// Listens for `kind` on `target` until the returned [`Listener`] is dropped.
pub fn listen(target: &EventTarget, kind: &str, f: impl FnMut(&Event) + 'static) -> Listener {
    let mut f = f;
    let f = Closure::<dyn FnMut(Event)>::new(move |e: Event| f(&e));
    let _ = target.add_event_listener_with_callback(kind, f.as_ref().unchecked_ref());
    Listener { target: target.clone(), kind: kind.to_string(), f }
}

/// Listens for `kind` on `target` for the rest of the page's life.
pub fn listen_forever(target: &EventTarget, kind: &str, f: impl FnMut(&Event) + 'static) {
    std::mem::forget(listen(target, kind, f));
}

/// Listens for `kind` on the document for the rest of the page's life (nothing off the browser).
pub fn on_document(kind: &str, f: impl FnMut(&Event) + 'static) {
    if let Some(d) = document() {
        listen_forever(&d, kind, f);
    }
}

/// Listens for `kind` on the window for the rest of the page's life (nothing off the browser).
pub fn on_window(kind: &str, f: impl FnMut(&Event) + 'static) {
    if let Some(w) = window() {
        listen_forever(&w, kind, f);
    }
}

/// `window.dispatchEvent(new CustomEvent(name))`: the app-wide events Encore's pages listen for
/// (`encore:bugs-changed`, …).
pub fn dispatch(name: &str) {
    if let (Some(w), Ok(e)) = (window(), web_sys::CustomEvent::new(name)) {
        let _ = w.dispatch_event(&e);
    }
}

/// The element an event happened on.
pub fn target_el(e: &Event) -> Option<Element> {
    e.target()?.dyn_into::<Element>().ok()
}

/// `event.target.closest(sel)`.
pub fn closest(e: &Event, sel: &str) -> Option<Element> {
    target_el(e)?.closest(sel).ok().flatten()
}

/// `location.reload()`.
pub fn reload() {
    if let Some(w) = window() {
        let _ = w.location().reload();
    }
}

/// `sessionStorage`, where the browser allows it.
pub fn session_storage() -> Option<web_sys::Storage> {
    window()?.session_storage().ok().flatten()
}

/// Mounts `view` (a Leptos view) as the last child of `parent` and keeps it until the returned handle is dropped.
/// Off the browser nothing is mounted.
pub fn mount<V: leptos::prelude::IntoView + 'static>(parent: &Element, view: impl FnOnce() -> V + 'static) -> Option<Mounted> {
    if !IN_BROWSER {
        return None;
    }
    let parent: web_sys::HtmlElement = parent.clone().dyn_into().ok()?;
    let handle = leptos::mount::mount_to(parent, view);
    Some(Mounted(Some(Box::new(handle))))
}

/// A view mounted by [`mount`]: dropping it takes the view out of the page.
pub struct Mounted(Option<Box<dyn std::any::Any>>);

impl Mounted {
    /// Keeps the view on the page for the rest of its life.
    pub fn forever(mut self) {
        if let Some(h) = self.0.take() {
            std::mem::forget(h);
        }
    }
}

thread_local! {
    /// Views mounted for the rest of the page's life, kept by name so mounting twice replaces the first.
    static KEPT: RefCell<Vec<(String, Mounted)>> = const { RefCell::new(Vec::new()) };
}

/// Keeps `m` under `name` (a second one with the same name takes the first one's place).
pub fn keep(name: &str, m: Option<Mounted>) {
    let Some(m) = m else { return };
    KEPT.with(|k| {
        let mut k = k.borrow_mut();
        k.retain(|(n, _)| n != name);
        k.push((name.to_string(), m));
    });
}

/// Takes the view kept under `name` off the page (nothing when there is none).
pub fn unkeep(name: &str) {
    let gone = KEPT.with(|k| {
        let mut k = k.borrow_mut();
        let i = k.iter().position(|(n, _)| n == name)?;
        Some(k.remove(i))
    });
    drop(gone);
}
