//! Drag and drop for every file upload: one set of listeners on the document turns any upload into a drop place.
//!
//! From Encore's `web-rs/src/dropzone.rs`. A drop place ("zone") is an element marked `data-dropzone` (its value
//! is the `id` of the file input it feeds, or empty for the first `input[type=file]` inside it; [`DropZone`]
//! draws one), or a `<label>` that holds a file input. A drop goes through the input itself: the files are
//! checked against its `accept` like the browser's chooser (kit-rules' [`choose`]), only the first is kept unless
//! it has `multiple`, they are put in `input.files` and a bubbling `change` fires, so a dropped file takes the
//! same path as a picked one. While files are dragged over the window every zone shows `drop-ready`, the one
//! under the pointer `drop-over`; files dropped anywhere else are swallowed, so a missed drop never opens the
//! file in the tab. Pages with their own drop handling call `preventDefault()` first and are left alone.

use crate::toasts::show;
use js_sys::{Array, Reflect};
use kit_rules::uploads::{choose, refused_kind, refused_message, NOT_POSSIBLE, OVER_CLASS, READY_CLASS};
use leptos::prelude::*;
use std::cell::Cell;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, Event, File, HtmlInputElement};

fn get(o: &JsValue, k: &str) -> JsValue {
    Reflect::get(o, &JsValue::from_str(k)).unwrap_or(JsValue::UNDEFINED)
}

/// Whether a drag carries files from the computer (not text or a link from the page). From `dropzone.rs`
/// `has_files`.
pub fn has_files(e: &Event) -> bool {
    let types = get(&get(e.as_ref(), "dataTransfer"), "types");
    if types.is_undefined() || types.is_null() {
        return false;
    }
    Array::from(&types).iter().any(|t| t.as_string().as_deref() == Some("Files"))
}

fn set_drop_effect(e: &Event, effect: &str) {
    let dt = get(e.as_ref(), "dataTransfer");
    if dt.is_object() {
        let _ = Reflect::set(&dt, &"dropEffect".into(), &effect.into());
    }
}

/// The zone an element sits in, with the input it feeds. From `dropzone.rs` `zone_of`.
fn zone_of(start: &Element) -> Option<(Element, HtmlInputElement)> {
    let mut el = Some(start.clone());
    while let Some(cur) = el {
        if let Some(id) = cur.get_attribute("data-dropzone") {
            let input = if id.trim().is_empty() { crate::rt::qs_in(&cur, "input[type=file]") } else { crate::rt::by_id(id.trim()) };
            if let Some(i) = input.and_then(|i| i.dyn_into::<HtmlInputElement>().ok()) {
                return Some((cur, i));
            }
        } else if cur.tag_name().eq_ignore_ascii_case("label") {
            if let Some(i) = crate::rt::qs_in(&cur, "input[type=file]").and_then(|i| i.dyn_into::<HtmlInputElement>().ok()) {
                return Some((cur, i));
            }
        }
        el = cur.parent_element();
    }
    None
}

fn zone_at(e: &Event) -> Option<(Element, HtmlInputElement)> {
    crate::rt::target_el(e).and_then(|t| zone_of(&t))
}

/// Whether an input may take a drop now (not disabled, not opted out with `data-no-drop`).
fn usable(input: &HtmlInputElement) -> bool {
    !input.disabled() && !input.has_attribute("data-no-drop")
}

fn all_zones() -> Vec<Element> {
    let mut out = crate::rt::qsa("[data-dropzone]");
    for i in crate::rt::qsa("label input[type=file]") {
        if let Some(l) = i.closest("label").ok().flatten() {
            if !out.contains(&l) {
                out.push(l);
            }
        }
    }
    out.retain(|z| zone_of(z).is_some_and(|(_, i)| usable(&i)));
    out
}

thread_local! {
    /// How deep the drag is inside the window (`dragenter` +1, `dragleave` −1; 0: the files left).
    static DEPTH: Cell<i32> = const { Cell::new(0) };
}

fn set_ready(on: bool) {
    let zones = if on { all_zones() } else { crate::rt::qsa(&format!(".{READY_CLASS}, .{OVER_CLASS}")) };
    for z in zones {
        let _ = z.class_list().toggle_with_force(READY_CLASS, on);
        if !on {
            let _ = z.class_list().remove_1(OVER_CLASS);
        }
    }
}

