//! The command palette: one box that finds an app's actions and places by typing (Ctrl+K or ⌘K).
//!
//! The app hands in its items ([`Item`]): a group ("Actions", "Go to", or one of its own), a label, an optional
//! hint and icon. Items an app loads when the palette opens (its records, its sites…) are *dynamic*
//! ([`Item::dynamic`]). [`results`] decides what shows and in which order:
//!
//! - **No query:** every item that is not dynamic, in the app's order, then the first [`DYNAMIC_SHOWN`] of each
//!   dynamic group (groups in the order they first appear).
//! - **A query:** the items whose label holds it first, then those where only the hint holds it, each in the
//!   app's order; the query is trimmed and matched case-insensitively as a plain substring.
//!
//! The selection moves with Up and Down ([`step`]), stays on the list and goes back to the first result when the
//! query changes; Enter or a click runs the selected item. [`is_toggle`] says whether a key press opens or closes
//! the palette.

/// The group for things the app does ("New note", "Sign out").
pub const ACTIONS: &str = "Actions";
/// The group for the app's pages ("Settings", "Diagnostics").
pub const GO_TO: &str = "Go to";
/// The palette's name for screen readers.
pub const TITLE: &str = "Command palette";
/// The search box's placeholder.
pub const PLACEHOLDER: &str = "Type a command or search…";
/// What the list says when nothing matches.
pub const EMPTY: &str = "Nothing matches";
/// How many items of each dynamic group show while the query is empty.
pub const DYNAMIC_SHOWN: usize = 6;

/// The palette's words. [`PaletteWords::DEFAULT`] holds the kit's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteWords {
    /// The dialog's name for screen readers ([`TITLE`]).
    pub title: &'static str,
    /// The search box's placeholder ([`PLACEHOLDER`]).
    pub placeholder: &'static str,
    /// The line shown when nothing matches ([`EMPTY`]).
    pub empty: &'static str,
}

impl PaletteWords {
    /// The kit's words.
    pub const DEFAULT: PaletteWords = PaletteWords { title: TITLE, placeholder: PLACEHOLDER, empty: EMPTY };
}

impl Default for PaletteWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One entry of the palette, as it is found and drawn (what it does is the client's).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    /// The group it is listed under ([`ACTIONS`], [`GO_TO`] or the app's own).
    pub group: String,
    /// What it says.
    pub label: String,
    /// A second, quieter line (a path, a shortcut, where it goes); "" for none.
    pub hint: String,
    /// The kit icon drawn before it (`"settings"`, `"bug"`…); "" for none.
    pub icon: String,
    /// Loaded when the palette opens: only the first [`DYNAMIC_SHOWN`] of its group show with no query.
    pub dynamic: bool,
}

impl Item {
    /// An item labelled `label` in `group`, with no hint and no icon.
    pub fn new(group: impl Into<String>, label: impl Into<String>) -> Self {
        Item { group: group.into(), label: label.into(), ..Item::default() }
    }

    /// With this hint.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    /// With this kit icon.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Dynamic (loaded when the palette opens) or not.
    pub fn dynamic(mut self, dynamic: bool) -> Self {
        self.dynamic = dynamic;
        self
    }
}

/// The items that show for `query`, as indexes into `items`, in the order they are listed (see the module).
pub fn results(items: &[Item], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        let mut out: Vec<usize> = (0..items.len()).filter(|&i| !items[i].dynamic).collect();
        let mut groups: Vec<&str> = Vec::new();
        for it in items.iter().filter(|it| it.dynamic) {
            if !groups.contains(&it.group.as_str()) {
                groups.push(&it.group);
            }
        }
        for g in groups {
            out.extend((0..items.len()).filter(|&i| items[i].dynamic && items[i].group == g).take(DYNAMIC_SHOWN));
        }
        return out;
    }
    let holds = |s: &str| s.to_lowercase().contains(&q);
    let by_label: Vec<usize> = (0..items.len()).filter(|&i| holds(&items[i].label)).collect();
    let by_hint = (0..items.len()).filter(|&i| !holds(&items[i].label) && holds(&items[i].hint));
    by_label.into_iter().chain(by_hint).collect()
}

/// A move of the selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Up: the result before (stays on the first).
    Up,
    /// Down: the result after (stays on the last).
    Down,
}

/// The selection after `m`, from `sel` in a list of `len` results (0 for an empty list).
pub fn step(sel: usize, len: usize, m: Move) -> usize {
    if len == 0 {
        return 0;
    }
    match m {
        Move::Up => sel.min(len - 1).saturating_sub(1),
        Move::Down => (sel + 1).min(len - 1),
    }
}

