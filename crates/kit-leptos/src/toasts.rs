//! Toasts: `#toasts` and every toast in it, with a global API any code can call (Leptos or hand-written).
//!
//! - `<div class="toast ok">Saved</div>` (any kind but a staying error);
//! - `<div class="toast err stays" role="alert" data-msg="…">` with [`error_toast_html`] inside;
//! - `<div class="toast act">` with the text and one `data-toast-action` button;
//! - `<div {..()} id="sw-update" class="toast" role="status" …>`, the update toast ([`show_update`]).
//!
//! A Leptos app draws [`Toasts`] (`<div id="toasts">`); a page that already has an empty `#toasts` in its
//! index.html calls [`mount`]. Either way [`toast`], [`toast_action`] and [`show`] work from any code.
//!
//! How they stack and go is the configured [`ToastPolicy`] ([`crate::config::AppConfig::toast_policy`], the
//! web's by default: three errors stay, the same error again is shown once, 4.2 s with a fade), and Copy, Close
//! and the failed-copy words are the configured [`ToastWords`](kit_rules::toasts::ToastWords).

use crate::clipboard::copy_text;
use crate::html::{esc, html};
use crate::icons::icon;
use kit_rules::toasts::{Toast, ToastKind, ToastPolicy, ToastStack};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// One toast as it is drawn.
#[derive(Clone, Debug)]
pub struct Entry {
    /// Its id in the stack.
    pub id: u64,
    /// The words, as given.
    pub text: String,
    /// Its kind (the class after `toast`).
    pub kind: ToastKind,
    /// An error that stays until closed (Copy and Close).
    pub stays: bool,
    /// The action's label, for a toast with one button.
    pub action: Option<String>,
    /// The fade before it goes has begun.
    pub fading: ArcRwSignal<bool>,
}

/// The update toast while it is on screen.
#[derive(Clone, Debug)]
pub struct UpdateEntry {
    /// "A new version of Kit Demo is ready".
    pub text: String,
    /// "Reload".
    pub label: String,
    /// The second line: what the reload waits for ("" once nothing holds the page).
    pub why: ArcRwSignal<String>,
    /// Reload was pressed (the button is switched off).
    pub pressed: ArcRwSignal<bool>,
    /// The newest toast it came after: toasts that arrived later are drawn under it, in arrival order.
    pub after_seq: u64,
}

/// One line of `#toasts`.
#[derive(Clone, Debug)]
pub enum Item {
    /// A toast of the stack.
    Toast(Entry),
    /// The update toast.
    Update(UpdateEntry),
}

impl Item {
    fn key(&self) -> u64 {
        match self {
            Item::Toast(e) => e.id,
            Item::Update(_) => u64::MAX,
        }
    }
}

struct State {
    stack: ToastStack<u64>,
    actions: HashMap<u64, Box<dyn FnOnce()>>,
    fading: HashMap<u64, ArcRwSignal<bool>>,
    update: Option<UpdateEntry>,
    on_reload: Option<Rc<dyn Fn()>>,
}

/// `#toasts` and the toast each view mounted in it shows.
type Host = (web_sys::Element, Vec<(u64, crate::rt::Mounted)>);

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        stack: fresh_stack(),
        actions: HashMap::new(),
        fading: HashMap::new(),
        update: None,
        on_reload: None,
    });
    static ITEMS: ArcRwSignal<Vec<Item>> = ArcRwSignal::new(Vec::new());
    /// The `#toasts` that [`mount`] draws into, and the toast each mounted view shows.
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// An empty stack under the configured policy and words.
fn fresh_stack() -> ToastStack<u64> {
    let c = crate::config::get();
    let policy: ToastPolicy = c.toast_policy;
    ToastStack::new(policy).with_copy_failed(c.words.toasts.copy_failed)
}

/// Takes the configured policy and words (from [`crate::config::set`]); the toasts on screen go.
pub(crate) fn configure() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.stack = fresh_stack();
        s.actions.clear();
    });
    publish();
}

/// The lines of `#toasts`, for a view of one's own.
pub fn items() -> ArcRwSignal<Vec<Item>> {
    ITEMS.with(Clone::clone)
}

fn entry(t: &Toast<u64>, fading: &mut HashMap<u64, ArcRwSignal<bool>>) -> Entry {
    Entry {
        id: t.id,
        text: t.text.clone(),
        kind: t.kind,
        stays: t.stays(),
        action: t.action.as_ref().map(|a| a.label.clone()),
        fading: fading.entry(t.id).or_insert_with(|| ArcRwSignal::new(false)).clone(),
    }
}

