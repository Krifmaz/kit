//! What's new: release notes into blocks, blocks into lists and tables, and when the popup shows.
//!
//! From Encore's `server/crates/api/src/routes/changelog.rs` (`parse`, `plain`, `lead`, `short`, `unseen`,
//! `popup_on`, `POPUP_RELEASES`, `HIGHLIGHTS`; tests in `changelog_tests.rs`), `web-rs/src/whats_new.rs`
//! (`notes_html` grouping, `save_popup` words, the popup and Settings → What's new words) and the desktop's
//! `clients/desktop/src/whats_new.rs` / `pages/settings/whats_new.rs`.
//!
//! The parsing is here, not only in the server crate, so a server and every client read the same notes the same
//! way; the contract is `contracts/changelog.md`.

use crate::version;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// The popup shows at most this many releases (someone who skipped many updates gets the newest few and the
/// link to the full changelog). From `changelog.rs` `POPUP_RELEASES`.
pub const POPUP_RELEASES: usize = 4;
/// Highlights per release in the popup. From `changelog.rs` `HIGHLIGHTS`.
pub const HIGHLIGHTS: usize = 6;
/// The summary is cut to about this many characters. From `changelog.rs` `parse` (`short(text, 320)`).
pub const SUMMARY_CHARS: usize = 320;
/// A highlight is cut to about this many characters. From `changelog.rs` `parse` (`short(…, 110)`).
pub const HIGHLIGHT_CHARS: usize = 110;
/// Without bold leads, this many first bullets are the highlights. From `changelog.rs` `parse` (`take(4)`).
pub const PLAIN_HIGHLIGHTS: usize = 4;

/// The popup's title and Settings' card title. From `web-rs/src/whats_new.rs` `popup` / `render`.
pub const WHATS_NEW: &str = "What's new";
/// The popup's switch. From `web-rs/src/whats_new.rs` `popup`.
pub const SHOW_THIS_AFTER_UPDATES: &str = "Show this after updates";
/// The popup's button to Settings → What's new. From `web-rs/src/whats_new.rs` `popup`.
pub const FULL_CHANGELOG: &str = "Full changelog";
/// The popup's Close button. From `web-rs/src/whats_new.rs` `popup`.
pub const CLOSE: &str = "Close";
/// Settings → What's new's switch. From `web-rs/src/whats_new.rs` `render`.
pub const SHOW_WHATS_NEW_AFTER_UPDATES: &str = "Show what's new after updates";
/// Settings → What's new without notes. From `web-rs/src/whats_new.rs` `render`.
pub const NO_RELEASE_NOTES: &str = "No release notes in this build.";
/// The toast when the switch was turned on. From `web-rs/src/whats_new.rs` `save_popup`.
pub const POPUP_ON: &str = "What's new will show after updates";
/// The toast when the switch was turned off. From `web-rs/src/whats_new.rs` `save_popup`.
pub const POPUP_OFF: &str = "What's new won't show after updates – it stays in Settings";
/// Where the popup's Full changelog goes in Encore's web app. From `web-rs/src/whats_new.rs` `SETTINGS`.
pub const SETTINGS_PATH: &str = "/settings?s=whats-new";

/// The toast after the popup switch was saved. From `web-rs/src/whats_new.rs` `save_popup` and the desktop's
/// `pages/settings/whats_new.rs` `saved_words`.
pub fn saved_words(on: bool) -> &'static str {
    if on {
        POPUP_ON
    } else {
        POPUP_OFF
    }
}

/// Settings → What's new's first line for `app` at `version` ("This server runs Encore 0.36.4. After an
/// update, …"). From `web-rs/src/whats_new.rs` `render`.
pub fn settings_intro(app: &str, version: &str) -> String {
    format!(
        "This server runs {app} {version}. After an update, a short list of what changed shows once on the website and in the desktop app."
    )
}

/// One part of a release's notes, inline markdown taken out (bold, italics, code, links keep their words);
/// tagged by `k` as the server sends it.
///
/// From `changelog.rs` `Block`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "lowercase")]
pub enum Block {
    /// `### Library`.
    H {
        /// The heading's text.
        text: String,
    },
    /// A paragraph.
    P {
        /// The paragraph's text.
        text: String,
    },
    /// A bullet; `lead` is its bold start ("Delete albums by type."), when it has one.
    Li {
        /// The bold start, or "".
        #[serde(default, skip_serializing_if = "String::is_empty")]
        lead: String,
        /// The rest of the bullet.
        text: String,
    },
    /// A table row; `head` for the first.
    Row {
        /// The cells' texts.
        cells: Vec<String>,
        /// The header row.
        #[serde(default)]
        head: bool,
    },
    /// A part a newer server sends that this client does not know: drawn as nothing (`block_html`'s `_`).
    #[serde(other)]
    Unknown,
}

