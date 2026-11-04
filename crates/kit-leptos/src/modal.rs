//! Dialogs: a modal over the page, and the confirm dialog.
//!
//! [`open`] puts a modal in `#modal-root` ([`ModalRoot`]); Escape, a press beside it and every `[data-close]`
//! button close it. The confirm dialog asks one question before something happens: [`confirm`] opens it and
//! answers true for OK, false for any other way it closes; [`ConfirmDialog`] is the same dialog as a component
//! the app shows itself. Its words come from [`kit_rules::dialogs::Confirm`], with the configured
//! [`DialogWords`](kit_rules::dialogs::DialogWords) for the buttons it does not name.

use crate::html::esc;
use crate::html::html;
use crate::icons::icon;
use crate::rt::{self, Listener, Mounted};
use kit_rules::dialogs::{Answer, Confirm};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::Element;

/// A part of a modal: HTML (escaped by the caller) or a view.
pub enum Content {
    /// An HTML string, put in as is.
    Html(String),
    /// A view, drawn when the modal opens.
    View(Box<dyn FnOnce() -> AnyView>),
}

impl Content {
    /// A view part.
    pub fn view<V: IntoView + 'static>(f: impl FnOnce() -> V + 'static) -> Content {
        Content::View(Box::new(move || f().into_any()))
    }

    /// Plain text, escaped.
    pub fn text(s: &str) -> Content {
        Content::Html(esc(s))
    }
}

/// What a modal shows.
pub struct ModalOpts {
    /// The title (`<h2>`).
    pub title: Content,
    /// The body (`.modal-body`).
    pub body: Content,
    /// The buttons (`.modal-foot`), if any.
    pub footer: Option<Content>,
    /// The wide modal (980 px instead of 560 px).
    pub wide: bool,
}

impl ModalOpts {
    /// A modal with an HTML title and body (both are put in as is).
    pub fn new(title_html: &str, body_html: &str) -> Self {
        ModalOpts { title: Content::Html(title_html.to_string()), body: Content::Html(body_html.to_string()), footer: None, wide: false }
    }

    /// A modal with a plain-text title and a view as its body.
    pub fn with_view<V: IntoView + 'static>(title: &str, body: impl FnOnce() -> V + 'static) -> Self {
        ModalOpts { title: Content::text(title), body: Content::view(body), footer: None, wide: false }
    }

    /// The body as a view.
    pub fn body_view<V: IntoView + 'static>(mut self, f: impl FnOnce() -> V + 'static) -> Self {
        self.body = Content::view(f);
        self
    }

    /// The footer as HTML (an empty string is no footer).
    pub fn footer(mut self, html: &str) -> Self {
        self.footer = (!html.is_empty()).then(|| Content::Html(html.to_string()));
        self
    }

    /// The footer as a view.
    pub fn footer_view<V: IntoView + 'static>(mut self, f: impl FnOnce() -> V + 'static) -> Self {
        self.footer = Some(Content::view(f));
        self
    }

    /// The wide modal.
    pub fn wide(mut self, w: bool) -> Self {
        self.wide = w;
        self
    }
}

fn part(tag: &'static str, class: &'static str, c: Content) -> AnyView {
    match (tag, c) {
        ("h2", Content::Html(h)) => view! { <h2 {..html(h)}>{""}</h2> }.into_any(),
        ("h2", Content::View(f)) => view! { <h2>{f()}</h2> }.into_any(),
        (_, Content::Html(h)) => view! { <div class={class} {..html(h)}>{""}</div> }.into_any(),
        (_, Content::View(f)) => view! { <div class=class>{f()}</div> }.into_any(),
    }
}

/// The modal's markup: `.modal-bg` › `.modal` with its head (title and ×), body and footer.
pub fn modal_view(o: ModalOpts) -> AnyView {
    // no footer: an empty text node, not an empty `<!---->` placeholder
    let foot = o.footer.map(|f| part("div", "modal-foot", f)).unwrap_or_else(|| "".into_any());
    let close_w = crate::config::get().words.dialogs.close;
    view! {
        <div class="modal-bg">
            <div class=if o.wide { "modal wide" } else { "modal" } role="dialog" aria-modal="true">
                <div class="modal-head">{part("h2", "", o.title)}<button class="btn ghost icon" data-close aria-label={close_w} {..html(icon("x"))}>{""}</button></div>
                {part("div", "modal-body", o.body)}
                {foot}
            </div>
        </div>
    }
    .into_any()
}