/// Draws the stack again after a change.
fn publish() {
    let list = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let s = &mut *s;
        let live: Vec<u64> = s.stack.items().iter().map(|t| t.id).collect();
        s.fading.retain(|id, _| live.contains(id));
        s.actions.retain(|id, _| live.contains(id));
        let mut list: Vec<Item> = Vec::new();
        let mut update = s.update.clone();
        for t in s.stack.items() {
            if let Some(u) = update.as_ref().filter(|u| t.seq > u.after_seq) {
                list.push(Item::Update(u.clone()));
                update = None;
            }
            list.push(Item::Toast(entry(t, &mut s.fading)));
        }
        if let Some(u) = update {
            list.push(Item::Update(u));
        }
        list
    });
    sync(&list);
    ITEMS.with(|i| i.set(list));
}

/// Mounts each new toast on its own at the end of `#toasts` and takes the gone ones out, so the container holds
/// only toast elements (no list markers) and a toast another part of the app appends by hand stays in arrival
/// order.
fn sync(list: &[Item]) {
    HOST.with(|h| {
        let mut h = h.borrow_mut();
        let Some((el, mounted)) = h.as_mut() else { return };
        mounted.retain(|(k, _)| list.iter().any(|i| i.key() == *k));
        for item in list {
            let key = item.key();
            if mounted.iter().any(|(k, _)| *k == key) {
                continue;
            }
            let item = item.clone();
            let m = crate::rt::mount(el, move || match item {
                Item::Toast(e) => view! { <ToastView entry=e/> }.into_any(),
                Item::Update(u) => view! { <UpdateToastView entry=u/> }.into_any(),
            });
            if let Some(m) = m {
                mounted.push((key, m));
            }
        }
    });
}

/// Arms the fade and the end of a toast that goes by itself.
fn arm(id: u64) {
    let Some((seq, fade_at, end_at)) = STATE.with(|s| {
        let s = s.borrow();
        let t = s.stack.items().iter().find(|t| t.id == id)?;
        Some((t.seq, t.fades_at(s.stack.policy())?, t.expires_at()?))
    }) else {
        return;
    };
    let now = crate::rt::now_ms();
    crate::rt::after(fade_at.saturating_sub(now), move || {
        if let Some(f) = STATE.with(|s| s.borrow().fading.get(&id).cloned()) {
            f.set(true);
        }
    });
    crate::rt::after(end_at.saturating_sub(now), move || {
        STATE.with(|s| s.borrow_mut().stack.expire(id, seq));
        publish();
    });
}

/// `toast(msg, kind)` with the kind as a word (`""`, `info`, `ok`, `warn`, `err`).
pub fn toast(msg: &str, kind: &str) {
    show(msg, ToastKind::parse(kind));
}

/// A toast of `kind`. An error stays with Copy and Close (except the failed-copy words); the same error again
/// while it is on screen is shown once.
pub fn show(msg: &str, kind: ToastKind) {
    let id = STATE.with(|s| s.borrow_mut().stack.show(msg, kind, crate::rt::now_ms()));
    publish();
    if let Some(id) = id {
        arm(id);
    }
}

/// A toast with one button (`label`, "Undo") that stays `ms`: a press runs `f` once and the toast goes.
pub fn toast_action(msg: &str, label: &str, ms: u32, f: impl FnOnce() + 'static) {
    let id = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let id = s.stack.show_action(msg, label, 0, u64::from(ms), crate::rt::now_ms())?;
        s.actions.insert(id, Box::new(f));
        Some(id)
    });
    publish();
    if let Some(id) = id {
        arm(id);
    }
}

/// Closes toast `id` (its Close).
pub fn close(id: u64) {
    STATE.with(|s| s.borrow_mut().stack.close(id));
    publish();
}

/// Presses the button of toast `id`: it goes and its action runs once.
pub fn press(id: u64) {
    let f = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.stack.take_action(id)?;
        s.actions.remove(&id)
    });
    publish();
    if let Some(f) = f {
        f();
    }
}

/// Takes every toast away (a new page after signing out).
pub fn clear() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.stack = fresh_stack();
        s.actions.clear();
    });
    publish();
}

/// Shows the update toast ("A new version of Kit Demo is ready", the reason it waits, `label`): it stays until the
/// page reloads. `on_reload` runs when its button is pressed (the button is switched off then). Returns false
/// when it is already on screen.
pub fn show_update(text: &str, label: &str, on_reload: Rc<dyn Fn()>) -> bool {
    let shown = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.update.is_some() {
            return false;
        }
        let after_seq = s.stack.newest();
        s.update = Some(UpdateEntry {
            text: text.to_string(),
            label: label.to_string(),
            why: ArcRwSignal::new(String::new()),
            pressed: ArcRwSignal::new(false),
            after_seq,
        });
        s.on_reload = Some(on_reload);
        true
    });
    if shown {
        publish();
    }
    shown
}