/// One release's notes.
///
/// From `changelog.rs` `Release`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// Its version ("0.36.0").
    pub version: String,
    /// "Encore 0.36.0", or what follows the version in the heading ("Encore 2 (Rust) is here").
    pub title: String,
    /// The paragraph before the first section.
    #[serde(default)]
    pub summary: String,
    /// What the popup lists: the bold leads of the bullets, else the first bullets.
    #[serde(default)]
    pub highlights: Vec<String>,
    /// The notes, part by part (`GET /api/changelog` only).
    #[serde(default)]
    pub blocks: Vec<Block>,
}

/// Bold, italics, code and links out of one line of markdown: `**Pause for everyone** is in [x](y)` → `Pause
/// for everyone is in x`.
///
/// From `changelog.rs` `plain`.
pub fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '*' | '`' => {}
            '_' if (i == 0 || !c[i - 1].is_alphanumeric()) || (i + 1 == c.len() || !c[i + 1].is_alphanumeric()) => {}
            '[' => {
                // [words](url) → words
                if let Some(close) = c[i..].iter().position(|&x| x == ']').map(|p| i + p) {
                    if c.get(close + 1) == Some(&'(') {
                        if let Some(end) = c[close..].iter().position(|&x| x == ')').map(|p| close + p) {
                            out.push_str(&plain(&c[i + 1..close].iter().collect::<String>()));
                            i = end + 1;
                            continue;
                        }
                    }
                }
                out.push('[');
            }
            x => out.push(x),
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A bullet's bold start and the rest: `**Delete albums by type.** Library → …` → ("Delete albums by type.",
/// "Library → …").
///
/// From `changelog.rs` `lead`.
fn lead(text: &str) -> (String, String) {
    if let Some(rest) = text.strip_prefix("**") {
        if let Some(end) = rest.find("**") {
            let lead = plain(&rest[..end]);
            let after = rest[end + 2..].trim_start_matches([':', ' ']).trim_start_matches(['–', '-']).trim();
            return (lead, plain(after));
        }
    }
    (String::new(), plain(text))
}

/// `s` cut to about `max` characters, ending on a sentence or a word, with "…".
///
/// From `changelog.rs` `short`.
pub fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    // end on a sentence when there is one, else on a word
    if let Some(p) = cut.rfind(". ").filter(|&p| p > max / 2) {
        return cut[..=p].to_string();
    }
    match cut.rfind(' ') {
        Some(p) => format!("{}…", cut[..p].trim_end_matches([',', ';', ':'])),
        None => format!("{cut}…"),
    }
}

