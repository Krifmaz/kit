//! Suite: apps that link to each other and share one sidebar, one sign-in and one search
//! (`contracts/suite.md`).
//!
//! Every app serves a [`Manifest`] (its id, name, icon, sections, base URL, deep-link patterns and an optional
//! search endpoint). An app linked to others shows their sections next to its own in one sidebar; each person
//! hides, shows and reorders the entries ([`SidebarPrefs`]), and an entry opens the other app at that section.
//! The rules here are the parts every client and the server do the same way: building the sidebar from the
//! apps' manifests and the person's choices ([`sidebar`]), moving and hiding entries, filling deep-link
//! patterns ([`fill`]) and joining URLs ([`join`]). Nothing names an app or a section: they all come from the
//! manifests.

use serde::{Deserialize, Serialize};

/// The version of the manifest format this kit speaks ([`Manifest::kit_suite`]).
pub const MANIFEST_VERSION: u32 = 1;

/// The key [`SidebarPrefs`] are kept under in an account's preferences.
pub const PREFS_KEY: &str = "suite_sidebar";

/// One section of an app: an entry in the shared sidebar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Section {
    /// The section's id, unique in its app (`a-z`, `0-9`, `-`).
    pub id: String,
    /// The entry's words.
    pub title: String,
    /// The entry's icon: SVG path data in a 24×24 box, drawn as a 2 px round stroke like the kit's icons.
    pub icon: String,
    /// Where the section is in its app: a path from the app's base URL (`/things`).
    pub path: String,
}

/// The single sign-on an app offers when it is an account source (an OIDC provider).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SsoInfo {
    /// The OIDC issuer: the app's base URL, where `/.well-known/openid-configuration` is.
    pub issuer: String,
}

/// What an app says about itself at `GET /api/kit/manifest`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    /// The manifest format ([`MANIFEST_VERSION`]).
    pub kit_suite: u32,
    /// The app's id (`a-z`, `0-9`, `-`): the first half of every sidebar key.
    pub id: String,
    /// The app's name, as people see it.
    pub name: String,
    /// The app's icon: SVG path data in a 24×24 box, like a section's.
    pub icon: String,
    /// The app's version.
    pub version: String,
    /// Where the app is: `https://host[:port][/prefix]`, without a trailing `/`.
    pub base_url: String,
    /// The app's sections, in its own order.
    pub sections: Vec<Section>,
    /// Deep-link patterns by name: paths from the base URL with `{name}` placeholders (`/things/{id}`).
    pub deep_links: std::collections::BTreeMap<String, String>,
    /// The app's search endpoint (a path from the base URL), when it has one.
    pub search: Option<String>,
    /// The sign-on it offers other apps, when it is an account source.
    pub sso: Option<SsoInfo>,
}

impl Manifest {
    /// The URL of section `id`, if the app has it.
    pub fn section_url(&self, id: &str) -> Option<String> {
        self.sections.iter().find(|s| s.id == id).map(|s| join(&self.base_url, &s.path))
    }

    /// The URL of deep link `name` with its placeholders filled, if the app has that pattern.
    pub fn deep_link(&self, name: &str, vars: &[(&str, &str)]) -> Option<String> {
        self.deep_links.get(name).map(|p| join(&self.base_url, &fill(p, vars)))
    }

    /// What is wrong with the manifest, in one sentence each (none when it can be linked to).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.kit_suite == 0 || self.kit_suite > MANIFEST_VERSION {
            out.push(format!("The manifest format {} is not one this app reads (it reads 1 to {MANIFEST_VERSION}).", self.kit_suite));
        }
        if !valid_id(&self.id) {
            out.push("The app's id must be letters a–z, digits and dashes.".into());
        }
        if self.name.trim().is_empty() {
            out.push("The app has no name.".into());
        }
        if !(self.base_url.starts_with("http://") || self.base_url.starts_with("https://")) {
            out.push("The app's address must start with http:// or https://.".into());
        }
        let mut seen = std::collections::HashSet::new();
        for s in &self.sections {
            if !valid_id(&s.id) || !seen.insert(s.id.as_str()) {
                out.push(format!("The section id \"{}\" is not a unique id of letters a–z, digits and dashes.", s.id));
            }
        }
        out
    }
}

/// Whether `id` is an app or section id: 1 to 64 of `a-z`, `0-9` and `-`.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `base` and `path` as one URL: one `/` between them; a `path` that is already a full URL is kept as is.
pub fn join(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let base = base.trim_end_matches('/');
    if path.is_empty() {
        return base.to_string();
    }
    format!("{base}/{}", path.trim_start_matches('/'))
}

