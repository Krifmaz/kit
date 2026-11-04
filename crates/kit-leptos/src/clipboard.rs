//! Copy to the clipboard, with a fallback for a server reached over plain http (no clipboard API there).
//!
//! The toasts say the configured words ([`ToastWords::copied`](kit_rules::toasts::ToastWords::copied) and
//! [`ToastWords::copy_failed`](kit_rules::toasts::ToastWords::copy_failed)).

use crate::toasts::show;
use kit_rules::toasts::ToastKind;
use wasm_bindgen::JsCast;

pub use kit_rules::toasts::COPY_FAILED;

/// Copies `t` and says so ("Copied to clipboard").
pub fn copy_text(t: &str) {
    copy_then(t, crate::config::get().words.toasts.copied);
}

/// [`copy_text`] with `copied` as the toast when it worked (a share link says what the link does instead).
pub fn copy_then(t: &str, copied: &str) {
    let Some(w) = crate::rt::window() else { return };
    let has_api = js_sys::Reflect::get(&w.navigator(), &"clipboard".into()).map(|c| !c.is_undefined() && !c.is_null()).unwrap_or(false);
    if !has_api {
        // no await before this: the browser still counts the click that asked for it
        done(copy_by_selection(t), copied);
        return;
    }
    let (t, copied) = (t.to_string(), copied.to_string());
    crate::rt::spawn(async move {
        let clip = w.navigator().clipboard();
        let ok = wasm_bindgen_futures::JsFuture::from(clip.write_text(&t)).await.is_ok() || copy_by_selection(&t);
        done(ok, &copied);
    });
}

fn done(ok: bool, copied: &str) {
    if ok {
        show(copied, ToastKind::Ok);
    } else {
        show(crate::config::get().words.toasts.copy_failed, ToastKind::Err);
    }
}

/// The old way: the text in an off-screen box, selected, `document.execCommand('copy')`. Inside an open dialog,
/// so focus stays where the user is.
pub fn copy_by_selection(t: &str) -> bool {
    let Some(doc) = crate::rt::document() else { return false };
    let Ok(area) = doc.create_element("textarea").map(|e| e.unchecked_into::<web_sys::HtmlTextAreaElement>()) else { return false };
    area.set_value(t);
    let _ = area.set_attribute("readonly", "");
    let _ = area.set_attribute("style", "position:fixed;top:-1000px;left:0;width:1px;height:1px;opacity:0");
    let host: web_sys::Element = match crate::rt::qs("#modal-root .modal-bg:last-child .modal") {
        Some(m) => m,
        None => match doc.body() {
            Some(b) => b.into(),
            None => return false,
        },
    };
    let _ = host.append_child(&area);
    area.select();
    // iOS selects a read-only box only with a range
    let _ = area.set_selection_range(0, t.encode_utf16().count() as u32);
    let ok = js_sys::Reflect::get(&doc, &"execCommand".into())
        .ok()
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
        .and_then(|f| f.call1(&doc, &"copy".into()).ok())
        .and_then(|r| r.as_bool())
        .unwrap_or(false);
    area.remove();
    ok
}
