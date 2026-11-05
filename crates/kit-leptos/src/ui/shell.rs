//! The app's frame: [`AppShell`] lays out the navigation for the screen it is on, [`Page`] fades each new page
//! in, [`PageHead`] and [`Section`] title a page and its parts.
//!
//! | Width | Navigation |
//! | --- | --- |
//! | below 768 px (phones) | a glass top app bar (back button or logo, title, actions) and a glass bottom tab bar: up to five sections; with more, four and a More button that opens the rest in a sheet |
//! | 768–1023 px (tablets) | a navigation rail: icons with short labels, the app bar on top of the page |
//! | from 1024 px | the full sidebar: logo and name, every section, the app's footer (account, settings) |
//!
//! The bars keep clear of notches and the home indicator (`env(safe-area-inset-*)`; the page needs
//! `<meta name="viewport" content="…, viewport-fit=cover">`).

use crate::icons::icon;
use crate::ui::sheet::Sheet;
use leptos::prelude::*;

/// One section of the app's navigation.
#[derive(Clone, Debug, PartialEq)]
pub struct NavItem {
    /// Where it goes (`#/library`, `/library`).
    pub href: String,
    /// Its name.
    pub label: String,
    /// A kit icon name ([`crate::icons::icon`]).
    pub icon: String,
}

impl NavItem {
    /// A section called `label` at `href` with kit icon `icon`.
    pub fn new(href: impl Into<String>, label: impl Into<String>, icon: impl Into<String>) -> NavItem {
        NavItem { href: href.into(), label: label.into(), icon: icon.into() }
    }

    /// Whether the page at `current` (an href in the same form) belongs to this section: the same address, or
    /// one under it (`#/library/3` under `#/library`). The home section (`#/`, `/`, `#`) is active only on
    /// itself.
    pub fn is_active(&self, current: &str) -> bool {
        let home = |s: &str| matches!(s.trim_end_matches('/'), "" | "#");
        if home(&self.href) {
            return home(current);
        }
        let base = self.href.trim_end_matches('/');
        current == base || current.strip_prefix(base).is_some_and(|rest| rest.starts_with(['/', '?']))
    }
}

/// How the tab bar splits `count` sections with room for `tabs`: the ones it shows, and whether a More button
/// holds the rest.
pub fn tab_split(count: usize, tabs: usize) -> (usize, bool) {
    let tabs = tabs.clamp(2, 5);
    if count <= tabs {
        (count, false)
    } else {
        (tabs - 1, true)
    }
}

/// One link of the sidebar, rail or tab bar.
fn nav_link(item: NavItem, active: Signal<String>, class: &'static str, on_go: Option<Callback<()>>) -> impl IntoView {
    let it = item.clone();
    let on = move || it.is_active(&active.get());
    view! {
        <a
            class=class
            class:active=on.clone()
            href=item.href.clone()
            aria-current=move || on().then_some("page")
            on:click=move |_| {
                if let Some(cb) = on_go {
                    cb.run(());
                }
            }
        >
            <span class="k-ico" aria-hidden="true" inner_html=icon(&item.icon)></span>
            <span class="k-nav-label">{item.label.clone()}</span>
        </a>
    }
}

