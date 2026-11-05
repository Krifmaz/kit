//! [`Sheet`]: a panel over the page. On phones it is a full-width bottom sheet with a grab handle that keeps clear
//! of the home indicator; from 768 px it is a centred panel. Escape, the close button and a tap on the dimmed page
//! close it.

use crate::icons::icon;
use leptos::prelude::*;

/// A sheet with a title over the page while `open` is true.
#[component]
pub fn Sheet(
    /// Whether the sheet shows; closing it sets this to false.
    open: RwSignal<bool>,
    /// Its title.
    #[prop(into)]
    title: String,
    /// The close button's name for screen readers.
    #[prop(optional, into, default = "Close".to_string())]
    close_label: String,
    /// The content.
    children: ChildrenFn,
) -> impl IntoView {
    let panel = NodeRef::<leptos::html::Div>::new();
    // focus the sheet once drawn, so Escape reaches it and screen readers start there (in the browser only:
    // markup drawn to a string has no focus)
    if cfg!(target_arch = "wasm32") {
        Effect::new(move |_| {
            if open.get() {
                if let Some(p) = panel.get() {
                    let _ = p.focus();
                }
            }
        });
    }
    let title = StoredValue::new(title);
    let close_label = StoredValue::new(close_label);
    let children = StoredValue::new(children);
    view! {
        <Show when=move || open.get()>
            <div
                class="k-sheet-bg"
                on:click=move |e| {
                    // a tap on the dimmed page, not on the sheet
                    if e.target() == e.current_target() {
                        open.set(false);
                    }
                }
                on:keydown=move |e| {
                    if e.key() == "Escape" {
                        open.set(false);
                    }
                }
            >
                <div class="k-sheet" role="dialog" aria-modal="true" aria-label=title.get_value() tabindex="-1" node_ref=panel>
                    <div class="k-sheet-handle" aria-hidden="true"></div>
                    <div class="k-sheet-head">
                        <h2>{title.get_value()}</h2>
                        <button type="button" class="k-icon-btn" aria-label=close_label.get_value() on:click=move |_| open.set(false) inner_html=icon("x")></button>
                    </div>
                    <div class="k-sheet-body">{children.read_value()()}</div>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_sheet_draws_nothing_and_an_open_one_a_dialog() {
        let closed = view! { <Sheet open=RwSignal::new(false) title="T"><p>"x"</p></Sheet> }.to_html();
        assert!(!closed.contains("k-sheet"), "{closed}");
        let open = view! { <Sheet open=RwSignal::new(true) title="Pick one"><p>"x"</p></Sheet> }.to_html();
        for want in ["k-sheet-bg", "role=\"dialog\"", "aria-modal=\"true\"", "Pick one", "aria-label=\"Close\"", "<p>x</p>"] {
            assert!(open.contains(want), "{want} in {open}");
        }
    }
}