/// One release's markdown (`docs/releases/<version>.md`) into its parts; `app` names the title when the
/// heading gives no name ("Encore 0.36.0").
///
/// From `changelog.rs` `parse` (where the app is always "Encore").
pub fn parse(app: &str, ver: &str, md: &str) -> Release {
    enum Open {
        None,
        P(String),
        Li(String),
    }
    let mut blocks = Vec::new();
    let mut title = format!("{app} {ver}");
    let mut open = Open::None;
    let mut in_table = false;
    let flush = |open: &mut Open, blocks: &mut Vec<Block>| match std::mem::replace(open, Open::None) {
        Open::P(t) if !t.trim().is_empty() => blocks.push(Block::P { text: plain(&t) }),
        Open::Li(t) => {
            let (lead, text) = lead(t.trim());
            blocks.push(Block::Li { lead, text });
        }
        _ => {}
    };
    for line in md.lines() {
        let t = line.trim();
        if t.is_empty() {
            flush(&mut open, &mut blocks);
            in_table = false;
            continue;
        }
        if let Some(h) = t.strip_prefix("## ") {
            flush(&mut open, &mut blocks);
            let h = plain(h);
            // "Encore 2026.10.11.1 – Encore 2 (Rust) is here" → what follows the version
            title = match h.split_once(" – ") {
                Some((_, rest)) if !rest.is_empty() => rest.to_string(),
                _ => h,
            };
            continue;
        }
        if let Some(h) = t.strip_prefix("### ").or_else(|| t.strip_prefix("#### ")) {
            flush(&mut open, &mut blocks);
            blocks.push(Block::H { text: plain(h) });
            continue;
        }
        if t == "---" {
            flush(&mut open, &mut blocks);
            continue;
        }
        if t.starts_with('|') {
            flush(&mut open, &mut blocks);
            let cells: Vec<String> = t.trim_matches('|').split('|').map(|c| plain(c.trim())).collect();
            // the |---|---| line under the head
            if cells.iter().all(|c| c.chars().all(|x| matches!(x, '-' | ':' | ' '))) {
                continue;
            }
            blocks.push(Block::Row { cells, head: !in_table });
            in_table = true;
            continue;
        }
        let bullet = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| {
            // "1. text"
            let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
            (d > 0).then(|| t[d..].strip_prefix(". ")).flatten()
        });
        match (bullet, &mut open) {
            (Some(b), _) => {
                flush(&mut open, &mut blocks);
                open = Open::Li(b.to_string());
            }
            (None, Open::Li(s)) | (None, Open::P(s)) => {
                s.push(' ');
                s.push_str(t);
            }
            (None, Open::None) => open = Open::P(t.to_string()),
        }
    }
    flush(&mut open, &mut blocks);

    let summary = blocks
        .iter()
        .take_while(|b| !matches!(b, Block::H { .. }))
        .find_map(|b| match b {
            Block::P { text } => Some(short(text, SUMMARY_CHARS)),
            _ => None,
        })
        .unwrap_or_default();
    let mut highlights: Vec<String> = blocks
        .iter()
        .filter_map(|b| match b {
            Block::Li { lead, .. } if !lead.is_empty() => Some(short(lead.trim_end_matches(['.', ':']), HIGHLIGHT_CHARS)),
            _ => None,
        })
        .take(HIGHLIGHTS)
        .collect();
    if highlights.is_empty() {
        highlights = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Li { text, .. } => Some(short(text, HIGHLIGHT_CHARS)),
                _ => None,
            })
            .take(PLAIN_HIGHLIGHTS)
            .collect();
    }
    Release { version: ver.to_string(), title, summary, highlights, blocks }
}

/// The releases a server at `current` includes (a hotfix includes its release, never notes written ahead for
/// the next one), newest first as `all` is.
///
/// From `changelog.rs` `all` (the filter of `GET /api/changelog`).
pub fn included<'a>(all: &'a [Release], current: &str) -> Vec<&'a Release> {
    let cur = version::base(current);
    all.iter().filter(|r| version::compare(&r.version, &cur) != Ordering::Greater).collect()
}

/// The releases that are new for an account: those this server (`current`) includes, newest first; with
/// `seen` (the version the account last saw), only those newer than its release, at most [`POPUP_RELEASES`].
/// Without `seen` (an account that never saw the popup) only the newest one. Hotfixes count as their release.
///
/// From `changelog.rs` `unseen`.
pub fn unseen<'a>(all: &'a [Release], current: &str, seen: Option<&str>) -> Vec<&'a Release> {
    let mut out = included(all, current);
    match seen.filter(|s| !version::parts(s).is_empty()) {
        Some(s) => {
            let s = version::base(s);
            out.retain(|r| version::newer(&r.version, &s));
            out.truncate(POPUP_RELEASES);
        }
        None => out.truncate(1),
    }
    out
}

/// The account's popup switch: on unless the person switched it off (`prefs.whats_new_popup`).
///
/// From `changelog.rs` `popup_on`.
pub fn popup_on(pref: Option<bool>) -> bool {
    pref.unwrap_or(true)
}

/// Whether the popup shows: the switch is on and something is new (`show` of `GET /api/changelog/new`).
///
/// From `changelog.rs` `new`.
pub fn show_popup(popup: bool, unseen: usize) -> bool {
    popup && unseen > 0
}

/// One bullet of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The bold start (drawn bold, followed by a space), or "".
    pub lead: String,
    /// The rest of the bullet.
    pub text: String,
}

/// One table row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The cells' texts.
    pub cells: Vec<String>,
    /// The header row (`th` cells on the web, semibold on the desktop).
    pub head: bool,
}

/// A release's notes as they are drawn: bullets grouped into lists, table rows into tables.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Group {
    /// A heading.
    Heading(String),
    /// A paragraph.
    Paragraph(String),
    /// Bullets that follow each other.
    List(Vec<Item>),
    /// Table rows that follow each other.
    Table(Vec<Row>),
}