/// A base URL as the suite keeps it: trimmed, without a trailing `/`; None when it is not `http(s)://host…`.
pub fn normalize_base(url: &str) -> Option<String> {
    let u = url.trim().trim_end_matches('/');
    let rest = u.strip_prefix("https://").or_else(|| u.strip_prefix("http://"))?;
    let host = rest.split('/').next().unwrap_or("");
    if host.is_empty() || u.contains(['?', '#', ' ']) {
        return None;
    }
    Some(u.to_string())
}

/// `pattern` with each `{name}` replaced by its value, percent-encoded; unknown placeholders stay as they are.
pub fn fill(pattern: &str, vars: &[(&str, &str)]) -> String {
    let mut out = pattern.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), &encode(v));
    }
    out
}

/// `s` percent-encoded for a path segment or a query value (everything but `A-Z a-z 0-9 - . _ ~`).
pub fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A sidebar entry's key: `<app id>/<section id>`.
pub fn key(app: &str, section: &str) -> String {
    format!("{app}/{section}")
}

/// What a person chose for the shared sidebar: the order of the entries they moved, and the ones they hid.
/// Kept in their preferences under [`PREFS_KEY`]. Entries not in `order` (an app linked later) come after
/// the ordered ones, in the apps' own order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarPrefs {
    /// Entry keys ([`key`]) in the order the person put them.
    pub order: Vec<String>,
    /// Entry keys the person hid.
    pub hidden: Vec<String>,
}

/// One entry of the shared sidebar.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SidebarItem {
    /// `<app id>/<section id>`.
    pub key: String,
    /// The app's id.
    pub app: String,
    /// The app's name.
    pub app_name: String,
    /// The section's id.
    pub section: String,
    /// The entry's words.
    pub title: String,
    /// The section's icon (SVG path data), the app's when the section has none.
    pub icon: String,
    /// Where it opens: the section's full URL.
    pub url: String,
    /// The section's path in its app.
    pub path: String,
    /// The person hid it (it shows only while they edit the sidebar).
    pub hidden: bool,
    /// It belongs to the app showing the sidebar (opened in place, not as another app).
    pub local: bool,
}

/// The shared sidebar: every section of `apps` (the showing app first, then the linked apps), in the order
/// `prefs` gives, each marked hidden when the person hid it. `local` is the showing app's id. An app id that
/// shows up twice keeps only its first manifest.
pub fn sidebar(apps: &[Manifest], local: &str, prefs: &SidebarPrefs) -> Vec<SidebarItem> {
    let mut seen_apps = std::collections::HashSet::new();
    let mut all: Vec<SidebarItem> = Vec::new();
    for m in apps {
        if !seen_apps.insert(m.id.as_str()) {
            continue;
        }
        for s in &m.sections {
            all.push(SidebarItem {
                key: key(&m.id, &s.id),
                app: m.id.clone(),
                app_name: m.name.clone(),
                section: s.id.clone(),
                title: s.title.clone(),
                icon: if s.icon.is_empty() { m.icon.clone() } else { s.icon.clone() },
                url: join(&m.base_url, &s.path),
                path: s.path.clone(),
                hidden: prefs.hidden.contains(&key(&m.id, &s.id)),
                local: m.id == local,
            });
        }
    }
    let rank = |k: &str| prefs.order.iter().position(|o| o == k).unwrap_or(usize::MAX);
    // a stable sort keeps the apps' own order for everything the person never moved
    all.sort_by_key(|i| rank(&i.key));
    all
}

/// The entries that show while the person is not editing the sidebar.
pub fn visible(items: &[SidebarItem]) -> Vec<&SidebarItem> {
    items.iter().filter(|i| !i.hidden).collect()
}

/// The preferences that put the entries in exactly the order of `items` and hide the hidden ones.
pub fn prefs_of(items: &[SidebarItem]) -> SidebarPrefs {
    SidebarPrefs {
        order: items.iter().map(|i| i.key.clone()).collect(),
        hidden: items.iter().filter(|i| i.hidden).map(|i| i.key.clone()).collect(),
    }
}

/// Move entry `key` one place up (`by = -1`) or down (`by = 1`) among `items`; nothing at either end.
pub fn move_item(items: &mut [SidebarItem], key: &str, by: i32) {
    let Some(i) = items.iter().position(|x| x.key == key) else { return };
    let j = i as i64 + i64::from(by);
    if j < 0 || j >= items.len() as i64 {
        return;
    }
    items.swap(i, j as usize);
}

/// Hide entry `key` or show it again.
pub fn toggle_hidden(items: &mut [SidebarItem], key: &str) {
    if let Some(i) = items.iter_mut().find(|x| x.key == key) {
        i.hidden = !i.hidden;
    }
}