fn set_over(zone: Option<&Element>) {
    for z in crate::rt::qsa(&format!(".{OVER_CLASS}")) {
        if Some(&z) != zone {
            let _ = z.class_list().remove_1(OVER_CLASS);
        }
    }
    if let Some(z) = zone {
        let _ = z.class_list().add_1(OVER_CLASS);
    }
}

fn reset() {
    DEPTH.with(|d| d.set(0));
    set_ready(false);
}

fn hand_over(input: &HtmlInputElement, files: &[File]) {
    let Ok(dt) = web_sys::DataTransfer::new() else { return };
    for f in files {
        let _ = dt.items().add_with_file(f);
    }
    input.set_files(dt.files().as_ref());
    let init = web_sys::EventInit::new();
    init.set_bubbles(true);
    if let Ok(ev) = Event::new_with_event_init_dict("change", &init) {
        let _ = input.dispatch_event(&ev);
    }
}

fn dropped_files(e: &Event) -> Vec<File> {
    let list = get(&get(e.as_ref(), "dataTransfer"), "files");
    let n = get(&list, "length").as_f64().unwrap_or(0.0) as u32;
    (0..n).filter_map(|i| Reflect::get_u32(&list, i).ok()?.dyn_into::<File>().ok()).collect()
}

fn drop_on(input: &HtmlInputElement, files: Vec<File>) {
    let accept = input.accept();
    let (keep, refused) = choose(files, &accept, input.multiple(), |f| f.name(), |f| f.type_());
    let msg = refused_message(&refused, &accept, input.multiple(), keep.len());
    if !msg.is_empty() {
        show(&msg, refused_kind(keep.len()));
    }
    if !keep.is_empty() {
        hand_over(input, &keep);
    }
}

/// Starts drag and drop for the whole app (once, at start-up). The listeners sit on the document for the whole
/// session, so zones drawn later work without any setup. From `dropzone.rs` `init`.
pub fn init() {
    crate::rt::on_document("dragenter", |e| {
        if !has_files(e) {
            return;
        }
        let first = DEPTH.with(|d| {
            d.set(d.get() + 1);
            d.get() == 1
        });
        if first {
            set_ready(true);
        }
    });
    crate::rt::on_document("dragleave", |e| {
        if !has_files(e) {
            return;
        }
        let left = DEPTH.with(|d| {
            d.set((d.get() - 1).max(0));
            d.get() == 0
        });
        if left {
            reset();
        }
    });
    crate::rt::on_document("dragover", |e| {
        if !has_files(e) {
            return;
        }
        if e.default_prevented() {
            set_over(None);
            return;
        }
        e.prevent_default();
        match zone_at(e).filter(|(_, i)| usable(i)) {
            Some((zone, _)) => {
                set_drop_effect(e, "copy");
                set_over(Some(&zone));
            }
            None => {
                set_drop_effect(e, "none");
                set_over(None);
            }
        }
    });
    crate::rt::on_document("drop", |e| {
        if !has_files(e) {
            return;
        }
        let handled = e.default_prevented();
        // never let the browser open a file dropped beside a zone (it would leave the app)
        e.prevent_default();
        let target = zone_at(e);
        reset();
        if handled {
            return;
        }
        let Some((_, input)) = target else { return };
        if !usable(&input) {
            show(NOT_POSSIBLE, kit_rules::toasts::ToastKind::Err);
            return;
        }
        drop_on(&input, dropped_files(e));
    });
    crate::rt::on_document("dragend", |_| reset());
}

/// A drop place: `<div data-dropzone="input">` around `children`, feeding the file input with id `input` (or the
/// first file input inside it, when `input` is empty). [`init`] must run once.
#[component]
pub fn DropZone(
    /// The id of the file input the zone feeds ("" for the first one inside it).
    #[prop(into, optional)]
    input: String,
    /// Extra classes of the zone.
    #[prop(into, optional)]
    class: String,
    /// What the zone holds.
    children: Children,
) -> impl IntoView {
    view! { <div class=class data-dropzone=input>{children()}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};

    #[test]
    fn a_zone_names_its_input() {
        let html = render(|| view! { <DropZone input="bk-upload" class="card flat"><p>"x"</p></DropZone> });
        assert_same_html(&html, r#"<div class="card flat" data-dropzone="bk-upload"><p>x</p></div>"#);
        let html = render(|| view! { <DropZone><input type="file"/></DropZone> });
        assert_same_html(&html, r#"<div class="" data-dropzone=""><input type="file"></div>"#);
    }
}