/// The update toast's second line: what the reload waits for (`None` once nothing holds the page).
pub fn set_update_reason(why: Option<&str>) {
    if let Some(u) = STATE.with(|s| s.borrow().update.clone()) {
        let text = why.unwrap_or("").to_string();
        if u.why.get_untracked() != text {
            u.why.set(text);
        }
    }
}

/// Whether the update toast is on screen.
pub fn update_shown() -> bool {
    STATE.with(|s| s.borrow().update.is_some())
}

fn press_update() {
    let Some((u, f)) = STATE.with(|s| {
        let s = s.borrow();
        Some((s.update.clone()?, s.on_reload.clone()?))
    }) else {
        return;
    };
    u.pressed.set(true);
    f();
}

/// The error toast's inside: the whole message, Copy and Close.
pub fn error_toast_html(msg: &str) -> String {
    let w = crate::config::get().words.toasts;
    let (copy_w, close_w) = (w.copy, w.close);
    format!(
        r#"<div class="toast-text">{m}</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-copy-text="{m}">{copy}{copy_w}</button><button type="button" class="btn sm ghost icon" data-toast-close title="{close_w}" aria-label="{close_w}">{x}</button></div>"#,
        m = esc(msg),
        copy = icon("copy"),
        x = icon("x")
    )
}

/// One toast.
#[component]
pub fn ToastView(
    /// The toast.
    entry: Entry,
) -> impl IntoView {
    let id = entry.id;
    // the fade sets `opacity`, then `transition`, on the element's style (`style="opacity: 0; transition:
    // opacity 0.3s;"`); before it the toast has no `style` at all
    let (f1, f2) = (entry.fading.clone(), entry.fading.clone());
    let opacity = move || f1.get().then_some("0");
    let transition = move || f2.get().then_some("opacity .3s");
    if entry.stays {
        let w = crate::config::get().words.toasts;
        let (copy_w, close_w) = (w.copy, w.close);
        let msg = entry.text.clone();
        let copy = msg.clone();
        return view! {
            <div {..()} class="toast err stays" role="alert" data-msg=msg.clone()>
                <div class="toast-text">{msg.clone()}</div>
                <div class="toast-acts">
                    <button {..()} type="button" class="btn sm ghost" data-copy-text=msg.clone()
                        on:click=move |e| { e.stop_propagation(); copy_text(&copy); }
                        {..html(format!("{}{copy_w}", icon("copy")))}>{""}</button>
                    <button {..()} type="button" class="btn sm ghost icon" data-toast-close title=close_w aria-label=close_w
                        on:click=move |e| { e.stop_propagation(); close(id); }
                        {..html(icon("x"))}>{""}</button>
                </div>
            </div>
        }
        .into_any();
    }
    if let Some(label) = entry.action.clone() {
        return view! {
            <div {..()} class="toast act" style:opacity=opacity style:transition=transition>
                <div class="toast-text">{entry.text.clone()}</div>
                <div class="toast-acts">
                    <button {..()} type="button" class="btn sm ghost" data-toast-action on:click=move |_| press(id)>{label}</button>
                </div>
            </div>
        }
        .into_any();
    }
    view! { <div {..()} class=format!("toast {}", entry.kind.as_str()) style:opacity=opacity style:transition=transition>{entry.text.clone()}</div> }.into_any()
}

/// The update toast.
#[component]
pub fn UpdateToastView(
    /// The update toast.
    entry: UpdateEntry,
) -> impl IntoView {
    let why = entry.why.clone();
    let pressed = entry.pressed.clone();
    view! {
        <div {..()} id="sw-update" class="toast" role="status" style="display:flex;align-items:center;gap:12px">
            <span class="grow">{entry.text.clone()}<span {..()} class="small muted" data-update-why style="display:block">{move || why.get()}</span></span>
            <button {..()} class="btn sm primary" type="button" disabled=move || pressed.get() on:click=move |_| press_update()>{entry.label.clone()}</button>
        </div>
    }
}

/// The toasts, without their container, as one reactive list (for a view of one's own; [`mount`] draws each toast
/// on its own instead).
#[component]
pub fn ToastList() -> impl IntoView {
    let list = items();
    view! {
        <For each=move || list.get() key=Item::key let:item>
            {match item {
                Item::Toast(e) => view! { <ToastView entry=e/> }.into_any(),
                Item::Update(u) => view! { <UpdateToastView entry=u/> }.into_any(),
            }}
        </For>
    }
}

/// `<div id="toasts">` with every toast in it.
#[component]
pub fn Toasts() -> impl IntoView {
    view! { <div id="toasts"><ToastList/></div> }
}