/// Whether a key press opens or closes the palette: K with Ctrl (Windows, Linux) or ⌘ (macOS).
pub fn is_toggle(key: &str, ctrl: bool, meta: bool) -> bool {
    (ctrl || meta) && key.eq_ignore_ascii_case("k")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(items: &[Item], q: &str) -> Vec<String> {
        results(items, q).into_iter().map(|i| items[i].label.clone()).collect()
    }

    fn sample() -> Vec<Item> {
        let mut v = vec![
            Item::new(ACTIONS, "New note").icon("plus"),
            Item::new(ACTIONS, "Report a bug").icon("bug"),
            Item::new(GO_TO, "Settings").hint("/settings").icon("settings"),
            Item::new(GO_TO, "Diagnostics").hint("/diagnostics"),
        ];
        for n in 1..=8 {
            v.push(Item::new("Sites", format!("Site {n}")).hint(format!("site{n}.example")).dynamic(true));
        }
        v.push(Item::new("Folders", "Inbox").hint("notes/inbox").dynamic(true));
        v
    }

    #[test]
    fn no_query_shows_every_fixed_item_then_six_of_each_dynamic_group() {
        let items = sample();
        let want: Vec<&str> = vec![
            "New note",
            "Report a bug",
            "Settings",
            "Diagnostics",
            "Site 1",
            "Site 2",
            "Site 3",
            "Site 4",
            "Site 5",
            "Site 6",
            "Inbox",
        ];
        assert_eq!(labels(&items, ""), want);
        // spaces only is no query
        assert_eq!(labels(&items, "   "), want);
        assert!(results(&[], "").is_empty());
    }

    #[test]
    fn dynamic_groups_keep_the_order_they_first_appear_in() {
        let items = vec![
            Item::new("B", "b1").dynamic(true),
            Item::new("A", "a1").dynamic(true),
            Item::new(ACTIONS, "fixed"),
            Item::new("B", "b2").dynamic(true),
        ];
        assert_eq!(labels(&items, ""), ["fixed", "b1", "b2", "a1"]);
    }

    #[test]
    fn a_query_finds_labels_first_then_hints_case_insensitively() {
        let items = sample();
        // "set" is in "Settings"' label and its hint: listed once, with the label matches
        assert_eq!(labels(&items, "SET"), ["Settings"]);
        // label matches (every site), then hint-only matches (none): all eight sites, not six
        assert_eq!(labels(&items, "site").len(), 8);
        // "notes" is only in Inbox's hint; "note" is in New note's label too, which comes first
        assert_eq!(labels(&items, "notes"), ["Inbox"]);
        assert_eq!(labels(&items, "note"), ["New note", "Inbox"]);
        // a hint-only match after a label match from a later group
        let items = vec![Item::new(GO_TO, "Home").hint("diag"), Item::new(ACTIONS, "Diagnose")];
        assert_eq!(labels(&items, "diag"), ["Diagnose", "Home"]);
        // trimmed, and nothing found is an empty list
        assert_eq!(labels(&sample(), "  report "), ["Report a bug"]);
        assert!(labels(&sample(), "zzz").is_empty());
    }

    #[test]
    fn a_query_matches_non_ascii_text_without_regard_to_case() {
        let items = vec![Item::new(GO_TO, "Übersicht"), Item::new(GO_TO, "Einstellungen").hint("ÄNDERN")];
        assert_eq!(labels(&items, "über"), ["Übersicht"]);
        assert_eq!(labels(&items, "ändern"), ["Einstellungen"]);
    }

    #[test]
    fn the_selection_stays_on_the_list() {
        assert_eq!(step(0, 3, Move::Down), 1);
        assert_eq!(step(2, 3, Move::Down), 2);
        assert_eq!(step(2, 3, Move::Up), 1);
        assert_eq!(step(0, 3, Move::Up), 0);
        // a selection past a list that shrank comes back onto it
        assert_eq!(step(9, 3, Move::Up), 1);
        assert_eq!(step(9, 3, Move::Down), 2);
        assert_eq!(step(0, 0, Move::Down), 0);
        assert_eq!(step(0, 0, Move::Up), 0);
    }

    #[test]
    fn ctrl_or_cmd_k_toggles() {
        assert!(is_toggle("k", true, false));
        assert!(is_toggle("K", false, true));
        assert!(!is_toggle("k", false, false));
        assert!(!is_toggle("j", true, false));
    }

    #[test]
    fn the_words_have_kit_defaults() {
        let w = PaletteWords::default();
        assert_eq!((w.title, w.placeholder, w.empty), (TITLE, PLACEHOLDER, EMPTY));
        let it = Item::new(GO_TO, "Settings").hint("/settings").icon("settings");
        assert_eq!((it.group.as_str(), it.hint.as_str(), it.icon.as_str(), it.dynamic), (GO_TO, "/settings", "settings", false));
    }
}
