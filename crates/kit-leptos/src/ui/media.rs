//! Covers and the pages built from them: [`MediaCard`] (a poster, square or wide cover with its title), a
//! [`MediaGrid`] of them, a [`Carousel`] row ("Trending", "Recently added"), the [`Hero`] banner with blurred
//! backdrop art, the [`DetailHeader`] of an item's page, and [`Chip`]s for metadata and filters.
//!
//! Covers lift and brighten under a mouse, press in under a finger, and show a focus ring for the keyboard.
//! An item without art gets [`placeholder_art`] from its seed and title.

use crate::icons::icon;
use crate::ui::art::placeholder_art;
use leptos::prelude::*;

/// A cover's shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shape {
    /// 2 : 3, a poster or a book.
    #[default]
    Poster,
    /// 1 : 1, an album or a photo.
    Square,
    /// 16 : 9, a video or a page.
    Wide,
}

impl Shape {
    /// Its CSS class word.
    pub fn class(self) -> &'static str {
        match self {
            Shape::Poster => "poster",
            Shape::Square => "square",
            Shape::Wide => "wide",
        }
    }
}

/// The art to draw: `art` when given and not empty, else the placeholder of `seed` and `title`.
fn art_or_placeholder(art: Option<String>, seed: u32, title: &str) -> String {
    art.filter(|a| !a.is_empty()).unwrap_or_else(|| placeholder_art(seed, title))
}

/// A progress value between 0 and 1 (anything else is clamped; not a number is 0).
pub(crate) fn unit(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v.clamp(0.0, 1.0)
    }
}

/// `url("…")` for a CSS background (the URL's own quotes escaped).
fn css_url(u: &str) -> String {
    format!("background-image:url(\"{}\")", u.replace('"', "%22"))
}

/// One cover with its title: a link to the item.
#[component]
pub fn MediaCard(
    /// The item's title.
    #[prop(into)]
    title: String,
    /// Where it goes.
    #[prop(into)]
    href: String,
    /// The cover's URL; [`placeholder_art`] when not given.
    #[prop(optional, into)]
    art: Option<String>,
    /// The placeholder's seed (the item's id).
    #[prop(optional)]
    seed: u32,
    /// A line under the title (a year, a count, a name).
    #[prop(optional, into)]
    subtitle: MaybeProp<String>,
    /// A short badge on the cover ("New", "4K", "12").
    #[prop(optional, into)]
    badge: MaybeProp<String>,
    /// How far along it is (0–1): a bar along the cover's foot.
    #[prop(optional, into)]
    progress: MaybeProp<f64>,
    /// The cover's shape.
    #[prop(optional)]
    shape: Shape,
) -> impl IntoView {
    let src = art_or_placeholder(art, seed, &title);
    view! {
        <a class=format!("k-card k-{}", shape.class()) href=href role="listitem">
            <div class="k-card-art">
                <img src=src alt="" loading="lazy" decoding="async"/>
                {move || badge.get().map(|b| view! { <span class="k-card-badge">{b}</span> })}
                <span class="k-card-play" aria-hidden="true" inner_html=icon("play")></span>
                {move || progress.get().map(|p| view! { <div class="k-card-progress"><i style=format!("width:{:.1}%", unit(p) * 100.0)></i></div> })}
            </div>
            <div class="k-card-meta">
                <div class="k-card-title" title=title.clone()>{title.clone()}</div>
                {move || subtitle.get().map(|s| view! { <div class="k-card-sub">{s}</div> })}
            </div>
        </a>
    }
}

/// A grid of [`MediaCard`]s that fits as many columns as the width allows (three on a phone).
#[component]
pub fn MediaGrid(
    /// The covers' shape (sets the column width).
    #[prop(optional)]
    shape: Shape,
    /// The name of the list for screen readers.
    #[prop(optional, into)]
    label: Option<String>,
    /// The cards.
    children: Children,
) -> impl IntoView {
    view! { <div class=format!("k-grid k-grid-{}", shape.class()) role="list" aria-label=label>{children()}</div> }
}