#[derive(Default)]
struct Inner {
    bg: Option<Element>,
    mounted: RefCell<Option<Mounted>>,
    listeners: RefCell<Vec<Listener>>,
    on_close: RefCell<Vec<Box<dyn FnOnce()>>>,
}

/// A modal on screen: [`Modal::close`] takes it away.
#[derive(Clone, Default)]
pub struct Modal(Rc<Inner>);

impl Modal {
    /// Takes the modal away (once); the [`Modal::on_close`] callbacks run.
    pub fn close(&self) {
        let Some(m) = self.0.mounted.borrow_mut().take() else { return };
        OPEN.with(|o| o.borrow_mut().retain(|x| !Rc::ptr_eq(&x.0, &self.0)));
        drop(m);
        self.0.listeners.borrow_mut().clear();
        let fs: Vec<_> = self.0.on_close.borrow_mut().drain(..).collect();
        for f in fs {
            f();
        }
    }

    /// Runs `f` when the modal goes, however it is closed (right away when it is not open).
    pub fn on_close(&self, f: impl FnOnce() + 'static) {
        if self.is_open() {
            self.0.on_close.borrow_mut().push(Box::new(f));
        } else {
            f();
        }
    }

    /// The `.modal` element.
    pub fn el(&self) -> Option<Element> {
        rt::qs_in(self.0.bg.as_ref()?, ".modal")
    }

    /// The `.modal-bg` element.
    pub fn bg(&self) -> Option<Element> {
        self.0.bg.clone()
    }

    /// `querySelector` inside the modal.
    pub fn qs(&self, sel: &str) -> Option<Element> {
        rt::qs_in(self.0.bg.as_ref()?, sel)
    }

    /// `querySelectorAll` inside the modal.
    pub fn qsa(&self, sel: &str) -> Vec<Element> {
        self.0.bg.as_ref().map(|b| rt::qsa_in(b, sel)).unwrap_or_default()
    }

    /// Whether it is still on screen: not closed, and its `.modal-bg` still in the page (a dialog that other code
    /// took off the page counts as gone).
    pub fn is_open(&self) -> bool {
        self.0.mounted.borrow().is_some() && self.0.bg.as_ref().is_some_and(|b| b.is_connected())
    }
}

thread_local! {
    /// The modals on screen. A modal stays open until it is closed, whether or not its caller keeps the
    /// handle.
    static OPEN: RefCell<Vec<Modal>> = const { RefCell::new(Vec::new()) };
}

/// Closes every modal on screen (a new page); each one's
/// [`Modal::on_close`] callbacks run, so a confirm dialog that was open answers false.
pub fn close_all() {
    let all: Vec<Modal> = OPEN.with(|o| o.borrow_mut().drain(..).collect());
    for m in all {
        m.close();
    }
}

/// `<div id="modal-root">`, where modals go.
#[component]
pub fn ModalRoot() -> impl IntoView {
    view! { <div id="modal-root"></div> }
}

/// Opens a modal in `#modal-root`.
pub fn open(o: ModalOpts) -> Modal {
    let Some(root) = rt::by_id("modal-root") else { return Modal::default() };
    let Some(mounted) = rt::mount(&root, move || modal_view(o)) else { return Modal::default() };
    let Some(bg) = root.last_element_child() else { return Modal::default() };
    let m = Modal(Rc::new(Inner { bg: Some(bg.clone()), mounted: RefCell::new(Some(mounted)), ..Inner::default() }));
    let weak = Rc::downgrade(&m.0);
    let closer = move || {
        if let Some(i) = weak.upgrade() {
            Modal(i).close();
        }
    };
    let mut ls = Vec::new();
    if let Some(doc) = rt::document() {
        let c = closer.clone();
        ls.push(rt::listen(&doc, "keydown", move |e| {
            if e.dyn_ref::<web_sys::KeyboardEvent>().is_some_and(|k| k.key() == "Escape") {
                c();
            }
        }));
    }
    let c = closer.clone();
    let bg2 = bg.clone();
    ls.push(rt::listen(&bg, "mousedown", move |e| {
        if e.target().is_some_and(|t| t.dyn_ref::<Element>() == Some(&bg2)) {
            c();
        }
    }));
    for b in rt::qsa_in(&bg, "[data-close]") {
        let c = closer.clone();
        ls.push(rt::listen(&b, "click", move |_| c()));
    }
    *m.0.listeners.borrow_mut() = ls;
    OPEN.with(|o| o.borrow_mut().push(m.clone()));
    m
}

