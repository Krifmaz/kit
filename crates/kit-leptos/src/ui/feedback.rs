//! What a page shows when it has nothing yet: [`EmptyState`] (an illustration, a title, a line and an action)
//! and skeletons ([`SkeletonLines`], [`SkeletonGrid`], [`SkeletonRail`]) in the shape of the content on its way.
//! The skeletons shimmer, and stay still when the system asks for reduced motion.

use crate::icons::icon;
use crate::ui::media::Shape;
use leptos::prelude::*;

/// The picture of an empty state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Illustration {
    /// Nothing here yet.
    #[default]
    Empty,
    /// A search found nothing.
    Search,
    /// No connection.
    Offline,
    /// Something went wrong.
    Error,
    /// All done.
    Done,
}

impl Illustration {
    /// The kit icon at its centre.
    pub fn icon(self) -> &'static str {
        match self {
            Illustration::Empty => "inbox",
            Illustration::Search => "search",
            Illustration::Offline => "wifi-off",
            Illustration::Error => "triangle-alert",
            Illustration::Done => "circle-check",
        }
    }

    /// Its CSS class word (the tone).
    pub fn class(self) -> &'static str {
        match self {
            Illustration::Empty => "empty",
            Illustration::Search => "search",
            Illustration::Offline => "offline",
            Illustration::Error => "error",
            Illustration::Done => "done",
        }
    }
}

/// The illustration: soft rings, two cards behind and the icon in front, in the theme's colours.
fn illustration(kind: Illustration) -> String {
    let ico = icon(kind.icon());
    format!(
        r#"<svg class="k-empty-art" viewBox="0 0 160 120" aria-hidden="true">
<circle cx="80" cy="60" r="56" class="k-ill-ring"/><circle cx="80" cy="60" r="40" class="k-ill-ring2"/>
<rect x="36" y="34" width="56" height="64" rx="10" class="k-ill-card" transform="rotate(-10 64 66)"/>
<rect x="68" y="30" width="56" height="64" rx="10" class="k-ill-card" transform="rotate(8 96 62)"/>
<circle cx="132" cy="24" r="4" class="k-ill-dot"/><circle cx="24" cy="92" r="3" class="k-ill-dot"/><circle cx="140" cy="96" r="2.5" class="k-ill-dot"/>
<g transform="translate(56 36) scale(2)" class="k-ill-icon">{inner}</g></svg>"#,
        inner = ico.trim_start_matches(|c| c != '>').trim_start_matches('>').trim_end_matches("</svg>")
    )
}

/// A page or list with nothing to show: what is (not) there and what to do.
#[component]
pub fn EmptyState(
    /// What is not there ("Nothing here yet").
    #[prop(into)]
    title: String,
    /// What to do about it.
    #[prop(optional, into)]
    text: Option<String>,
    /// The picture.
    #[prop(optional)]
    art: Illustration,
    /// Buttons.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=format!("k-empty k-empty-{}", art.class()) role="status">
            <div class="k-empty-pic" inner_html=illustration(art)></div>
            <h3 class="k-empty-title">{title}</h3>
            {text.map(|t| view! { <p class="k-empty-text">{t}</p> })}
            {children.map(|c| view! { <div class="k-actions">{c()}</div> })}
        </div>
    }
}

/// Lines of text on their way: `lines` bars, the last one shorter.
#[component]
pub fn SkeletonLines(
    /// How many lines.
    #[prop(optional, default = 3)]
    lines: usize,
) -> impl IntoView {
    view! {
        <div class="k-skel-lines" aria-busy="true" aria-hidden="true">
            {(0..lines)
                .map(|i| {
                    let short = i + 1 == lines && lines > 1;
                    view! { <div class="k-skel k-skel-line" class:short=short></div> }
                })
                .collect_view()}
        </div>
    }
}

/// One card on its way.
fn skeleton_card() -> impl IntoView {
    view! {
        <div class="k-card k-card-skel" aria-hidden="true">
            <div class="k-skel k-card-art"></div>
            <div class="k-card-meta"><div class="k-skel k-skel-line"></div><div class="k-skel k-skel-line short"></div></div>
        </div>
    }
}

/// A [`crate::ui::MediaGrid`] on its way: `count` cards.
#[component]
pub fn SkeletonGrid(
    /// How many cards.
    #[prop(optional, default = 12)]
    count: usize,
    /// The covers' shape.
    #[prop(optional)]
    shape: Shape,
) -> impl IntoView {
    view! { <div class=format!("k-grid k-grid-{}", shape.class()) aria-busy="true">{(0..count).map(|_| skeleton_card()).collect_view()}</div> }
}

/// A [`crate::ui::Carousel`] on its way: a title bar and `count` cards.
#[component]
pub fn SkeletonRail(
    /// How many cards.
    #[prop(optional, default = 8)]
    count: usize,
    /// The covers' shape.
    #[prop(optional)]
    shape: Shape,
) -> impl IntoView {
    view! {
        <section class="k-rail" aria-busy="true">
            <div class="k-section-head"><div class="k-skel k-skel-title"></div></div>
            <div class=format!("k-rail-track k-rail-{}", shape.class())>{(0..count).map(|_| skeleton_card()).collect_view()}</div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_state_has_its_picture_words_and_actions() {
        let html = view! { <EmptyState title="Nothing here yet" text="Add something" art=Illustration::Search><button>"Add"</button></EmptyState> }.to_html();
        for want in
            ["k-empty k-empty-search", "k-empty-art", "Nothing here yet", "Add something", "<button>Add</button>", "role=\"status\""]
        {
            assert!(html.contains(want), "{want} in {html}");
        }
        // the icon's elements, without a second <svg> around them
        assert_eq!(html.matches("<svg").count(), 1, "{html}");
    }

    #[test]
    fn every_illustration_has_an_icon() {
        for k in [Illustration::Empty, Illustration::Search, Illustration::Offline, Illustration::Error, Illustration::Done] {
            assert!(!icon(k.icon()).is_empty(), "{k:?}");
        }
    }

    #[test]
    fn skeletons_are_busy_and_shaped_like_the_content() {
        let grid = view! { <SkeletonGrid count=4 shape=Shape::Square/> }.to_html();
        assert_eq!(grid.matches("k-card-skel").count(), 4);
        assert!(grid.contains("k-grid-square") && grid.contains("aria-busy"));
        let lines = view! { <SkeletonLines lines=3/> }.to_html();
        assert_eq!(lines.matches("k-skel k-skel-line").count(), 3);
        assert_eq!(lines.matches("short").count(), 1);
    }
}
