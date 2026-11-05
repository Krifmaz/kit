//! The kit's inline SVG icons: Lucide's stroked 24 × 24 outlines (design/lucide) that take the text colour
//! (`currentColor`). [`icon`] gives the whole `<svg>` as a string, [`Icon`] as a view.

use crate::html::html;
use leptos::prelude::*;

/// The attributes every stroked icon shares.
const OPEN: &str =
    r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#;

/// The kit's short names for its first icons, and the Lucide icon each one draws.
pub const ALIASES: [(&str, &str); 8] = [
    ("trash", "trash-2"),
    ("refresh", "refresh-cw"),
    ("external", "external-link"),
    ("chevron", "chevron-down"),
    ("offline", "wifi-off"),
    ("alert", "triangle-alert"),
    ("home", "house"),
    ("more", "ellipsis"),
];

/// The SVG elements of icon `name` inside the common `<svg>` (`""` for a name the kit does not have): any Lucide
/// name the kit ships ([`crate::lucide::NAMES`]) or one of the kit's short names ([`ALIASES`]).
pub fn paths(name: &str) -> &'static str {
    let name = ALIASES.iter().find(|(a, _)| *a == name).map(|(_, l)| *l).unwrap_or(name);
    crate::lucide::elements(name)
}

/// `icon.<name>`: the whole SVG (`""` for a name the kit does not have).
pub fn icon(name: &str) -> String {
    let p = paths(name);
    if p.is_empty() {
        return String::new();
    }
    format!("{OPEN}{p}</svg>")
}

/// The chevron of a fold, or `▾` where the icon is missing.
pub fn chevron() -> String {
    let c = icon("chevron");
    if c.is_empty() {
        "▾".to_string()
    } else {
        c
    }
}

/// The password eye (shown).
pub const EYE: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></svg>"#;
/// The password eye (hidden).
pub const EYE_OFF: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M17.9 17.9A10.4 10.4 0 0 1 12 19c-6.4 0-10-7-10-7a18.5 18.5 0 0 1 5.1-5.9M9.9 5.2A9.6 9.6 0 0 1 12 5c6.4 0 10 7 10 7a18.6 18.6 0 0 1-2.2 3.2M14.1 14.1a3 3 0 1 1-4.2-4.2"/><path d="m2 2 20 20"/></svg>"#;

/// Icon `name` as a view: the common `<svg>` with the icon's paths.
#[component]
pub fn Icon(
    /// The icon's name, as [`icon`] takes it.
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
        // every alias names a shipped icon, and every shipped icon draws
        for (a, l) in ALIASES {
            assert_eq!(paths(a), paths(l), "{a}");
            assert!(!paths(l).is_empty(), "{l}");
        }
        assert!(crate::lucide::NAMES.iter().all(|n| !icon(n).is_empty()));
        assert!(icon("copy").starts_with(r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor""#));
    }
}