/// The app's frame: the navigation for the screen's width around the page (`children`, usually the router's
/// output wrapped in [`Page`]).
#[component]
pub fn AppShell(
    /// The app's name, at the top of the sidebar.
    #[prop(into)]
    brand: String,
    /// The app's logo (an image URL), next to its name and in the phone's app bar.
    #[prop(optional, into)]
    logo: Option<String>,
    /// The home address the logo links to.
    #[prop(optional, into, default = "#/".to_string())]
    home: String,
    /// The app's sections, in order.
    nav: Vec<NavItem>,
    /// The address of the page on screen, in the form of the sections' hrefs.
    #[prop(into)]
    active: Signal<String>,
    /// The page's title in the app bar (the app's name when not given).
    #[prop(optional, into)]
    title: MaybeProp<String>,
    /// Where the app bar's back button goes; no back button when not given (phones show the logo instead).
    #[prop(optional, into)]
    back: MaybeProp<String>,
    /// The app bar's buttons (search, account…), on every width.
    #[prop(optional, into)]
    actions: Option<ViewFn>,
    /// The sidebar's footer from 1024 px (the account, sign out).
    #[prop(optional, into)]
    side_foot: Option<ViewFn>,
    /// How many tabs fit in the phone's tab bar (2–5, default 5); with more sections the last tab is More.
    #[prop(optional, default = 5)]
    tabs: usize,
    /// The words of the More tab and its sheet.
    #[prop(optional, into, default = "More".to_string())]
    more_label: String,
    /// The back button's name for screen readers.
    #[prop(optional, into, default = "Back".to_string())]
    back_label: String,
    /// The page.
    children: Children,
) -> impl IntoView {
    let (shown, more) = tab_split(nav.len(), tabs);
    let more_open = RwSignal::new(false);
    let rest: Vec<NavItem> = nav[shown..].to_vec();
    let rest_for_active = rest.clone();
    let more_active = move || more && rest_for_active.iter().any(|i| i.is_active(&active.get()));
    let brand_for_title = brand.clone();
    let title_text = move || title.get().filter(|t| !t.is_empty()).unwrap_or_else(|| brand_for_title.clone());
    let logo_img = logo.clone().map(|src| view! { <img class="k-logo" src=src alt="" width="28" height="28"/> });
    let logo_bar = logo;
    let home_bar = home.clone();
    let side_links = nav.iter().cloned().map(|i| nav_link(i, active, "k-side-link", None)).collect_view();
    let tab_links = nav[..shown].iter().cloned().map(|i| nav_link(i, active, "k-tab", None)).collect_view();
    let close = Callback::new(move |_| more_open.set(false));
    let more_links = move || rest.clone().into_iter().map(|i| nav_link(i, active, "k-sheet-link", Some(close))).collect_view();
    let more_title = more_label.clone();
    view! {
        <div class="k-shell">
            <aside class="k-side" aria-label=brand.clone()>
                <a class="k-side-brand" href=home>
                    {logo_img}
                    <span class="k-side-name">{brand.clone()}</span>
                </a>
                <nav class="k-side-nav">{side_links}</nav>
                <div class="k-side-foot">{side_foot.map(|f| f.run())}</div>
            </aside>
            <div class="k-body">
                <header class="k-appbar">
                    {move || match back.get() {
                        Some(href) => view! {
                            <a class="k-icon-btn k-back" href=href aria-label=back_label.clone() inner_html=icon("arrow-left")></a>
                        }.into_any(),
                        None => view! { <a class="k-appbar-home" href=home_bar.clone() aria-label=brand_for_aria()>
                            {logo_bar.clone().map(|src| view! { <img class="k-logo" src=src alt="" width="28" height="28"/> })}
                        </a> }.into_any(),
                    }}
                    <h1 class="k-appbar-title">{title_text}</h1>
                    <div class="k-appbar-actions">{actions.map(|f| f.run())}</div>
                </header>
                <main class="k-main" id="k-main">{children()}</main>
            </div>
            <nav class="k-tabbar" aria-label=brand>
                {tab_links}
                {more.then(|| view! {
                    <button type="button" class="k-tab" class:active=more_active aria-haspopup="dialog" on:click=move |_| more_open.set(true)>
                        <span class="k-ico" aria-hidden="true" inner_html=icon("ellipsis")></span>
                        <span class="k-nav-label">{more_label.clone()}</span>
                    </button>
                })}
            </nav>
            <Sheet open=more_open title=more_title>
                <nav class="k-sheet-nav">{more_links()}</nav>
            </Sheet>
        </div>
    }
}

