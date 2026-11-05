//! The kit's icon set (the same names and paths as kit-leptos), as SVG handles tinted to the theme,
//! an app's own icons on top of it ([`AppIcon`], drawn by every widget that takes an [`Icon`]), and
//! the app's logo in the accent gradient ([`Logo`]).

use super::El;
use crate::theme;
use iced::widget::svg::{self, Handle};
use iced::Color;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// An icon of the web's set; [`Icon::name`] is its web name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)] // each variant is the web icon of the same name
pub enum Icon {
    Home,
    Search,
    Inbox,
    Download,
    Activity,
    Users,
    Ticket,
    Settings,
    Play,
    Pause,
    Plus,
    Check,
    X,
    Menu,
    Logout,
    Tag,
    Edit,
    Trash,
    Refresh,
    Link,
    User,
    Sun,
    Moon,
    Copy,
    External,
    Shield,
    Heart,
    Spark,
    Chevron,
    More,
    List,
    Devices,
    Bug,
    /// A video (save a video from a link).
    Video,
    /// One of the app's own icons (not in [`Icon::ALL`]).
    App(AppIcon),
}

/// An icon an app adds to the kit's set: its own SVG shapes in the kit's 24×24 box, drawn like the
/// kit's icons (a 2 px round stroke, or filled), so it fits beside them. Keep them as consts:
///
/// ```
/// use kit_iced::widgets::{AppIcon, Icon};
/// const STAR: Icon = Icon::App(AppIcon::stroke("star", r#"<path d="m12 3 3 6 6 1-4.5 4 1 6-5.5-3-5.5 3 1-6L3 10l6-1z"/>"#));
/// assert_eq!(STAR.name(), "star");
/// assert!(STAR.svg().contains("stroke-width"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AppIcon {
    /// The icon's name (for logs and tests; the app keeps names unique).
    pub name: &'static str,
    /// The SVG elements inside the 24×24 view box (`<path d="…"/>`, `<circle …/>`).
    pub body: &'static str,
    /// Filled with the colour (transport shapes) instead of stroked.
    pub filled: bool,
}

impl AppIcon {
    /// An icon drawn with the kit's 2 px round stroke.
    pub const fn stroke(name: &'static str, body: &'static str) -> AppIcon {
        AppIcon { name, body, filled: false }
    }

    /// An icon filled with the colour (like the kit's Play and Pause).
    pub const fn filled(name: &'static str, body: &'static str) -> AppIcon {
        AppIcon { name, body, filled: true }
    }
}

impl Icon {
    /// Every icon of the kit's own set (an app's [`Icon::App`] icons are its own).
    pub const ALL: [Icon; 34] = [
        Icon::Home,
        Icon::Search,
        Icon::Inbox,
        Icon::Download,
        Icon::Activity,
        Icon::Users,
        Icon::Ticket,
        Icon::Settings,
        Icon::Play,
        Icon::Pause,
        Icon::Plus,
        Icon::Check,
        Icon::X,
        Icon::Menu,
        Icon::Logout,
        Icon::Tag,
        Icon::Edit,
        Icon::Trash,
        Icon::Refresh,
        Icon::Link,
        Icon::User,
        Icon::Sun,
        Icon::Moon,
        Icon::Copy,
        Icon::External,
        Icon::Shield,
        Icon::Heart,
        Icon::Spark,
        Icon::Chevron,
        Icon::More,
        Icon::List,
        Icon::Devices,
        Icon::Bug,
        Icon::Video,
    ];

