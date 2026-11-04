//! Encore's inline SVG icons, the same markup byte for byte.
//!
//! From Encore's `web-rs/src/ui.rs` (`icon`, `p`, `EYE`, `EYE_OFF`): only the icons the kit's components draw.
//! [`icon`] gives the SVG as a string (for `inner_html` and for hand-written pages), [`Icon`] draws it as a
//! Leptos view with the same markup.

use crate::html::html;
use leptos::prelude::*;

/// The attributes every stroked icon shares. From `ui.rs` `p`.
const OPEN: &str =
    r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#;

/// The paths of icon `name` inside Encore's common `<svg>` (`""` for a name the kit does not have).
pub fn paths(name: &str) -> &'static str {
    match name {
        "plus" => r#"<path d="M12 5v14M5 12h14"/>"#,
        "check" => r#"<path d="M20 6 9 17l-5-5"/>"#,
        "x" => r#"<path d="M18 6 6 18M6 6l12 12"/>"#,
        "trash" => r#"<path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/>"#,
        "refresh" => r#"<path d="M21 12a9 9 0 1 1-2.64-6.36L21 8M21 3v5h-5"/>"#,
        "copy" => r#"<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"#,
        "external" => r#"<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6M15 3h6v6M10 14 21 3"/>"#,
        "download" => r#"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5M12 15V3"/>"#,
        "chevron" => r#"<polyline points="6 9 12 15 18 9"/>"#,
        "offline" => {
            r#"<path d="m2 2 20 20M8.5 16.4a5 5 0 0 1 7 0M5 12.9a10 10 0 0 1 5.2-2.7M19 12.9a10 10 0 0 0-2-1.5M2 8.8a15 15 0 0 1 4.2-2.6M22 8.8a15 15 0 0 0-11.3-3.8M12 20h.01"/>"#
        }
        "alert" => {
            r#"<path d="M10.29 3.86 1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><path d="M12 9v4M12 17h.01"/>"#
        }
        "bug" => {
            r#"<path d="M9 7V6a3 3 0 0 1 6 0v1"/><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6z"/><path d="M12 20v-9M6 13H2M22 13h-4M3 21c0-2.1 1.7-3.9 3.8-4M20.2 17c2.1.1 3.8 1.9 3.8 4M3 5c0 2.1 1.6 3.8 3.5 4M21 5c0 2.1-1.6 3.8-3.5 4M8 2l1.9 1.9M16 2l-1.9 1.9"/>"#
        }
        "settings" => {
            r#"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>"#
        }
        _ => "",
    }
}

/// `icon.<name>`: the whole SVG as Encore writes it (`""` for a name the kit does not have).
pub fn icon(name: &str) -> String {
    let p = paths(name);
    if p.is_empty() {
        return String::new();
    }
    format!("{OPEN}{p}</svg>")
}

/// The chevron of a fold, or `▾` where the icon is missing (Encore's `chev`).
pub fn chevron() -> String {
    let c = icon("chevron");
    if c.is_empty() {
        "▾".to_string()
    } else {
        c
    }
}

/// The password eye (shown). From `ui.rs` `EYE`.
pub const EYE: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></svg>"#;
/// The password eye (hidden). From `ui.rs` `EYE_OFF`.
pub const EYE_OFF: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.9 17.9A10.4 10.4 0 0 1 12 19c-6.4 0-10-7-10-7a18.5 18.5 0 0 1 5.1-5.9M9.9 5.2A9.6 9.6 0 0 1 12 5c6.4 0 10 7 10 7a18.6 18.6 0 0 1-2.2 3.2M14.1 14.1a3 3 0 1 1-4.2-4.2"/><path d="m2 2 20 20"/></svg>"#;

/// Icon `name` as a view: Encore's `<svg>` with the icon's paths.
#[component]
pub fn Icon(
    /// The icon's name, as Encore's `icon.<name>`.
    name: &'static str,
) -> impl IntoView {
    view! { <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(paths(name))}>{""}</svg> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_draws_the_same_svg() {
        for name in ["copy", "x", "alert", "refresh", "offline", "chevron", "check", "plus", "bug", "external", "download", "trash"] {
            let html = view! { <Icon name=name/> }.to_html();
            assert!(crate::testing::same_html(&html, &icon(name)), "{name}: {html}");
        }
        assert_eq!(icon("nothing"), "");
        assert!(icon("copy").starts_with(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor""#));
    }
}
