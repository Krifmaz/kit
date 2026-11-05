//! The demo's design-system pages: Home (hero, carousels, stat cards), Browse (filter chips, a grid of covers,
//! skeletons while it "loads"), an item's page (backdrop, cover, chips, actions, a ring and a related row), Stats
//! (stat cards and rings) and States (empty states, skeletons, a sheet). The items are made up and neutral; their
//! covers are the kit's placeholder art.

use kit_leptos::icons::icon;
use kit_leptos::toasts::toast;
use kit_leptos::ui::{
    Carousel, DetailHeader, EmptyState, FilterChips, Hero, Illustration, MediaCard, MediaGrid, Page, PageHead, ProgressRing, Section,
    Shape, Sheet, SkeletonGrid, SkeletonLines, SkeletonRail, StatCard,
};
use leptos::prelude::*;

/// The made-up items: a title, a kind and a year.
pub const ITEMS: [(&str, &str, u32); 24] = [
    ("Northern Lights", "Story", 2024),
    ("Paper Boats", "Guide", 2023),
    ("Glass Garden", "Story", 2022),
    ("Quiet Hours", "Notes", 2024),
    ("Salt and Cedar", "Guide", 2021),
    ("The Long Weekend", "Story", 2020),
    ("Copper Sky", "Notes", 2024),
    ("Field Notes", "Notes", 2019),
    ("Small Wonders", "Guide", 2023),
    ("Harbour Lights", "Story", 2018),
    ("Open Windows", "Guide", 2024),
    ("Blue Hour", "Story", 2022),
    ("Paper Moon Atlas", "Guide", 2021),
    ("Slow Rivers", "Notes", 2020),
    ("Lantern Street", "Story", 2023),
    ("Wild Orchard", "Guide", 2024),
    ("Summer Static", "Notes", 2022),
    ("Echo Valley", "Story", 2019),
    ("Morning Pages", "Notes", 2024),
    ("A Map of Clouds", "Guide", 2020),
    ("Stone and Feather", "Story", 2021),
    ("Late Light", "Notes", 2023),
    ("Winter Harbour", "Story", 2024),
    ("Tiny Machines", "Guide", 2022),
];

/// One item's card.
fn card(i: usize, shape: Shape) -> impl IntoView {
    let (title, kind, year) = ITEMS[i % ITEMS.len()];
    let badge = (year == 2024).then_some("New");
    let progress = (i % 5 == 1).then_some(0.35 + (i % 3) as f64 * 0.2);
    view! {
        <MediaCard
            title=title
            href=format!("#/item/{i}")
            seed=i as u32 + 1
            subtitle=format!("{kind} · {year}")
            badge=badge.map(String::from)
            progress=progress
            shape=shape
        />
    }
}

/// Home: the featured item, two carousels and the numbers.
#[component]
pub fn Home() -> impl IntoView {
    view! {
        <Page>
            <Hero title=ITEMS[0].0 eyebrow="Featured" text="A made-up item to show the hero banner: the cover's colours blurred behind it, the title, a line of text and the actions. Every cover here is the kit's placeholder art." seed=1>
                <a class="btn primary" href="#/item/0"><span inner_html=icon("play")></span>"Open"</a>
                <button class="btn" on:click=|_| toast("Added to your list", "ok")><span inner_html=icon("plus")></span>"My list"</button>
            </Hero>
            <Carousel title="Trending" see_all="#/browse">{(1..13).map(|i| card(i, Shape::Poster)).collect_view()}</Carousel>
            <Carousel title="Recently added" see_all="#/browse" shape=Shape::Wide>{(6..16).map(|i| card(i, Shape::Wide)).collect_view()}</Carousel>
            <Section title="At a glance" see_all="#/stats">
                <div class="k-stats">
                    <StatCard label="Items" value="1,284" icon="layers" delta=0.12 hint="this month"/>
                    <StatCard label="Opened today" value="86" icon="eye" delta=-0.04/>
                    <StatCard label="Storage" value="41.2 GB" icon="hard-drive" hint="of 100 GB"/>
                    <StatCard label="People" value="7" icon="users" delta=0.0/>
                </div>
            </Section>
            <Carousel title="Squares" shape=Shape::Square>{(12..22).map(|i| card(i, Shape::Square)).collect_view()}</Carousel>
        </Page>
    }
}

/// Browse: filter chips over a grid of covers; a moment of skeletons when the filter changes.
#[component]
pub fn Browse() -> impl IntoView {
    let kinds: Vec<String> = ["All", "Story", "Guide", "Notes"].iter().map(|s| s.to_string()).collect();
    let kind = RwSignal::new("All".to_string());
    let loading = RwSignal::new(false);
    Effect::new(move |prev: Option<String>| {
        let k = kind.get();
        if prev.is_some_and(|p| p != k) {
            loading.set(true);
            kit_leptos::rt::after(450, move || loading.set(false));
        }
        k
    });
    view! {
        <Page>
            <PageHead title="Browse" subtitle="Every item, three to a row on a phone.">
                <button class="btn" on:click=|_| toast("Sorted by name", "info")><span inner_html=icon("sliders-horizontal")></span>"Sort"</button>
            </PageHead>
            <FilterChips options=kinds selected=kind label="Kind"/>
            {move || {
                if loading.get() {
                    return view! { <SkeletonGrid count=12/> }.into_any();
                }
                let k = kind.get();
                let shown: Vec<usize> = (0..ITEMS.len()).filter(|i| k == "All" || ITEMS[*i].1 == k).collect();
                view! { <MediaGrid label="Items">{shown.into_iter().map(|i| card(i, Shape::Poster)).collect_view()}</MediaGrid> }.into_any()
            }}
        </Page>
    }
}

