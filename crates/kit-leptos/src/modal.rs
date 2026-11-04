//! Dialogs: a modal over the page, and the confirm dialog.
//!
//! From Encore's `web-rs/src/ui.rs` (`modal`, `ModalOpts`, `Modal`, `confirm_dialog`, `confirm_dialog_html`).
//! Each modal is a `.modal-bg` appended to the page's `#modal-root` (draw [`ModalRoot`] or keep Encore's
//! `<div id="modal-root">`), closed by Escape, a press on the backdrop or any `[data-close]` inside it. Its title,
//! body and footer are either HTML strings, as Encore's hand-written pages pass them ([`Content::Html`]), or
//! Leptos views ([`Content::View`]). [`open`] mounts it at once, so a caller can look up its elements right after
//! ([`Modal::qs`]), as Encore's pages do.

use crate::html::esc;
use crate::html::html;
use crate::icons::icon;
use crate::rt::{self, Listener, Mounted};
use kit_rules::dialogs::{CANCEL, CLOSE};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::Element;

/// A part of a modal: HTML (escaped by the caller, as Encore's `modal` takes it) or a view.
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

/// What a modal shows. From `ui.rs` `ModalOpts`.
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
    /// A modal with an HTML title and body, as Encore's `ModalOpts::new` (both are put in as is).
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

    /// The footer as HTML (Encore's `.footer(…)`; an empty string is no footer).
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

/// The modal's markup: `.modal-bg` › `.modal` with its head (title and ×), body and footer, with the line breaks
/// Encore's template has between them (so the DOM is the same node for node). From `ui.rs` `modal`.
pub fn modal_view(o: ModalOpts) -> AnyView {
    // no footer: an empty text node, not an empty `<!---->` placeholder
    let foot = o.footer.map(|f| part("div", "modal-foot", f)).unwrap_or_else(|| "".into_any());
    view! {
        <div class="modal-bg">
            <div class=if o.wide { "modal wide" } else { "modal " } role="dialog" aria-modal="true">
                {"\n    "}
                <div class="modal-head">{part("h2", "", o.title)}<button class="btn ghost icon" data-close aria-label={CLOSE} {..html(icon("x"))}>{""}</button></div>
                {"\n    "}
                {part("div", "modal-body", o.body)}
                {"\n    "}
                {foot}
                {"\n  "}
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

/// A modal on screen: [`Modal::close`] takes it away. From `ui.rs` `Modal`.
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

    /// Whether it is still on screen: not closed, and its `.modal-bg` still in the page (Encore's
    /// `el.is_connected()`, so a dialog that other code took off the page counts as gone).
    pub fn is_open(&self) -> bool {
        self.0.mounted.borrow().is_some() && self.0.bg.as_ref().is_some_and(|b| b.is_connected())
    }
}

thread_local! {
    /// The modals on screen. A modal stays open until it is closed, whether or not its caller keeps the
    /// handle (Encore's pages often drop it, as the DOM kept Encore's).
    static OPEN: RefCell<Vec<Modal>> = const { RefCell::new(Vec::new()) };
}

/// Closes every modal on screen (a new page: Encore's router empties `#modal-root`); each one's
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

/// Opens a modal in `#modal-root`. From `ui.rs` `modal`.
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

/// The confirm dialog's body: the text as a muted line. From `ui.rs` `confirm_dialog`.
pub fn confirm_body_html(text: &str) -> String {
    format!(r#"<p class="muted" style="margin:0">{}</p>"#, esc(text))
}

/// The confirm dialog's buttons: Cancel, and OK (`danger` makes it red). From `ui.rs` `confirm_dialog_html`.
pub fn confirm_footer_html(ok_label: &str, danger: bool) -> String {
    format!(
        r#"<button class="btn" data-close>{CANCEL}</button><button class="btn {}" data-ok>{}</button>"#,
        if danger { "danger" } else { "primary" },
        esc(ok_label)
    )
}

/// `confirmDialog(title, text, okLabel, danger)`: true on OK, false when dismissed. From `ui.rs` `confirm_dialog`.
pub async fn confirm_dialog(title: &str, text: &str, ok_label: &str, danger: bool) -> bool {
    confirm_dialog_html(title, &confirm_body_html(text), ok_label, danger).await
}

/// [`confirm_dialog`] with a body that is HTML already (escaped by the caller).
pub async fn confirm_dialog_html(title: &str, body: &str, ok_label: &str, danger: bool) -> bool {
    let (tx, rx) = futures::channel::oneshot::channel::<bool>();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let m = open(ModalOpts::new(&esc(title), body).footer(&confirm_footer_html(ok_label, danger)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_same_html;

    /// Encore's `modal` markup (`ui.rs`).
    fn encore(title: &str, body: &str, footer: &str, wide: bool) -> String {
        let foot = if footer.is_empty() { String::new() } else { format!(r#"<div class="modal-foot">{footer}</div>"#) };
        format!(
            r#"<div class="modal-bg"><div class="modal {}" role="dialog" aria-modal="true">
    <div class="modal-head"><h2>{}</h2><button class="btn ghost icon" data-close aria-label="Close">{}</button></div>
    <div class="modal-body">{}</div>
    {}
  </div></div>"#,
            if wide { "wide" } else { "" },
            title,
            icon("x"),
            body,
            foot
        )
    }

    #[test]
    fn a_modal_is_encores_markup() {
        let html = modal_view(ModalOpts::new("Delete &lt;x&gt;?", "<p>body</p>")).to_html();
        assert_same_html(&html, &encore("Delete &lt;x&gt;?", "<p>body</p>", "", false));
        let html = modal_view(ModalOpts::new("T", "b").footer("<button>x</button>").wide(true)).to_html();
        assert_same_html(&html, &encore("T", "b", "<button>x</button>", true));
        // a view body draws the same as its HTML
        let html = modal_view(ModalOpts::with_view("A & B", || view! { <p class="muted">"x"</p> })).to_html();
        assert_same_html(&html, &encore("A &amp; B", r#"<p class="muted">x</p>"#, "", false));
    }

    #[test]
    fn the_confirm_dialog_is_encores() {
        let html = modal_view(
            ModalOpts::new(&esc("Delete this report?"), &confirm_body_html("Gone for <good>."))
                .footer(&confirm_footer_html("Delete", true)),
        )
        .to_html();
        let want = encore(
            "Delete this report?",
            r#"<p class="muted" style="margin:0">Gone for &lt;good&gt;.</p>"#,
            r#"<button class="btn" data-close>Cancel</button><button class="btn danger" data-ok>Delete</button>"#,
            false,
        );
        assert_same_html(&html, &want);
        assert!(confirm_footer_html("OK", false).contains(r#"class="btn primary" data-ok"#));
    }
}
