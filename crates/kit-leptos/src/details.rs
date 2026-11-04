//! The whole text of an error or a status, readable and copyable.
//!
//! The rules are kit-rules' [`kit_rules::details`]: the title is "Details" when blank, Copy copies exactly the
//! text shown, and a click that ends a selection does not open the dialog. Copy and Close say the configured
//! words ([`ToastWords::copy`](kit_rules::toasts::ToastWords::copy),
//! [`DialogWords::close`](kit_rules::dialogs::DialogWords::close)).

use crate::clipboard::copy_text;
use crate::html::esc;
use crate::html::html;
use crate::icons::icon;
use crate::modal::{self, ModalOpts};
use kit_rules::details::{opens_on_click, title as title_or_details};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// ` data-details="…" data-details-title="…"`: a click on the element opens the whole text in [`show`] (with
/// [`init`] running).
pub fn attrs(title: &str, text: &str) -> String {
    format!(r#" data-details="{}" data-details-title="{}""#, esc(text), esc(title))
}

/// A small Copy button (an icon, "Copy" as its title) that puts `text` on the clipboard.
pub fn copy_button_html(text: &str) -> String {
    let copy = copy_word();
    format!(
        r#"<button type="button" class="btn sm ghost icon copy-btn" data-copy-text="{}" title="{copy}" aria-label="{copy}">{}</button>"#,
        esc(text),
        icon("copy")
    )
}

/// [`copy_button_html`] as a view.
#[component]
pub fn CopyButton(
    /// The text to copy.
    text: String,
) -> impl IntoView {
    let t = text.clone();
    let copy = copy_word();
    view! {
        <button {..()} type="button" class="btn sm ghost icon copy-btn" data-copy-text=text title=copy aria-label=copy
            on:click=move |e| { e.stop_propagation(); copy_text(&t); } {..html(icon("copy"))}>{""}</button>
    }
}

/// The dialog's body: the whole text in a wrapped, selectable block.
pub fn body_html(text: &str) -> String {
    format!(r#"<pre class="details-text" tabindex="0">{}</pre>"#, esc(text))
}

/// The dialog's buttons: Close and Copy (which copies exactly the text shown).
pub fn footer_html(text: &str) -> String {
    let (close, copy) = (close_word(), copy_word());
    format!(
        r#"<button class="btn" data-close>{close}</button><button class="btn primary" data-copy-text="{}">{}{copy}</button>"#,
        esc(text),
        icon("copy")
    )
}

/// The dialog's buttons as a view (Copy handled here, so no page listener is needed).
fn footer_view(text: String) -> impl IntoView {
    let t = text.clone();
    let (close, copy) = (close_word(), copy_word());
    view! {
        <button class="btn" data-close>{close}</button>
        <button class="btn primary" data-copy-text=text on:click=move |e| { e.stop_propagation(); copy_text(&t); }
            {..html(format!("{}{copy}", icon("copy")))}>{""}</button>
    }
}

fn copy_word() -> &'static str {
    crate::config::get().words.toasts.copy
}

fn close_word() -> &'static str {
    crate::config::get().words.dialogs.close
}

/// The options of the dialog with the whole text (for a view of one's own).
pub fn opts(title: &str, text: &str) -> ModalOpts {
    let t = text.to_string();
    ModalOpts::new(&esc(title_or_details(title)), &body_html(text)).footer_view(move || footer_view(t)).wide(true)
}

/// The dialog with the whole text: wrapped, selectable, with Copy and Close.
pub fn show(title: &str, text: &str) -> modal::Modal {
    modal::open(opts(title, text))
}

/// Whether the user has text selected on the page: a click that ends a selection must not open the dialog.
pub fn selecting() -> bool {
    let Some(w) = crate::rt::window() else { return false };
    let text = w
        .get_selection()
        .ok()
        .flatten()
        .and_then(|s| {
            js_sys::Reflect::get(&s, &"toString".into())
                .ok()
                .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
                .and_then(|f| f.call0(&s).ok())
        })
        .and_then(|t| t.as_string());
    !opens_on_click(&text.unwrap_or_default())
}

/// The click listener for hand-written pages: `[data-copy-text]` copies, `[data-details]` opens the whole text,
/// an error toast's `[data-toast-close]` closes it. Links and buttons inside a `[data-details]` element keep their
/// own action.
pub fn init() {
    crate::rt::on_document("click", |e| {
        if let Some(b) = crate::rt::closest(e, "[data-toast-close]") {
            if let Some(t) = b.closest(".toast").ok().flatten() {
                t.remove();
            }
            return;
        }
        if let Some(b) = crate::rt::closest(e, "[data-copy-text]") {
            e.stop_propagation();
            copy_text(&b.get_attribute("data-copy-text").unwrap_or_default());
            return;
        }
        let Some(el) = crate::rt::closest(e, "[data-details]") else { return };
        if let Some(inner) = crate::rt::closest(e, "a, button, input, select, textarea, summary") {
            let node: &web_sys::Node = inner.as_ref();
            if inner != el && el.contains(Some(node)) {
                return;
            }
        }
        if selecting() {
            return;
        }
        show(&el.get_attribute("data-details-title").unwrap_or_default(), &el.get_attribute("data-details").unwrap_or_default());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::assert_same_html;

    #[test]
    fn the_text_is_escaped_once_and_copied_as_shown() {
        let t = "upload failed: mkdir /data/<Team> & \"Co\": Input/output error (os error 5)\nline 2";
        let a = attrs("Request #12", t);
        assert!(
            a.contains(r#"data-details="upload failed: mkdir /data/&lt;Team&gt; &amp; &quot;Co&quot;: Input/output error (os error 5)"#),
            "{a}"
        );
        assert!(a.contains(r#"data-details-title="Request #12""#), "{a}");
        assert!(body_html(t).starts_with(r#"<pre class="details-text" tabindex="0">upload failed: mkdir /data/&lt;Team&gt;"#));
        let f = footer_html(t);
        assert!(f.contains(">Close</button>") && f.ends_with("Copy</button>"), "{f}");
        assert!(f.contains(r#"data-copy-text="upload failed: mkdir /data/&lt;Team&gt; &amp; &quot;Co&quot;"#), "{f}");
        let b = copy_button_html("x");
        assert!(b.contains(r#"data-copy-text="x""#) && b.contains(r#"title="Copy""#), "{b}");
    }

    #[test]
    fn the_views_draw_the_same_markup_as_the_strings() {
        let t = "a <b> & \"c\"";
        assert_same_html(&view! { <CopyButton text=t.to_string()/> }.to_html(), &copy_button_html(t));
        assert_same_html(&footer_view(t.to_string()).to_html(), &footer_html(t));
        // the dialog: "Details" for a blank title, wide, the whole text
        let html = crate::modal::modal_view(opts("  ", t)).to_html();
        assert!(html.contains("<h2>Details</h2>") && html.contains("modal wide"), "{html}");
        assert!(html.contains(&body_html(t)), "{html}");
    }
}