/// A titled row of cards that scrolls sideways (by touch, trackpad, or the arrow buttons on wider screens), each
/// card snapping into place.
#[component]
pub fn Carousel(
    /// The row's title ("Trending").
    #[prop(into)]
    title: String,
    /// Where "See all" goes; no link when not given.
    #[prop(optional, into)]
    see_all: Option<String>,
    /// The link's words.
    #[prop(optional, into, default = "See all".to_string())]
    see_all_label: String,
    /// The arrow buttons' names for screen readers.
    #[prop(optional, into, default = ("Scroll back".to_string(), "Scroll on".to_string()))]
    arrow_labels: (String, String),
    /// The cards' shape (sets their width).
    #[prop(optional)]
    shape: Shape,
    /// The cards.
    children: Children,
) -> impl IntoView {
    let track = NodeRef::<leptos::html::Div>::new();
    let scroll = move |dir: f64| {
        if let Some(t) = track.get() {
            // most of a screenful; the track's scroll-behavior makes it smooth unless motion is reduced
            let by = f64::from(t.client_width()) * 0.85 * dir;
            t.scroll_by_with_x_and_y(by, 0.0);
        }
    };
    let (back, on) = arrow_labels;
    view! {
        <section class="k-rail">
            <div class="k-section-head">
                <h2>{title.clone()}</h2>
                <div class="k-rail-nav">
                    {see_all.map(|h| view! { <a class="k-link" href=h>{see_all_label}<span class="k-ico" aria-hidden="true" inner_html=icon("chevron-right")></span></a> })}
                    <button type="button" class="k-icon-btn k-rail-arrow" aria-label=back on:click=move |_| scroll(-1.0) inner_html=icon("chevron-left")></button>
                    <button type="button" class="k-icon-btn k-rail-arrow" aria-label=on on:click=move |_| scroll(1.0) inner_html=icon("chevron-right")></button>
                </div>
            </div>
            <div class=format!("k-rail-track k-rail-{}", shape.class()) role="list" aria-label=title node_ref=track>{children()}</div>
        </section>
    }
}

/// A banner across the top of a page: the art blurred behind, the cover beside the title, a line of text and the
/// actions.
#[component]
pub fn Hero(
    /// The title.
    #[prop(into)]
    title: String,
    /// A small line above the title ("Featured").
    #[prop(optional, into)]
    eyebrow: Option<String>,
    /// A line or two under the title.
    #[prop(optional, into)]
    text: Option<String>,
    /// The art (backdrop and cover); [`placeholder_art`] when not given.
    #[prop(optional, into)]
    art: Option<String>,
    /// A wide backdrop instead of the blurred cover.
    #[prop(optional, into)]
    backdrop: Option<String>,
    /// The placeholder's seed.
    #[prop(optional)]
    seed: u32,
    /// Buttons.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let cover = art_or_placeholder(art, seed, &title);
    let back = backdrop.filter(|b| !b.is_empty()).unwrap_or_else(|| cover.clone());
    view! {
        <section class="k-hero">
            <div class="k-hero-backdrop" style=css_url(&back) aria-hidden="true"></div>
            <div class="k-hero-body">
                <img class="k-hero-cover" src=cover alt="" decoding="async"/>
                <div class="k-hero-text">
                    {eyebrow.map(|e| view! { <div class="k-eyebrow">{e}</div> })}
                    <h2 class="k-hero-title">{title}</h2>
                    {text.map(|t| view! { <p class="k-hero-line">{t}</p> })}
                    {children.map(|c| view! { <div class="k-actions">{c()}</div> })}
                </div>
            </div>
        </section>
    }
}

/// The top of an item's page: a wide backdrop, the cover, the title, metadata chips and the actions.
#[component]
pub fn DetailHeader(
    /// The title.
    #[prop(into)]
    title: String,
    /// A line under the title (who made it, when).
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// The cover; [`placeholder_art`] when not given.
    #[prop(optional, into)]
    art: Option<String>,
    /// A wide backdrop; the blurred cover when not given.
    #[prop(optional, into)]
    backdrop: Option<String>,
    /// The placeholder's seed.
    #[prop(optional)]
    seed: u32,
    /// The cover's shape.
    #[prop(optional)]
    shape: Shape,
    /// Metadata chips ("2024", "1 h 52 min", "Drama").
    #[prop(optional)]
    chips: Vec<String>,
    /// Buttons.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let cover = art_or_placeholder(art, seed, &title);
    let (back, blurred) = match backdrop.filter(|b| !b.is_empty()) {
        Some(b) => (b, false),
        None => (cover.clone(), true),
    };
    view! {
        <section class="k-detail">
            <div class="k-detail-backdrop" class:blurred=blurred style=css_url(&back) aria-hidden="true"></div>
            <div class="k-detail-body">
                <img class=format!("k-detail-cover k-{}", shape.class()) src=cover alt="" decoding="async"/>
                <div class="k-detail-text">
                    <h1 class="k-detail-title">{title}</h1>
                    {subtitle.map(|s| view! { <p class="k-sub">{s}</p> })}
                    <div class="k-chips">{chips.into_iter().map(|c| view! { <Chip label=c/> }).collect_view()}</div>
                    {children.map(|c| view! { <div class="k-actions">{c()}</div> })}
                </div>
            </div>
        </section>
    }
}