/// Draws the toasts into the page's existing `#toasts` (an index.html with its own). Call once at start-up.
/// Each toast is mounted on its own and appended.
pub fn mount() {
    if let Some(el) = crate::rt::by_id("toasts") {
        HOST.with(|h| *h.borrow_mut() = Some((el, Vec::new())));
        let list = ITEMS.with(|i| i.get_untracked());
        sync(&list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_same_html;
    use kit_rules::toasts::COPY_FAILED;

    fn drawn() -> String {
        crate::testing::render(|| view! { <Toasts/> })
    }

    const COPY_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"#;
    const X_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>"#;

    /// The kit's markup of an error toast with `text` (already escaped) and the words Copy and Close.
    fn error(text: &str) -> String {
        format!(
            r#"<div class="toast err stays" role="alert" data-msg="{text}"><div class="toast-text">{text}</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-copy-text="{text}">{COPY_SVG}Copy</button><button type="button" class="btn sm ghost icon" data-toast-close title="Close" aria-label="Close">{X_SVG}</button></div></div>"#
        )
    }

    #[test]
    fn each_kind_draws_its_markup() {
        clear();
        toast("Saved", "ok");
        toast("upload failed: <a.png> & \"b\"", "err");
        toast(COPY_FAILED, "err");
        toast("Some files were left out", "");
        let want = format!(
            r#"<div id="toasts"><div class="toast ok">Saved</div>{}<div class="toast err">{COPY_FAILED}</div><div class="toast ">Some files were left out</div></div>"#,
            error("upload failed: &lt;a.png&gt; &amp; &quot;b&quot;")
        );
        assert_same_html(&drawn(), &want);
        // the string builder for hand-written pages draws the same inside
        assert_same_html(
            &format!(r#"<div class="toast err stays" role="alert" data-msg="x">{}</div>"#, error_toast_html("x")),
            &error("x"),
        );
    }

    #[test]
    fn three_errors_stay_and_the_same_one_is_shown_once() {
        clear();
        for m in ["a", "b", "a", "c", "d"] {
            toast(m, "err");
        }
        let want = format!(r#"<div id="toasts">{}{}{}</div>"#, error("b"), error("c"), error("d"));
        assert_same_html(&drawn(), &want);
    }

    #[test]
    fn an_action_toast_runs_once_and_goes() {
        clear();
        let ran = Rc::new(std::cell::Cell::new(0));
        let r = ran.clone();
        toast_action("Note deleted", "Undo", 6000, move || r.set(r.get() + 1));
        let want = r#"<div id="toasts"><div class="toast act"><div class="toast-text">Note deleted</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-toast-action>Undo</button></div></div></div>"#;
        assert_same_html(&drawn(), want);
        let id = STATE.with(|s| s.borrow().stack.items()[0].id);
        press(id);
        press(id);
        assert_eq!(ran.get(), 1);
        assert_same_html(&drawn(), r#"<div id="toasts"></div>"#);
    }

    #[test]
    fn the_update_toast_sits_where_it_arrived() {
        clear();
        STATE.with(|s| s.borrow_mut().update = None);
        toast("before", "ok");
        assert!(show_update("A new version of Kit Demo is ready", "Reload", Rc::new(|| {})));
        assert!(!show_update("A new version of Kit Demo is ready", "Reload", Rc::new(|| {})));
        toast("after", "ok");
        set_update_reason(Some("It reloads when this dialog is closed"));
        let update = r#"<div id="sw-update" class="toast" role="status" style="display:flex;align-items:center;gap:12px"><span class="grow">A new version of Kit Demo is ready<span class="small muted" data-update-why style="display:block">It reloads when this dialog is closed</span></span><button class="btn sm primary" type="button">Reload</button></div>"#;
        let want = format!(r#"<div id="toasts"><div class="toast ok">before</div>{update}<div class="toast ok">after</div></div>"#);
        assert_same_html(&drawn(), &want);
        STATE.with(|s| s.borrow_mut().update = None);
    }

    #[test]
    fn an_app_words_and_stacks_toasts_its_own_way() {
        let mut cfg = crate::config::AppConfig::default();
        cfg.words.toasts.copy = "Kopieren";
        cfg.words.toasts.close = "Schließen";
        cfg.words.toasts.copy_failed = "Kopieren fehlgeschlagen";
        cfg.toast_policy = ToastPolicy::desktop();
        crate::config::set(cfg);
        toast("Kopieren fehlgeschlagen", "err");
        toast("kaputt", "err");
        let html = drawn();
        assert!(html.contains(r#"<div class="toast err">Kopieren fehlgeschlagen</div>"#), "{html}");
        assert!(html.contains("Kopieren</button>") && html.contains(r#"title="Schließen""#), "{html}");
        for i in 0..9 {
            toast(&format!("n{i}"), "ok");
        }
        let count = STATE.with(|s| s.borrow().stack.items().len());
        assert_eq!(count, kit_rules::toasts::DESKTOP_MAX_TOASTS, "the desktop's cap");
        crate::config::set(crate::config::AppConfig::default());
    }
}