/// The app bar's home link has no text of its own: screen readers say "Home".
fn brand_for_aria() -> &'static str {
    "Home"
}

/// One page: its content fades and rises in when it is drawn (under 200 ms, none with reduced motion). Return a
/// new `Page` for each route, so every page change animates.
#[component]
pub fn Page(
    /// The page's content.
    children: Children,
) -> impl IntoView {
    view! { <div class="k-page">{children()}</div> }
}

/// A page's heading: its title, a line under it and its actions (which wrap under the title on phones).
#[component]
pub fn PageHead(
    /// The title (the page's `<h1>` from 768 px; phones show it in the app bar too).
    #[prop(into)]
    title: String,
    /// A line under the title.
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// Buttons.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="k-pagehead">
            <div class="k-pagehead-text">
                <h1>{title}</h1>
                {subtitle.map(|s| view! { <p class="k-sub">{s}</p> })}
            </div>
            {children.map(|c| view! { <div class="k-actions">{c()}</div> })}
        </div>
    }
}

/// A part of a page with a heading and an optional "See all" link.
#[component]
pub fn Section(
    /// The heading.
    #[prop(into)]
    title: String,
    /// A line under the heading.
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// Where "See all" goes; no link when not given.
    #[prop(optional, into)]
    see_all: Option<String>,
    /// The link's words.
    #[prop(optional, into, default = "See all".to_string())]
    see_all_label: String,
    /// The content.
    children: Children,
) -> impl IntoView {
    view! {
        <section class="k-section">
            <div class="k-section-head">
                <div class="k-section-text">
                    <h2>{title}</h2>
                    {subtitle.map(|s| view! { <p class="k-sub">{s}</p> })}
                </div>
                {see_all.map(|h| view! { <a class="k-link" href=h>{see_all_label}<span class="k-ico" aria-hidden="true" inner_html=icon("chevron-right")></span></a> })}
            </div>
            {children()}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_is_active_on_itself_and_under_it() {
        let lib = NavItem::new("#/library", "Library", "library");
        assert!(lib.is_active("#/library"));
        assert!(lib.is_active("#/library/3"));
        assert!(lib.is_active("#/library?sort=new"));
        assert!(!lib.is_active("#/libraryx"));
        assert!(!lib.is_active("#/"));
        let home = NavItem::new("#/", "Home", "home");
        assert!(home.is_active("#/") && home.is_active("") && home.is_active("#"));
        assert!(!home.is_active("#/library"));
    }

    #[test]
    fn more_than_five_sections_put_the_rest_under_more() {
        assert_eq!(tab_split(4, 5), (4, false));
        assert_eq!(tab_split(5, 5), (5, false));
        assert_eq!(tab_split(6, 5), (4, true));
        assert_eq!(tab_split(9, 4), (3, true));
        assert_eq!(tab_split(3, 9), (3, false));
    }

    #[test]
    fn the_shell_draws_the_sidebar_the_app_bar_and_the_tab_bar() {
        let nav: Vec<NavItem> = (1..=6).map(|n| NavItem::new(format!("#/s{n}"), format!("Section {n}"), "star")).collect();
        let html = view! {
            <AppShell brand="App" nav=nav active=Signal::derive(|| "#/s2".to_string())>
                <Page><p>"Hi"</p></Page>
            </AppShell>
        }
        .to_html();
        for want in ["k-side", "k-appbar", "k-tabbar", "k-page", "aria-current=\"page\"", ">More<", "Section 6"] {
            assert!(html.contains(want), "{want} in {html}");
        }
        // four tabs and More in the tab bar; all six in the sidebar
        let tabbar = &html[html.find("k-tabbar").unwrap_or(0)..];
        let tabs = tabbar.matches("\"k-tab\"").count() + tabbar.matches("\"k-tab ").count();
        assert_eq!(tabs, 5, "{tabbar}");
        assert_eq!(html.matches("k-side-link").count(), 6);
    }
}