/// An item's page.
#[component]
pub fn Item(
    /// The item's number.
    n: usize,
) -> impl IntoView {
    let (title, kind, year) = ITEMS[n % ITEMS.len()];
    let done = 0.2 + (n % 7) as f64 * 0.11;
    view! {
        <Page>
            <DetailHeader
                title=title
                subtitle=format!("{kind} · {year} · made up for the demo")
                seed=n as u32 + 1
                chips=vec![year.to_string(), kind.to_string(), "1 h 52 min".to_string(), "English".to_string()]
            >
                <button class="btn primary" on:click=|_| toast("Would open it", "info")><span inner_html=icon("play")></span>"Open"</button>
                <button class="btn" on:click=|_| toast("Starred", "ok")><span inner_html=icon("star")></span>"Star"</button>
                <button class="btn icon" aria-label="Share" on:click=|_| toast("Link copied", "ok") inner_html=icon("share-2")></button>
            </DetailHeader>
            <div class="demo-detail">
                <div class="card">
                    <h3>"About"</h3>
                    <p class="muted">"The detail page: a backdrop from the cover, the cover, the title, metadata chips and the actions. On a phone it stacks and centres; from 768 px the cover sits beside the text."</p>
                </div>
                <div class="card demo-progress">
                    <ProgressRing value=done label="Read"/>
                    <div class="grow"><h3>"Progress"</h3><p class="muted">"Where you left off, as a ring."</p></div>
                </div>
            </div>
            <Carousel title="More like this">{(0..10).map(|i| card((n + i + 1) % ITEMS.len(), Shape::Poster)).collect_view()}</Carousel>
        </Page>
    }
}

/// Stats: stat cards and rings.
#[component]
pub fn Stats() -> impl IntoView {
    let sync = RwSignal::new(0.62);
    view! {
        <Page>
            <PageHead title="Stats" subtitle="Stat cards and progress rings.">
                <button class="btn" on:click=move |_| sync.update(|v: &mut f64| *v = if *v >= 0.99 { 0.1 } else { (*v + 0.13).min(1.0) })>
                    <span inner_html=icon("refresh-cw")></span>"Step"
                </button>
            </PageHead>
            <div class="k-stats">
                <StatCard label="Items" value="1,284" icon="layers" delta=0.12 hint="this month"/>
                <StatCard label="Opened today" value="86" icon="eye" delta=-0.04 hint="since yesterday"/>
                <StatCard label="Storage used" value="41.2 GB" icon="hard-drive" hint="of 100 GB"/>
                <StatCard label="People" value="7" icon="users" delta=0.0 hint="no change"/>
                <StatCard label="Uptime" value="99.98%" icon="activity" delta=0.002/>
                <StatCard label="Queue" value="3" icon="clock" hint="waiting"/>
            </div>
            <Section title="Rings">
                <div class="card k-rings">
                    <ProgressRing value=sync label="Sync"/>
                    <ProgressRing value=0.41 label="Storage" tone="info"/>
                    <ProgressRing value=0.92 label="Health" tone="ok" size=88/>
                    <ProgressRing value=0.18 label="Space left" tone="warn"/>
                    <ProgressRing value=0.05 label="Errors" tone="err" size=56 stroke=6/>
                </div>
            </Section>
        </Page>
    }
}

/// States: empty states, skeletons and a sheet.
#[component]
pub fn States() -> impl IntoView {
    let sheet = RwSignal::new(false);
    view! {
        <Page>
            <PageHead title="States" subtitle="What a page shows before it has something to show.">
                <button class="btn primary" id="open-sheet" on:click=move |_| sheet.set(true)><span inner_html=icon("square-stack")></span>"Open a sheet"</button>
            </PageHead>
            <div class="demo-empties">
                <EmptyState title="Nothing here yet" text="Things you add show up here." art=Illustration::Empty>
                    <button class="btn primary" on:click=|_| toast("Would add one", "info")><span inner_html=icon("plus")></span>"Add one"</button>
                </EmptyState>
                <EmptyState title="No results" text="Try fewer words, or another spelling." art=Illustration::Search/>
                <EmptyState title="You're offline" text="What you saw last is still here. New things come when the connection is back." art=Illustration::Offline/>
                <EmptyState title="All done" text="Nothing waits for you." art=Illustration::Done/>
            </div>
            <Section title="Loading">
                <SkeletonRail count=8/>
                <div class="card"><SkeletonLines lines=4/></div>
            </Section>
            <Section title="A grid on its way"><SkeletonGrid count=6 shape=Shape::Square/></Section>
            <Sheet open=sheet title="A sheet">
                <p class="muted">"On a phone a sheet comes up from the bottom, full width, clear of the home indicator; on wider screens it is a panel in the middle. Escape, the close button or a tap outside closes it."</p>
                <div class="k-actions">
                    <button class="btn primary" on:click=move |_| { sheet.set(false); toast("Done", "ok"); }>"Done"</button>
                    <button class="btn" on:click=move |_| sheet.set(false)>"Cancel"</button>
                </div>
            </Sheet>
        </Page>
    }
}
