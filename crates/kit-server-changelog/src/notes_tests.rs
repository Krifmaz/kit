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

#[test]
fn a_heading_with_a_name_keeps_the_name() {
    assert_eq!(parse("Encore", "1.0.0", "## Encore 1.0.0 – Encore 2 (Rust) is here\n\nText.").title, "Encore 2 (Rust) is here");
}

#[test]
fn without_bold_leads_the_first_bullets_are_the_highlights() {
    let r = parse("Encore", "0.1.0", "## Encore 0.1.0\n\n### Fixed\n- One thing.\n- Another thing.\n");
    assert_eq!(r.highlights, vec!["One thing.".to_string(), "Another thing.".to_string()]);
}

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
}

#[test]
fn hotfixes_share_their_release_and_long_text_is_cut() {
    assert_eq!(base("0.36.0.17"), "0.36.0");
    let long = format!(
        "## X 1.0.0

{}

- **{}** x
",
        "word ".repeat(100),
        "lead ".repeat(40)
    );
    let r = parse("X", "1.0.0", &long);
    assert!(r.summary.ends_with('…') && r.summary.chars().count() <= 321, "{}", r.summary);
    assert!(r.highlights[0].ends_with('…') && r.highlights[0].chars().count() <= 111);
    assert_eq!(parse("Filebrary", "0.1.0", "").title, "Filebrary 0.1.0");
    // a summary that has a sentence end past half its length ends there
    let r = parse("X", "1.0.0", &format!("{}. {}", "a".repeat(200), "b ".repeat(100)));
    assert_eq!(r.summary, format!("{}.", "a".repeat(200)));
}