/// A small pill of metadata, with an optional icon.
#[component]
pub fn Chip(
    /// Its words.
    #[prop(into)]
    label: String,
    /// A kit icon name.
    #[prop(optional, into)]
    icon: Option<String>,
) -> impl IntoView {
    view! {
        <span class="k-chip">
            {icon.map(|i| view! { <span class="k-ico" aria-hidden="true" inner_html=crate::icons::icon(&i)></span> })}
            {label}
        </span>
    }
}

/// A row of filter chips, one of them on: the row scrolls sideways on phones rather than wrapping.
#[component]
pub fn FilterChips(
    /// The choices' words.
    options: Vec<String>,
    /// The chosen one.
    selected: RwSignal<String>,
    /// The row's name for screen readers.
    #[prop(optional, into)]
    label: Option<String>,
) -> impl IntoView {
    view! {
        <div class="k-chips k-chips-scroll" role="group" aria-label=label>
            {options
                .into_iter()
                .map(|o| {
                    let (o1, o2, o3) = (o.clone(), o.clone(), o.clone());
                    view! {
                        <button
                            type="button"
                            class="k-chip k-chip-btn"
                            class:on=move || selected.get() == o1
                            aria-pressed=move || if selected.get() == o2 { "true" } else { "false" }
                            on:click=move |_| selected.set(o3.clone())
                        >
                            {o}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_without_art_gets_its_placeholder() {
        let html = view! { <MediaCard title="Paper boats" href="#/i/4" seed=4 subtitle="2019" badge="New" progress=0.5/> }.to_html();
        for want in ["k-card k-poster", "href=\"#/i/4\"", "data:image/svg+xml", "Paper boats", "2019", "k-card-badge", "width:50.0%"] {
            assert!(html.contains(want), "{want} in {html}");
        }
        let with = view! { <MediaCard title="x" href="#" art="/a.jpg" shape=Shape::Wide/> }.to_html();
        assert!(with.contains("src=\"/a.jpg\"") && with.contains("k-wide"), "{with}");
    }

    #[test]
    fn progress_is_kept_between_nothing_and_all() {
        assert_eq!(unit(-1.0), 0.0);
        assert_eq!(unit(2.0), 1.0);
        assert_eq!(unit(f64::NAN), 0.0);
        assert_eq!(unit(0.25), 0.25);
    }

    #[test]
    fn the_hero_and_the_detail_header_blur_the_cover_behind() {
        let hero = view! { <Hero title="Glass garden" eyebrow="Featured" text="A line" seed=2/> }.to_html();
        for want in ["k-hero-backdrop", "background-image:url(", "Featured", "Glass garden", "k-hero-cover"] {
            assert!(hero.contains(want), "{want} in {hero}");
        }
        let detail = view! { <DetailHeader title="T" chips=vec!["2024".into(), "Drama".into()]/> }.to_html();
        assert!(detail.contains("blurred") && detail.matches("k-chip").count() >= 2, "{detail}");
        let wide = view! { <DetailHeader title="T" backdrop="/b.jpg"/> }.to_html();
        assert!(!wide.contains("blurred"), "{wide}");
    }

    #[test]
    fn a_carousel_has_its_title_arrows_and_see_all() {
        let html = view! { <Carousel title="Trending" see_all="#/all"><MediaCard title="a" href="#"/></Carousel> }.to_html();
        for want in ["k-rail-track", "Trending", "See all", "aria-label=\"Scroll back\"", "aria-label=\"Scroll on\""] {
            assert!(html.contains(want), "{want} in {html}");
        }
    }

    #[test]
    fn filter_chips_say_which_one_is_on() {
        let sel = RwSignal::new("B".to_string());
        let html = view! { <FilterChips options=vec!["A".into(), "B".into()] selected=sel/> }.to_html();
        assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1, "{html}");
        assert_eq!(html.matches("aria-pressed=\"false\"").count(), 1, "{html}");
    }
}