/// The confirm dialog's body: its line of text, muted (nothing for none).
pub fn confirm_body_html(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    format!(r#"<p class="confirm-text muted">{}</p>"#, esc(text))
}

/// The confirm dialog's buttons: Cancel, and OK (`danger` makes it red), with the configured Cancel.
pub fn confirm_footer_html(ok_label: &str, danger: bool) -> String {
    let c = Confirm::new("").ok(ok_label).danger(danger);
    footer_html(&c)
}

fn footer_html(c: &Confirm) -> String {
    let w = crate::config::get().words.dialogs;
    format!(
        r#"<button class="btn" data-close>{}</button><button class="btn {}" data-ok>{}</button>"#,
        esc(c.cancel_label(&w)),
        c.ok_style(),
        esc(c.ok_label(&w))
    )
}

/// The options of the dialog that asks `c` (for a view of one's own).
pub fn confirm_opts(c: &Confirm) -> ModalOpts {
    ModalOpts::new(&esc(&c.title), &confirm_body_html(&c.text)).footer(&footer_html(c))
}

/// Asks `c` in a confirm dialog: true on OK, false for any other way it closes (Cancel, Escape, a press beside
/// it, [`close_all`]).
///
/// ```no_run
/// # async fn f() {
/// use kit_leptos::modal::confirm;
/// use kit_rules::dialogs::Confirm;
/// if confirm(Confirm::new("Delete this note?").text("It is gone for good.").ok("Delete").danger(true)).await {
///     // delete it
/// }
/// # }
/// ```
pub async fn confirm(c: Confirm) -> bool {
    ask(confirm_opts(&c)).await
}

/// `confirm_dialog(title, text, ok_label, danger)`: [`confirm`] with these four.
pub async fn confirm_dialog(title: &str, text: &str, ok_label: &str, danger: bool) -> bool {
    confirm(Confirm::new(title).text(text).ok(ok_label).danger(danger)).await
}

/// [`confirm_dialog`] with a body that is HTML already (escaped by the caller).
pub async fn confirm_dialog_html(title: &str, body: &str, ok_label: &str, danger: bool) -> bool {
    let c = Confirm::new(title).ok(ok_label).danger(danger);
    ask(ModalOpts::new(&esc(title), body).footer(&footer_html(&c))).await
}

/// Opens `o` and waits for its answer: its `[data-ok]` button is yes, any other way it closes is no.
async fn ask(o: ModalOpts) -> bool {
    let (tx, rx) = futures::channel::oneshot::channel::<bool>();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let m = open(o);
    if let Some(ok) = m.qs("[data-ok]") {
        let (m2, tx2) = (m.clone(), tx.clone());
        rt::listen_forever(&ok, "click", move |_| {
            if let Some(t) = tx2.borrow_mut().take() {
                let _ = t.send(true);
            }
            m2.close();
        });
    }
    // dismissed (Escape, backdrop, Cancel): the modal goes without OK
    m.on_close(move || {
        if let Some(t) = tx.borrow_mut().take() {
            let _ = t.send(false);
        }
    });
    rx.await.unwrap_or(false)
}

/// The confirm dialog as a component: the app draws it while it asks and takes it away on `on_answer`, which
/// runs once, with true for OK and false for Cancel, Escape or a press beside it.
#[component]
pub fn ConfirmDialog(
    /// What it asks.
    confirm: Confirm,
    /// Its answer (once).
    on_answer: Callback<bool>,
) -> impl IntoView {
    let w = crate::config::get().words.dialogs;
    let answer = StoredValue::new(Answer::default());
    let decide = move |yes: bool| {
        let first = answer.try_update_value(|a| if yes { a.ok() } else { a.dismissed() }).unwrap_or(false);
        if first {
            on_answer.run(yes);
        }
    };
    let esc_listener = rt::document().map(|d| {
        rt::listen(&d, "keydown", move |e| {
            if e.dyn_ref::<web_sys::KeyboardEvent>().is_some_and(|k| k.key() == "Escape") {
                decide(false);
            }
        })
    });
    let esc_listener = send_wrapper::SendWrapper::new(esc_listener);
    on_cleanup(move || drop(esc_listener));
    let (ok, cancel, style) = (confirm.ok_label(&w).to_string(), confirm.cancel_label(&w).to_string(), confirm.ok_style());
    let text = (!confirm.text.is_empty()).then(|| view! { <p class="confirm-text muted">{confirm.text.clone()}</p> });
    view! {
        <div class="modal-bg" on:mousedown=move |e| {
            if e.target() == e.current_target() {
                decide(false);
            }
        }>
            <div class="modal" role="dialog" aria-modal="true">
                <div class="modal-head"><h2>{confirm.title.clone()}</h2><button class="btn ghost icon" data-close aria-label=w.close on:click=move |_| decide(false) {..html(icon("x"))}>{""}</button></div>
                <div class="modal-body">{crate::html::opt(text)}</div>
                <div class="modal-foot">
                    <button class="btn" data-close on:click=move |_| decide(false)>{cancel}</button>
                    <button class=format!("btn {style}") data-ok on:click=move |_| decide(true)>{ok}</button>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};

    const X_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>"#;

    /// The kit's modal markup around an (already escaped) title, body and footer.
    fn modal(title: &str, body: &str, footer: &str, wide: bool) -> String {
        let foot = if footer.is_empty() { String::new() } else { format!(r#"<div class="modal-foot">{footer}</div>"#) };
        format!(
            r#"<div class="modal-bg"><div class="modal{}" role="dialog" aria-modal="true">
                <div class="modal-head"><h2>{title}</h2><button class="btn ghost icon" data-close aria-label="Close">{X_SVG}</button></div>
                <div class="modal-body">{body}</div>{foot}</div></div>"#,
            if wide { " wide" } else { "" },
        )
    }

    #[test]
    fn a_modal_draws_its_head_body_and_footer() {
        let html = modal_view(ModalOpts::new("Delete &lt;x&gt;?", "<p>body</p>")).to_html();
        assert_same_html(&html, &modal("Delete &lt;x&gt;?", "<p>body</p>", "", false));
        let html = modal_view(ModalOpts::new("T", "b").footer("<button>x</button>").wide(true)).to_html();
        assert_same_html(&html, &modal("T", "b", "<button>x</button>", true));
        // a view body draws the same as its HTML
        let html = modal_view(ModalOpts::with_view("A & B", || view! { <p class="muted">"x"</p> })).to_html();
        assert_same_html(&html, &modal("A &amp; B", r#"<p class="muted">x</p>"#, "", false));
    }

    #[test]
    fn the_confirm_dialog_names_its_buttons() {
        let c = Confirm::new("Delete this note?").text("Gone for <good>.").ok("Delete").danger(true);
        let want = modal(
            "Delete this note?",
            r#"<p class="confirm-text muted">Gone for &lt;good&gt;.</p>"#,
            r#"<button class="btn" data-close>Cancel</button><button class="btn danger" data-ok>Delete</button>"#,
            false,
        );
        assert_same_html(&modal_view(confirm_opts(&c)).to_html(), &want);
        // the component draws the same dialog
        assert_same_html(&render(|| view! { <ConfirmDialog confirm=c.clone() on_answer=Callback::new(|_| {})/> }), &want);
        // no text, no action named: OK, primary
        let plain = Confirm::new("Leave this page?");
        let want = modal(
            "Leave this page?",
            "",
            r#"<button class="btn" data-close>Cancel</button><button class="btn primary" data-ok>OK</button>"#,
            false,
        );
        assert_same_html(&modal_view(confirm_opts(&plain)).to_html(), &want);
        assert_same_html(&render(|| view! { <ConfirmDialog confirm=plain.clone() on_answer=Callback::new(|_| {})/> }), &want);
        assert!(confirm_footer_html("OK", false).contains(r#"class="btn primary" data-ok"#));
    }

    #[test]
    fn the_buttons_say_the_configured_words() {
        let mut cfg = crate::config::AppConfig::default();
        cfg.words.dialogs = kit_rules::dialogs::DialogWords { cancel: "Abbrechen", close: "Schließen", ok: "Ja" };
        crate::config::set(cfg);
        let html = modal_view(confirm_opts(&Confirm::new("Weiter?"))).to_html();
        assert!(
            html.contains(">Abbrechen</button>") && html.contains(">Ja</button>") && html.contains(r#"aria-label="Schließen""#),
            "{html}"
        );
        // a label the dialog names itself wins
        let html = modal_view(confirm_opts(&Confirm::new("Löschen?").ok("Löschen").cancel("Behalten"))).to_html();
        assert!(html.contains(">Behalten</button>") && html.contains(">Löschen</button>"), "{html}");
        crate::config::set(crate::config::AppConfig::default());
    }
}