    /// The web's name (`icon("home")`).
    pub fn name(self) -> &'static str {
        match self {
            Icon::Home => "home",
            Icon::Search => "search",
            Icon::Inbox => "inbox",
            Icon::Download => "download",
            Icon::Video => "video",
            Icon::Activity => "activity",
            Icon::Users => "users",
            Icon::Ticket => "ticket",
            Icon::Settings => "settings",
            Icon::Play => "play",
            Icon::Pause => "pause",
            Icon::Plus => "plus",
            Icon::Check => "check",
            Icon::X => "x",
            Icon::Menu => "menu",
            Icon::Logout => "logout",
            Icon::Tag => "tag",
            Icon::Edit => "edit",
            Icon::Trash => "trash",
            Icon::Refresh => "refresh",
            Icon::Link => "link",
            Icon::User => "user",
            Icon::Sun => "sun",
            Icon::Moon => "moon",
            Icon::Copy => "copy",
            Icon::External => "external",
            Icon::Shield => "shield",
            Icon::Heart => "heart",
            Icon::Spark => "spark",
            Icon::Chevron => "chevron",
            Icon::More => "more",
            Icon::List => "list",
            Icon::Devices => "devices",
            Icon::Bug => "bug",
            Icon::App(a) => a.name,
        }
    }

    /// The kit icon of a web name, for tables that carry icon names (the bar actions); an app's own
    /// icons are not looked up by name.
    pub fn by_name(name: &str) -> Option<Icon> {
        Icon::ALL.into_iter().find(|i| i.name() == name)
    }

    /// Filled shapes (the transport) are drawn with `fill`, the rest with a 2px stroke.
    fn filled(self) -> bool {
        match self {
            Icon::App(a) => a.filled,
            _ => matches!(self, Icon::Play | Icon::Pause),
        }
    }

    fn body(self) -> &'static str {
        match self {
            Icon::Home => r##"<path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>"##,
            Icon::Search => r##"<circle cx="11" cy="11" r="7"/><path d="m21 21-4.3-4.3"/>"##,
            Icon::Inbox => {
                r##"<path d="M22 12h-6l-2 3h-4l-2-3H2"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>"##
            }
            Icon::Download => r##"<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5M12 15V3"/>"##,
            Icon::Video => r##"<rect x="2" y="6" width="14" height="12" rx="2"/><path d="m16 10 6-4v12l-6-4z"/>"##,
            Icon::Activity => r##"<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>"##,
            Icon::Users => {
                r##"<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/>"##
            }
            Icon::Ticket => {
                r##"<path d="M2 9a3 3 0 0 0 0 6v2a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-2a3 3 0 0 0 0-6V7a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2z"/><path d="M13 5v2M13 17v2M13 11v2"/>"##
            }
            Icon::Settings => {
                r##"<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>"##
            }
            Icon::Play => r##"<path d="M7 4.5v15a1 1 0 0 0 1.5.86l12-7.5a1 1 0 0 0 0-1.72l-12-7.5A1 1 0 0 0 7 4.5z"/>"##,
            Icon::Pause => r##"<rect x="6" y="4" width="4" height="16" rx="1"/><rect x="14" y="4" width="4" height="16" rx="1"/>"##,
            Icon::Plus => r##"<path d="M12 5v14M5 12h14"/>"##,
            Icon::Check => r##"<path d="M20 6 9 17l-5-5"/>"##,
            Icon::X => r##"<path d="M18 6 6 18M6 6l12 12"/>"##,
            Icon::Menu => r##"<path d="M3 6h18M3 12h18M3 18h18"/>"##,
            Icon::Logout => r##"<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9"/>"##,
            Icon::Tag => {
                r##"<path d="M20.59 13.41 13.42 20.58a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/><circle cx="7" cy="7" r="1.5"/>"##
            }
            Icon::Edit => r##"<path d="M12 20h9M16.5 3.5a2.12 2.12 0 1 1 3 3L7 19l-4 1 1-4z"/>"##,
            Icon::Trash => r##"<path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6"/>"##,
            Icon::Refresh => r##"<path d="M21 12a9 9 0 1 1-2.64-6.36L21 8M21 3v5h-5"/>"##,
            Icon::Link => {
                r##"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>"##
            }
            Icon::User => r##"<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>"##,
            Icon::Sun => {
                r##"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>"##
            }
            Icon::Moon => r##"<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>"##,
            Icon::Copy => {
                r##"<rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>"##
            }
            Icon::External => r##"<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6M15 3h6v6M10 14 21 3"/>"##,
            Icon::Shield => r##"<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>"##,
            Icon::Heart => {
                r##"<path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/>"##
            }
            Icon::Spark => {
                r##"<path d="M12 3l1.9 5.6L19.5 10l-5.6 1.9L12 17.5l-1.9-5.6L4.5 10l5.6-1.4z"/><path d="M19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8z"/>"##
            }
            Icon::Chevron => r##"<polyline points="6 9 12 15 18 9"/>"##,
            Icon::More => r##"<circle cx="12" cy="5" r="1.6"/><circle cx="12" cy="12" r="1.6"/><circle cx="12" cy="19" r="1.6"/>"##,
            Icon::List => r##"<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>"##,
            Icon::Devices => {
                r##"<rect x="2" y="4" width="13" height="10" rx="2"/><path d="M6 18h5M8.5 14v4"/><rect x="17" y="8" width="5" height="12" rx="1.5"/><path d="M19.5 17h.01"/>"##
            }
            Icon::Bug => {
                r##"<path d="M9 7V6a3 3 0 0 1 6 0v1"/><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6z"/><path d="M12 20v-9M6 13H2M22 13h-4M3 21c0-2.1 1.7-3.9 3.8-4M20.2 17c2.1.1 3.8 1.9 3.8 4M3 5c0 2.1 1.6 3.8 3.5 4M21 5c0 2.1-1.6 3.8-3.5 4M8 2l1.9 1.9M16 2l-1.9 1.9"/>"##
            }
            Icon::App(a) => a.body,
        }
    }

    /// The SVG document.
    pub fn svg(self) -> String {
        if self.filled() {
            format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="#000">{}</svg>"##, self.body())
        } else {
            format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
                self.body()
            )
        }
    }
}

fn handles() -> &'static HashMap<Icon, Handle> {
    static H: OnceLock<HashMap<Icon, Handle>> = OnceLock::new();
    H.get_or_init(|| Icon::ALL.into_iter().map(|i| (i, Handle::from_memory(i.svg().into_bytes()))).collect())
}

