//! Toasts: `#toasts` and every toast in it, with a global API that hand-written pages call as they called
//! Encore's `ui::toast`.
//!
//! From Encore's `web-rs/src/ui.rs` (`toast`, `toast_action`, `error_toast`, `error_toast_html`) and
//! `web-rs/src/update.rs` (`show_toast`, the `#sw-update` toast). The rules are kit-rules'
//! [`ToastStack`] under [`ToastPolicy::web`]: an error (`err`) stays until closed and has Copy and Close, at most
//! three errors stay (a new one pushes out the oldest), the same error again is dropped while it is on screen,
//! any other toast fades out from 3.8 s and goes at 4.2 s, an action toast (Undo) stays its own time and goes
//! when its button is pressed. The markup is Encore's:
//!
//! - `<div class="toast ok">Saved</div>` (any kind but a staying error);
//! - `<div class="toast err stays" role="alert" data-msg="…">` with [`error_toast_html`] inside;
//! - `<div class="toast act">` with the text and one `data-toast-action` button;
//! - `<div {..()} id="sw-update" class="toast" role="status" …>`, the update toast ([`show_update`]).
//!
//! A Leptos app draws [`Toasts`] (`<div id="toasts">`); a page that already has an empty `#toasts` (Encore's
//! index.html) calls [`mount`]. Either way [`toast`], [`toast_action`] and [`show`] work from any code.

use crate::clipboard::copy_text;
use crate::html::{esc, html};
use crate::icons::icon;
use kit_rules::toasts::{Toast, ToastKind, ToastPolicy, ToastStack, CLOSE, COPY};
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
    /// "A new version of Encore is ready".
    pub text: String,
    /// "Reload".
    pub label: String,
    /// The second line: what the reload waits for ("" once nothing holds the page).
    pub why: ArcRwSignal<String>,
    /// Reload was pressed (the button is switched off).
    pub pressed: ArcRwSignal<bool>,
    /// The newest toast it came after: toasts that arrived later are drawn under it, as Encore appends them.
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
        stack: ToastStack::new(ToastPolicy::web()),
        actions: HashMap::new(),
        fading: HashMap::new(),
        update: None,
        on_reload: None,
    });
    static ITEMS: ArcRwSignal<Vec<Item>> = ArcRwSignal::new(Vec::new());
    /// The `#toasts` that [`mount`] draws into, and the toast each mounted view shows.
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
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
/// only Encore's toast elements (no list markers) and a toast another part of the app appends by hand stays in
/// arrival order, as with Encore's `appendChild`.
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

/// `toast(msg, kind)` with the kind as Encore names it (`""`, `info`, `ok`, `warn`, `err`).
pub fn toast(msg: &str, kind: &str) {
    show(msg, ToastKind::parse(kind));
}

/// A toast of `kind`. An error stays with Copy and Close (except
/// [`COPY_FAILED`](kit_rules::toasts::COPY_FAILED)); the same error again while it is on screen is shown once.
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
        s.stack = ToastStack::new(ToastPolicy::web());
        s.actions.clear();
    });
    publish();
}

/// Shows the update toast ("A new version of Encore is ready", the reason it waits, `label`): it stays until the
/// page reloads. `on_reload` runs when its button is pressed (the button is switched off then). Returns false
/// when it is already on screen. From `update.rs` `show_toast`.
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

/// The update toast's second line: what the reload waits for (`None` once nothing holds the page). From
/// `update.rs` `set_reason`.
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

/// The error toast's inside: the whole message, Copy and Close. From `ui.rs` `error_toast_html`.
pub fn error_toast_html(msg: &str) -> String {
    format!(
        r#"<div class="toast-text">{m}</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-copy-text="{m}">{copy}{COPY}</button><button type="button" class="btn sm ghost icon" data-toast-close title="{CLOSE}" aria-label="{CLOSE}">{x}</button></div>"#,
        m = esc(msg),
        copy = icon("copy"),
        x = icon("x")
    )
}

