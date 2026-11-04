use super::*;

const MD: &str = "## Demo 1.3.0

**Sort your projects by type, see who changed what.** If you skipped
1.2.1, its fixes are in [1.2.1](https://example.org/1.2.1.md).

### Projects
- **Archive projects by type.** Projects → All has a **Type** filter: Drafts,
  Shared, Templates.
- How the type is known: `KIND`, then the name.

### What runs at once
| Job | At once |
|---|---|
| export | 1 per account |

### Before you update
- The database goes from schema 4 to 5.
";

#[test]
fn notes_become_blocks() {
    let r = parse("Demo", "1.3.0", MD);
    assert_eq!(r.title, "Demo 1.3.0");
    assert_eq!(r.summary, "Sort your projects by type, see who changed what. If you skipped 1.2.1, its fixes are in 1.2.1.");
    assert_eq!(r.highlights, vec!["Archive projects by type".to_string()]);
    assert_eq!(r.blocks[1], Block::H { text: "Projects".into() });
    assert_eq!(
        r.blocks[2],
        Block::Li {
            lead: "Archive projects by type.".into(), text: "Projects → All has a Type filter: Drafts, Shared, Templates.".into()
        }
    );
    assert_eq!(r.blocks[3], Block::Li { lead: String::new(), text: "How the type is known: KIND, then the name.".into() });
    assert_eq!(r.blocks[5], Block::Row { cells: vec!["Job".into(), "At once".into()], head: true });
    assert_eq!(r.blocks[6], Block::Row { cells: vec!["export".into(), "1 per account".into()], head: false });
    assert_eq!(r.blocks.len(), 9);
}

#[test]
fn a_heading_with_a_name_keeps_the_name() {
    assert_eq!(parse("Demo", "2.0.0", "## Demo 2.0.0 – Demo 2 is here\n\nText.").title, "Demo 2 is here");
}

#[test]
fn without_bold_leads_the_first_bullets_are_the_highlights() {
    let r = parse("Demo", "0.1.0", "## Demo 0.1.0\n\n### Fixed\n- One thing.\n- Another thing.\n");
    assert_eq!(r.highlights, vec!["One thing.".to_string(), "Another thing.".to_string()]);
}

#[test]
fn plain_keeps_the_words() {
    assert_eq!(
        plain("**Share with everyone** is in the *project* `menu` – see [docs](http://x/y)"),
        "Share with everyone is in the project menu – see docs"
    );
    assert_eq!(plain("snake_case stays, _italic_ goes"), "snake_case stays, italic goes");
}

fn rel(v: &str) -> Release {
    parse("Demo", v, &format!("## Demo {v}\n"))
}

#[test]
fn only_releases_after_the_seen_one_and_not_past_this_server() {
    let all = vec![rel("1.5.0"), rel("1.4.0"), rel("1.3.0"), rel("1.2.1")];
    let v = |l: Vec<&Release>| l.iter().map(|r| r.version.clone()).collect::<Vec<_>>();
    // a hotfix build of 1.4.0 includes 1.4.0, never the notes written ahead for 1.5.0
    assert_eq!(v(unseen(&all, "1.4.0.17", Some("1.2.1"))), vec!["1.4.0", "1.3.0"]);
    // another hotfix of a release already seen: nothing new
    assert_eq!(v(unseen(&all, "1.4.0.18", Some("1.4.0.17"))), Vec::<String>::new());
    // never seen: the newest only
    assert_eq!(v(unseen(&all, "1.4.0", None)), vec!["1.4.0"]);
    assert_eq!(v(unseen(&all, "1.4.0", Some(""))), vec!["1.4.0"]);
}

#[test]
fn hotfixes_share_their_release_and_long_text_is_cut() {
    assert_eq!(base("1.3.0.17"), "1.3.0");
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
    assert_eq!(parse("Acme", "0.1.0", "").title, "Acme 0.1.0");
    // a summary that has a sentence end past half its length ends there
    let r = parse("X", "1.0.0", &format!("{}. {}", "a".repeat(200), "b ".repeat(100)));
    assert_eq!(r.summary, format!("{}.", "a".repeat(200)));
}