/// The handle of an icon (shared, cheap to clone). An app's own icons are made once, on first use.
pub fn handle(i: Icon) -> Handle {
    if let Icon::App(a) = i {
        static APP: Mutex<Option<HashMap<AppIcon, Handle>>> = Mutex::new(None);
        let mut g = APP.lock().unwrap_or_else(|e| e.into_inner());
        return g.get_or_insert_with(HashMap::new).entry(a).or_insert_with(|| Handle::from_memory(i.svg().into_bytes())).clone();
    }
    handles()[&i].clone()
}

/// An icon of `size` px in `color`.
pub fn icon<'a, M: 'a>(i: Icon, size: f32, color: Color) -> El<'a, M> {
    svg::Svg::new(handle(i)).width(size).height(size).style(move |_, _| svg::Style { color: Some(color) }).into()
}

/// An app's logo: SVG paths in a view box, filled with the accent gradient. The app gives its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Logo {
    /// The SVG `viewBox` ("0 0 24 24").
    pub view_box: &'static str,
    /// The `d` of each path, filled with the gradient.
    pub paths: &'static [&'static str],
    /// Where the gradient runs, in view box units: `(x1, y1, x2, y2)`.
    pub gradient: (f32, f32, f32, f32),
}

impl Logo {
    /// A neutral mark (a rounded square with a spark) for apps without a logo yet, and the demo.
    pub const KIT: Logo = Logo {
        view_box: "0 0 24 24",
        paths: &["M6 2h12a4 4 0 0 1 4 4v12a4 4 0 0 1-4 4H6a4 4 0 0 1-4-4V6a4 4 0 0 1 4-4zm6 4-1.6 4.4L6 12l4.4 1.6L12 18l1.6-4.4L18 12l-4.4-1.6z"],
        gradient: (2.0, 22.0, 22.0, 2.0),
    };

    /// The logo as an SVG document in the colours `a` → `b`.
    pub fn svg(&self, a: Color, b: Color) -> String {
        let (x1, y1, x2, y2) = self.gradient;
        let mut s = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{}"><defs><linearGradient id="g" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient></defs>"#,
            self.view_box,
            theme::to_hex(a),
            theme::to_hex(b)
        );
        for d in self.paths {
            s.push_str(&format!(r##"<path d="{d}" fill="url(#g)" fill-rule="evenodd"/>"##));
        }
        s.push_str("</svg>");
        s
    }
}

/// The logo in the gradient `a` → `b`, cached per logo and pair of colours.
pub fn logo_handle(logo: &Logo, a: Color, b: Color) -> Handle {
    static CACHE: Mutex<Option<HashMap<String, Handle>>> = Mutex::new(None);
    let key = format!("{}{}{}", logo.paths.join("|"), theme::to_hex(a), theme::to_hex(b));
    let mut g = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    if let Some(h) = map.get(&key) {
        return h.clone();
    }
    let h = Handle::from_memory(logo.svg(a, b).into_bytes());
    map.insert(key, h.clone());
    h
}

/// The app's logo at `size` px in the active look's accent colours.
pub fn logo<'a, M: 'a>(logo: &Logo, size: f32) -> El<'a, M> {
    let p = theme::look();
    svg::Svg::new(logo_handle(logo, p.accent, p.accent2)).width(size).height(size).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_web_icon_has_a_handle_and_a_name() {
        for i in Icon::ALL {
            assert_eq!(Icon::by_name(i.name()), Some(i));
            assert!(i.svg().contains("<svg"));
            let _ = handle(i);
        }
        assert_eq!(Icon::by_name("devices"), Some(Icon::Devices));
        assert_eq!(Icon::by_name("nothing"), None);
    }

    #[test]
    fn an_apps_own_icons_draw_like_the_kits() {
        const RING: Icon = Icon::App(AppIcon::stroke("ring", r#"<circle cx="12" cy="12" r="9"/>"#));
        const DOT: Icon = Icon::App(AppIcon::filled("dot", r#"<circle cx="12" cy="12" r="4"/>"#));
        assert_eq!((RING.name(), DOT.name()), ("ring", "dot"));
        assert!(RING.svg().contains(r#"stroke-width="2""#) && RING.svg().contains(r#"r="9""#));
        assert!(DOT.svg().contains(r##"fill="#000""##) && !DOT.svg().contains("stroke-width"));
        assert_eq!(handle(RING), handle(RING), "made once and shared");
        assert_eq!(Icon::by_name("ring"), None, "only the kit's icons are found by name");
        assert!(!Icon::ALL.contains(&RING));
        let _ = icon::<()>(DOT, 16.0, Color::WHITE);
    }

    #[test]
    fn a_logo_is_the_apps_paths_in_the_accent_gradient() {
        let s = Logo::KIT.svg(Color::WHITE, Color::BLACK);
        assert!(s.contains(r#"viewBox="0 0 24 24""#) && s.contains("#ffffff") && s.contains("#000000"));
        assert!(s.contains(Logo::KIT.paths[0]));
        let _ = logo::<()>(&Logo::KIT, 32.0);
    }
}