/// One toast. From `ui.rs` `toast`, `error_toast` and `toast_action`.
#[component]
pub fn ToastView(
    /// The toast.
    entry: Entry,
) -> impl IntoView {
    let id = entry.id;
    let fading = entry.fading.clone();
    let style = move || fading.get().then_some("opacity:0;transition:opacity .3s");
    if entry.stays {
        let msg = entry.text.clone();
        let copy = msg.clone();
        return view! {
            <div {..()} class="toast err stays" role="alert" data-msg=msg.clone()>
                <div class="toast-text">{msg.clone()}</div>
                <div class="toast-acts">
                    <button {..()} type="button" class="btn sm ghost" data-copy-text=msg.clone()
                        on:click=move |e| { e.stop_propagation(); copy_text(&copy); }
                        {..html(format!("{}{COPY}", icon("copy")))}>{""}</button>
                    <button {..()} type="button" class="btn sm ghost icon" data-toast-close title=CLOSE aria-label=CLOSE
                        on:click=move |e| { e.stop_propagation(); close(id); }
                        {..html(icon("x"))}>{""}</button>
                </div>
            </div>
        }
        .into_any();
    }
    if let Some(label) = entry.action.clone() {
        return view! {
            <div {..()} class="toast act" style=style>
                <div class="toast-text">{entry.text.clone()}</div>
                <div class="toast-acts">
                    <button {..()} type="button" class="btn sm ghost" data-toast-action on:click=move |_| press(id)>{label}</button>
                </div>
            </div>
        }
        .into_any();
    }
    view! { <div class=format!("toast {}", entry.kind.as_str()) style=style>{entry.text.clone()}</div> }.into_any()
}

/// The update toast. From `update.rs` `show_toast`.
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

/// Draws the toasts into the page's existing `#toasts` (Encore's index.html has one). Call once at start-up.
/// Each toast is mounted on its own and appended, as Encore's `appendChild` does.
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

    /// Encore's `toast` and `error_toast` markup (`ui.rs`), as they put it into `#toasts`.
    fn encore_plain(msg: &str, kind: &str) -> String {
        format!(r#"<div class="toast {kind}">{}</div>"#, esc(msg))
    }
    fn encore_error(msg: &str) -> String {
        format!(r#"<div class="toast err stays" role="alert" data-msg="{}">{}</div>"#, esc(msg), error_toast_html(msg))
    }

    #[test]
    fn toasts_draw_encores_markup() {
        clear();
        toast("Saved", "ok");
        toast("import failed: <Band> & \"Co\"", "err");
        toast(COPY_FAILED, "err");
        toast("Some files were left out", "");
        let want = format!(
            r#"<div id="toasts">{}{}{}{}</div>"#,
            encore_plain("Saved", "ok"),
            encore_error("import failed: <Band> & \"Co\""),
            encore_plain(COPY_FAILED, "err"),
            encore_plain("Some files were left out", "")
        );
        assert_same_html(&drawn(), &want);
    }

    #[test]
    fn three_errors_stay_and_the_same_one_is_shown_once() {
        clear();
        for m in ["a", "b", "a", "c", "d"] {
            toast(m, "err");
        }
        let want = format!(r#"<div id="toasts">{}{}{}</div>"#, encore_error("b"), encore_error("c"), encore_error("d"));
        assert_same_html(&drawn(), &want);
    }

    #[test]
    fn an_action_toast_runs_once_and_goes() {
        clear();
        let ran = Rc::new(std::cell::Cell::new(0));
        let r = ran.clone();
        toast_action("Up next cleared", "Undo", 6000, move || r.set(r.get() + 1));
        let want = r#"<div id="toasts"><div class="toast act"><div class="toast-text">Up next cleared</div><div class="toast-acts"><button type="button" class="btn sm ghost" data-toast-action>Undo</button></div></div></div>"#;
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
        assert!(show_update("A new version of Encore is ready", "Reload", Rc::new(|| {})));
        assert!(!show_update("A new version of Encore is ready", "Reload", Rc::new(|| {})));
        toast("after", "ok");
        set_update_reason(Some("It reloads when the music stops"));
        // update.rs `show_toast`
        let update = r#"<div id="sw-update" class="toast" role="status" style="display:flex;align-items:center;gap:12px"><span class="grow">A new version of Encore is ready<span class="small muted" data-update-why style="display:block">It reloads when the music stops</span></span><button class="btn sm primary" type="button">Reload</button></div>"#;
        let want = format!(r#"<div id="toasts">{}{update}{}</div>"#, encore_plain("before", "ok"), encore_plain("after", "ok"));
        assert_same_html(&drawn(), &want);
        STATE.with(|s| s.borrow_mut().update = None);
    }
}