/// One result of a search, as an app's search endpoint answers it (`{"results": [<hit>…]}`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchHit {
    /// What was found.
    pub title: String,
    /// A second line (where it is, what kind it is).
    pub subtitle: Option<String>,
    /// Where it opens: a path from the app's base URL, or a full URL.
    pub url: String,
    /// The section it belongs to, when the app says.
    pub section: Option<String>,
    /// An icon (SVG path data), when the hit has its own.
    pub icon: Option<String>,
}

/// One app's part of a suite search.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchGroup {
    /// The app's id.
    pub app: String,
    /// The app's name.
    pub app_name: String,
    /// The app's icon.
    pub icon: String,
    /// It is the app that was asked (its hits open in place).
    pub local: bool,
    /// What it found, each `url` a full URL.
    pub results: Vec<SearchHit>,
    /// Why it could not be searched, in one sentence; no results then.
    pub error: Option<String>,
}

impl SearchGroup {
    /// `hits` of the app `m` as a group, each hit's URL made full.
    pub fn of(m: &Manifest, local: bool, hits: Vec<SearchHit>) -> SearchGroup {
        let results = hits.into_iter().map(|h| SearchHit { url: join(&m.base_url, &h.url), ..h }).collect();
        SearchGroup { app: m.id.clone(), app_name: m.name.clone(), icon: m.icon.clone(), local, results, error: None }
    }
}

/// The words of the suite's screens. Templates fill `{app}` and `{query}` with [`crate::app::fill`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuiteWords {
    /// The sidebar's button that starts editing it.
    pub edit: &'static str,
    /// The button that ends editing.
    pub done: &'static str,
    /// Hide an entry.
    pub hide: &'static str,
    /// Show a hidden entry.
    pub show: &'static str,
    /// Move an entry up.
    pub move_up: &'static str,
    /// Move an entry down.
    pub move_down: &'static str,
    /// Put the sidebar back in the apps' own order with nothing hidden.
    pub reset: &'static str,
    /// The search field's placeholder.
    pub search_placeholder: &'static str,
    /// When nothing was found.
    pub no_results: &'static str,
    /// The heading of cross-links into another app.
    pub related_in: &'static str,
    /// The sign-in button of an account source.
    pub sign_in_with: &'static str,
    /// The linked apps page's title.
    pub linked_apps: &'static str,
    /// Add a link.
    pub add_link: &'static str,
    /// The link form's address field.
    pub address: &'static str,
    /// The link form's key field.
    pub api_key: &'static str,
    /// The account source switch.
    pub account_source: &'static str,
    /// Its explanation.
    pub account_source_help: &'static str,
    /// Remove a link.
    pub remove_link: &'static str,
    /// The confirm dialog's sentence for removing a link.
    pub remove_link_confirm: &'static str,
    /// The keys section's title.
    pub keys: &'static str,
    /// The keys section's explanation.
    pub keys_help: &'static str,
    /// Make a key.
    pub new_key: &'static str,
    /// The key name field.
    pub key_name: &'static str,
    /// The address an app signs people in from with a key (its base URL).
    pub key_redirect: &'static str,
    /// Said once next to a new key.
    pub key_once: &'static str,
    /// Revoke a key.
    pub revoke_key: &'static str,
    /// The confirm dialog's sentence for revoking a key.
    pub revoke_key_confirm: &'static str,
    /// Check a link again.
    pub refresh: &'static str,
    /// No links yet.
    pub no_links: &'static str,
}

impl SuiteWords {
    /// The kit's words.
    pub const DEFAULT: SuiteWords = SuiteWords {
        edit: "Edit sidebar",
        done: "Done",
        hide: "Hide",
        show: "Show",
        move_up: "Move up",
        move_down: "Move down",
        reset: "Reset order",
        search_placeholder: "Search every app",
        no_results: "Nothing found for \u{201c}{query}\u{201d}.",
        related_in: "Related in {app}",
        sign_in_with: "Sign in with {app}",
        linked_apps: "Linked apps",
        add_link: "Link an app",
        address: "Address",
        api_key: "Key",
        account_source: "Account source",
        account_source_help: "People sign in here with their account from this app.",
        remove_link: "Remove link",
        remove_link_confirm: "Remove the link to {app}? Its sections leave the sidebar and sign-in through it stops.",
        keys: "Keys for other apps",
        keys_help: "Another app links to this one with a key made here.",
        new_key: "Make a key",
        key_name: "Name",
        key_redirect: "Other app's address (for sign-in)",
        key_once: "Copy the key now: it is shown only once.",
        revoke_key: "Revoke",
        revoke_key_confirm: "Revoke the key \u{201c}{app}\u{201d}? The app using it can no longer link here.",
        refresh: "Check again",
        no_links: "No apps are linked yet.",
    };
}