/// A release's notes grouped for drawing: bullets that follow each other are one list, rows one table; an
/// unknown block ends a list or a table and draws nothing.
///
/// From `web-rs/src/whats_new.rs` `notes_html`.
pub fn group(blocks: &[Block]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    // whether the last group is still open for more bullets or rows (an unknown block closes it)
    let mut open = false;
    for b in blocks {
        match b {
            Block::Li { lead, text } => {
                let item = Item { lead: lead.clone(), text: text.clone() };
                match out.last_mut() {
                    Some(Group::List(l)) if open => l.push(item),
                    _ => out.push(Group::List(vec![item])),
                }
                open = true;
            }
            Block::Row { cells, head } => {
                let row = Row { cells: cells.clone(), head: *head };
                match out.last_mut() {
                    Some(Group::Table(t)) if open => t.push(row),
                    _ => out.push(Group::Table(vec![row])),
                }
                open = true;
            }
            Block::H { text } => {
                out.push(Group::Heading(text.clone()));
                open = false;
            }
            Block::P { text } => {
                out.push(Group::Paragraph(text.clone()));
                open = false;
            }
            Block::Unknown => open = false,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "## Encore 0.36.0

**Clean up your library by type, see who requested what.** If you skipped
0.35.2, its fixes are in [0.35.2](https://example.org/0.35.2.md).

### Library
- **Delete albums by type.** Library → Albums has a **Type** filter: Studio albums,
  Live, Compilations.
- How the type is known: `RELEASETYPE`, then the title.

### What runs at once
| Source | At once |
|---|---|
| streamrip | 1 on Deezer |

### Before you update
- The database goes from schema 30 to 31.
";

    /// Ported from `changelog_tests.rs` `notes_become_blocks`.
    #[test]
    fn notes_become_blocks() {
        let r = parse("Encore", "0.36.0", MD);
        assert_eq!(r.title, "Encore 0.36.0");
        assert_eq!(r.summary, "Clean up your library by type, see who requested what. If you skipped 0.35.2, its fixes are in 0.35.2.");
        assert_eq!(r.highlights, vec!["Delete albums by type".to_string()]);
        assert_eq!(r.blocks[1], Block::H { text: "Library".into() });
        assert_eq!(
            r.blocks[2],
            Block::Li {
                lead: "Delete albums by type.".into(),
                text: "Library → Albums has a Type filter: Studio albums, Live, Compilations.".into()
            }
        );
        assert_eq!(r.blocks[3], Block::Li { lead: String::new(), text: "How the type is known: RELEASETYPE, then the title.".into() });
        assert_eq!(r.blocks[5], Block::Row { cells: vec!["Source".into(), "At once".into()], head: true });
        assert_eq!(r.blocks[6], Block::Row { cells: vec!["streamrip".into(), "1 on Deezer".into()], head: false });
        assert_eq!(r.blocks.len(), 9);
    }

    /// Ported from `changelog_tests.rs` `a_heading_with_a_name_keeps_the_name`.
    #[test]
    fn a_heading_with_a_name_keeps_the_name() {
        assert_eq!(parse("Encore", "1.0.0", "## Encore 1.0.0 – Encore 2 (Rust) is here\n\nText.").title, "Encore 2 (Rust) is here");
    }

    /// Ported from `changelog_tests.rs` `without_bold_leads_the_first_bullets_are_the_highlights`.
    #[test]
    fn without_bold_leads_the_first_bullets_are_the_highlights() {
        let r = parse("Encore", "0.1.0", "## Encore 0.1.0\n\n### Fixed\n- One thing.\n- Another thing.\n");
        assert_eq!(r.highlights, vec!["One thing.".to_string(), "Another thing.".to_string()]);
    }

    /// Ported from `changelog_tests.rs` `plain_keeps_the_words`.
    #[test]
    fn plain_keeps_the_words() {
        assert_eq!(
            plain("**Pause for everyone** is in the *player* `menu` – see [docs](http://x/y)"),
            "Pause for everyone is in the player menu – see docs"
        );
        assert_eq!(plain("snake_case stays, _italic_ goes"), "snake_case stays, italic goes");
    }

    fn rel(v: &str) -> Release {
        parse("Encore", v, &format!("## Encore {v}\n"))
    }

    /// Ported from `changelog_tests.rs` `only_releases_after_the_seen_one_and_not_past_this_server`.
    #[test]
    fn only_releases_after_the_seen_one_and_not_past_this_server() {
        let all = vec![rel("0.37.0"), rel("0.36.1"), rel("0.36.0"), rel("0.35.2")];
        let v = |l: Vec<&Release>| l.iter().map(|r| r.version.clone()).collect::<Vec<_>>();
        // a hotfix build of 0.36.1 includes 0.36.1, never the notes written ahead for 0.37.0
        assert_eq!(v(unseen(&all, "0.36.1.17", Some("0.35.2"))), vec!["0.36.1", "0.36.0"]);
        // another hotfix of a release already seen: nothing new
        assert_eq!(v(unseen(&all, "0.36.1.18", Some("0.36.1.17"))), Vec::<String>::new());
        // never seen: the newest only
        assert_eq!(v(unseen(&all, "0.36.1", None)), vec!["0.36.1"]);
        assert_eq!(v(unseen(&all, "0.36.1", Some(""))), vec!["0.36.1"]);
        assert_eq!(v(included(&all, "0.36.1.17")), vec!["0.36.1", "0.36.0", "0.35.2"]);
    }

    #[test]
    fn the_popup_shows_once_something_is_new_and_the_switch_is_on() {
        let all: Vec<Release> = (0..6).map(|i| rel(&format!("0.3{}.0", 9 - i))).collect();
        assert_eq!(unseen(&all, "0.39.0", Some("0.30.0")).len(), POPUP_RELEASES);
        assert!(popup_on(None) && popup_on(Some(true)) && !popup_on(Some(false)));
        assert!(show_popup(true, 1));
        assert!(!show_popup(false, 3));
        assert!(!show_popup(true, 0));
    }

    #[test]
    fn a_long_summary_ends_on_a_sentence_or_a_word() {
        assert_eq!(short("short", 10), "short");
        assert_eq!(short("One two three four five six", 12), "One two…");
        assert_eq!(short("First sentence here. Second one goes on", 30), "First sentence here.");
        assert_eq!(short("abcdefghijkl", 5), "abcde…");
    }

    /// The grouping of `web-rs/src/whats_new.rs` `notes_html`.
    #[test]
    fn bullets_and_rows_are_grouped_into_lists_and_tables() {
        let r = parse("Encore", "0.36.0", MD);
        let g = group(&r.blocks);
        assert_eq!(g[0], Group::Paragraph(r.summary.clone()));
        assert_eq!(g[1], Group::Heading("Library".into()));
        assert!(matches!(&g[2], Group::List(l) if l.len() == 2 && l[0].lead == "Delete albums by type."));
        assert_eq!(g[3], Group::Heading("What runs at once".into()));
        assert!(matches!(&g[4], Group::Table(t) if t.len() == 2 && t[0].head && !t[1].head));
        assert_eq!(g.len(), 7);
        // an unknown block (from a newer server) closes the list and draws nothing
        let li = |t: &str| Block::Li { lead: String::new(), text: t.into() };
        let g = group(&[li("a"), Block::Unknown, li("b"), li("c")]);
        assert_eq!(g.len(), 2);
        // a row right after a bullet starts a table
        let g = group(&[li("a"), Block::Row { cells: vec!["x".into()], head: true }]);
        assert!(matches!(g.as_slice(), [Group::List(_), Group::Table(_)]));
    }

    #[test]
    fn blocks_read_and_write_the_contract() {
        let blocks: Vec<Block> = serde_json::from_str(
            r#"[{"k":"h","text":"Library"},{"k":"li","text":"x"},{"k":"li","lead":"L.","text":"y"},{"k":"row","cells":["a"],"head":true},{"k":"chart","data":1}]"#,
        )
        .unwrap();
        assert_eq!(blocks[1], Block::Li { lead: String::new(), text: "x".into() });
        assert_eq!(blocks[4], Block::Unknown);
        assert_eq!(serde_json::to_string(&blocks[1]).unwrap(), r#"{"k":"li","text":"x"}"#, "an empty lead is left out");
        assert_eq!(serde_json::to_string(&blocks[3]).unwrap(), r#"{"k":"row","cells":["a"],"head":true}"#);
        let popup: Release =
            serde_json::from_str(r#"{"version":"0.36.4","title":"Encore 0.36.4","summary":"s","highlights":["h"]}"#).unwrap();
        assert!(popup.blocks.is_empty());
    }

    #[test]
    fn the_words_are_encores() {
        assert_eq!(saved_words(true), "What's new will show after updates");
        assert_eq!(saved_words(false), "What's new won't show after updates – it stays in Settings");
        assert_eq!(
            settings_intro("Encore", "0.36.4"),
            "This server runs Encore 0.36.4. After an update, a short list of what changed shows once on the website and in the desktop app."
        );
    }
}