impl Default for SuiteWords {
    fn default() -> Self {
        SuiteWords::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, base: &str, sections: &[&str]) -> Manifest {
        Manifest {
            kit_suite: 1,
            id: id.into(),
            name: id.to_uppercase(),
            icon: "M1 1h1".into(),
            base_url: base.into(),
            sections: sections
                .iter()
                .map(|s| Section { id: (*s).into(), title: s.to_uppercase(), icon: String::new(), path: format!("/{s}") })
                .collect(),
            ..Manifest::default()
        }
    }

    #[test]
    fn urls_join_and_fill() {
        assert_eq!(join("https://a.test/", "/x"), "https://a.test/x");
        assert_eq!(join("https://a.test/p", "x/y"), "https://a.test/p/x/y");
        assert_eq!(join("https://a.test", ""), "https://a.test");
        assert_eq!(join("https://a.test", "https://b.test/z"), "https://b.test/z");
        assert_eq!(fill("/things/{id}?q={q}&k={k}", &[("id", "a b/c"), ("q", "é")]), "/things/a%20b%2Fc?q=%C3%A9&k={k}");
        assert_eq!(normalize_base(" https://a.test:8080/p/ ").as_deref(), Some("https://a.test:8080/p"));
        assert_eq!(normalize_base("ftp://a.test"), None);
        assert_eq!(normalize_base("https://"), None);
        assert_eq!(normalize_base("https://a.test/?x"), None);
    }

    #[test]
    fn manifests_check_themselves() {
        let mut m = app("one", "https://a.test", &["x", "y"]);
        m.deep_links.insert("item".into(), "/items/{id}".into());
        assert!(m.problems().is_empty());
        assert_eq!(m.section_url("y").as_deref(), Some("https://a.test/y"));
        assert_eq!(m.section_url("z"), None);
        assert_eq!(m.deep_link("item", &[("id", "7")]).as_deref(), Some("https://a.test/items/7"));
        m.sections.push(Section { id: "x".into(), ..Section::default() });
        m.base_url = "a.test".into();
        m.id = "One".into();
        m.kit_suite = 9;
        assert_eq!(m.problems().len(), 4);
        assert!(valid_id("a-1") && !valid_id("") && !valid_id("A") && !valid_id(&"a".repeat(65)));
    }

    #[test]
    fn the_sidebar_follows_the_persons_order() {
        let apps =
            [app("one", "https://a.test", &["x", "y"]), app("two", "https://b.test/", &["z"]), app("one", "https://dup.test", &["q"])];
        let items = sidebar(&apps, "one", &SidebarPrefs::default());
        assert_eq!(items.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), ["one/x", "one/y", "two/z"]);
        assert!(items[0].local && !items[2].local);
        assert_eq!(items[2].url, "https://b.test/z");
        assert_eq!(items[2].icon, "M1 1h1");
        assert_eq!(items[2].app_name, "TWO");

        let prefs = SidebarPrefs { order: vec!["two/z".into(), "one/y".into(), "gone/a".into()], hidden: vec!["one/y".into()] };
        let mut items = sidebar(&apps, "one", &prefs);
        assert_eq!(items.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(), ["two/z", "one/y", "one/x"]);
        assert_eq!(visible(&items).len(), 2);
        move_item(&mut items, "one/x", -1);
        move_item(&mut items, "two/z", -1);
        move_item(&mut items, "nope", 1);
        toggle_hidden(&mut items, "one/y");
        toggle_hidden(&mut items, "two/z");
        let p = prefs_of(&items);
        assert_eq!(p.order, ["two/z", "one/x", "one/y"]);
        assert_eq!(p.hidden, ["two/z"]);
        assert_eq!(sidebar(&apps, "one", &p).iter().map(|i| i.key.clone()).collect::<Vec<_>>(), p.order);
    }

    #[test]
    fn search_hits_get_full_urls() {
        let m = app("two", "https://b.test", &[]);
        let g = SearchGroup::of(&m, false, vec![SearchHit { title: "T".into(), url: "/items/1".into(), ..SearchHit::default() }]);
        assert_eq!(g.results[0].url, "https://b.test/items/1");
        assert_eq!((g.app.as_str(), g.app_name.as_str(), g.local), ("two", "TWO", false));
        let parsed: Manifest = serde_json::from_str(r#"{"kit_suite":1,"id":"a","name":"A","base_url":"http://x","extra":1}"#).unwrap();
        assert_eq!(parsed.sections.len(), 0);
        assert_eq!(parsed.search, None);
    }
}
