//! The generated themes equal Encore's current ones (design/fixtures/encore), and design/out is up to date.

use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tokens() -> Tokens {
    Tokens::load(&root().join("design/tokens.json")).expect("tokens.json loads")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(root().join("design/fixtures/encore").join(name)).expect("fixture")
}

/// CSS without comments, as (selector, [(property, value)]) blocks in order.
fn css_blocks(src: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut s = String::new();
    let mut rest = src;
    while let Some(i) = rest.find("/*") {
        s.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map(|j| &rest[i + j + 2..]).unwrap_or("");
    }
    s.push_str(rest);
    let mut out = Vec::new();
    for block in s.split('}') {
        let Some((sel, body)) = block.split_once('{') else { continue };
        let decls = body.split(';').filter_map(|d| d.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
        out.push((sel.trim().to_string(), decls));
    }
    out
}

#[test]
fn css_equals_encores_stylesheet() {
    let want = css_blocks(&fixture("app.css"));
    let got = css_blocks(&css(&tokens()));
    assert_eq!(got.len(), want.len(), "same blocks");
    for (g, w) in got.iter().zip(&want) {
        assert_eq!(g, w, "block {}", w.0);
    }
}

/// `val Name = Color(0xFFAABBCC)` lines of `object EncoreColors`.
fn kt_named_colors(src: &str) -> BTreeMap<String, String> {
    let start = src.find("object EncoreColors").expect("EncoreColors");
    let body = &src[start..start + src[start..].find("\n}").expect("end")];
    body.lines()
        .filter_map(|l| {
            let l = l.trim().strip_prefix("val ")?;
            let (name, v) = l.split_once(" = Color(0xFF")?;
            Some((name.trim().to_string(), format!("#{}", v[..6].to_ascii_lowercase())))
        })
        .collect()
}

/// A Kotlin colour expression as `#rrggbb`.
fn kt_value(expr: &str, named: &BTreeMap<String, String>) -> String {
    let e = expr.trim().trim_end_matches(',');
    if let Some(v) = e.strip_prefix("Color(0xFF") {
        return format!("#{}", v[..6].to_ascii_lowercase());
    }
    if let Some(n) = e.strip_prefix("EncoreColors.") {
        return named[n].clone();
    }
    match e {
        "Color.Black" => "#000000".into(),
        "Color.White" => "#ffffff".into(),
        other => panic!("unknown colour {other}"),
    }
}

/// The fields of `val NAME: EncorePalette = EncorePalette(…)`.
fn kt_palette(src: &str, name: &str) -> BTreeMap<String, String> {
    let named = kt_named_colors(src);
    let start = src.find(&format!("val {name}: EncorePalette = EncorePalette(")).unwrap_or_else(|| panic!("{name}"));
    let body = &src[start..start + src[start..].find("\n)").expect("end")];
    body.lines()
        .skip(1)
        .filter_map(|l| l.trim().split_once(" = "))
        .filter(|(k, _)| *k != "dark")
        .map(|(k, v)| (k.to_string(), kt_value(v, &named)))
        .collect()
}

#[test]
fn compose_palettes_equal_encores_theme() {
    let t = tokens();
    let src = fixture("Theme.kt");
    for th in &t.themes {
        let want = kt_palette(&src, &t.compose.palette_names[&th.id]);
        let p = t.compose_palette(th);
        let got: BTreeMap<String, String> = [
            ("bg", "bg"),
            ("bg2", "bg-2"),
            ("panel", "panel"),
            ("panel2", "panel-2"),
            ("border", "border"),
            ("text", "text"),
            ("muted", "muted"),
            ("faint", "faint"),
            ("accent", "accent"),
            ("accent2", "accent-2"),
            ("onAccent", "on-accent"),
        ]
        .iter()
        .map(|(f, k)| (f.to_string(), p[*k].clone()))
        .collect();
        assert_eq!(got, want, "theme {}", th.id);
    }
}

/// Every string literal in `src`, in order.
fn strings(src: &str) -> Vec<String> {
    src.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// The text from `start` up to the first `end` after it.
fn between<'a>(src: &'a str, start: &str, end: &str) -> &'a str {
    let i = src.find(start).unwrap_or_else(|| panic!("{start}"));
    let rest = &src[i + start.len()..];
    &rest[..rest.find(end).unwrap_or(rest.len())]
}

#[test]
fn compose_accents_presets_shapes_and_type_equal_encores() {
    let t = tokens();
    let src = fixture("Theme.kt");
    let acc: Vec<String> = t.accents.iter().flat_map(|a| [a.id.clone(), a.name.clone(), a.a.clone(), a.b.clone()]).collect();
    assert_eq!(strings(between(&src, "val ACCENTS: List<Accent> = listOf(", "\n    )")), acc);
    // Encore's Kotlin names the themes by its enum (ThemeChoice.DARK …); the strings are id, name, accent, custom?, background
    let presets = between(&src, "val PRESETS: List<Preset> = listOf(", "\n    )");
    for p in &t.presets {
        let line = presets.lines().find(|l| l.contains(&format!("Preset(\"{}\"", p.id))).unwrap_or_else(|| panic!("preset {}", p.id));
        assert!(line.contains(&format!("ThemeChoice.{}", p.theme.to_ascii_uppercase())), "{line}");
        let mut want = vec![p.id.clone(), p.name.clone(), p.accent.clone()];
        want.extend(p.custom.clone());
        want.push(p.background.clone());
        assert_eq!(strings(line), want, "{line}");
    }
    assert!(src.contains(&format!("const val DEFAULT_CUSTOM = \"{}\"", t.default_custom)));
    for (k, v) in &t.compose.shapes {
        assert!(src.contains(&format!("{k} = RoundedCornerShape({v}.dp)")), "shape {k}");
    }
    let ty = fixture("Type.kt");
    let gen = compose(&t);
    for r in &t.compose.typography {
        let line =
            ty.lines().find(|l| l.trim_start().starts_with(&format!("{} = TextStyle(", r.role))).unwrap_or_else(|| panic!("{}", r.role));
        assert!(gen.contains(line.trim()), "type role {} differs: {line}", r.role);
    }
}

#[test]
fn iced_tokens_equal_encores_desktop_theme() {
    let t = tokens();
    let src = fixture("desktop_theme.rs.txt");
    let gen = iced(&t);
    for (start, end) in [("pub const THEMES", "];"), ("pub const ACCENTS", "];"), ("pub const PRESETS", "];")] {
        assert_eq!(strings(between(&gen, start, end)), strings(between(&src, start, end)), "{start}");
    }
    let arms = between(&src, "let (bg, bg2, panel, panel2, border, text, muted, faint) = match theme {", "};");
    for th in &t.themes {
        let id = if th.id == "default" { "_".to_string() } else { format!("\"{}\"", t.iced.theme_ids[&th.id]) };
        let line = arms.lines().find(|l| l.trim_start().starts_with(&format!("{id} =>"))).unwrap_or_else(|| panic!("arm {id}"));
        let want: Vec<String> = strings(line).into_iter().filter(|s| s.starts_with('#')).collect();
        let got: Vec<String> = SURFACE.iter().map(|k| hex6(&th.colors[*k]).unwrap()).collect();
        assert_eq!(got, want, "desktop theme {id}");
    }
    for (name, v) in [("RADIUS", t.iced.radius), ("SIDEBAR", t.iced.sidebar), ("PLAYER_H", t.iced.player_h)] {
        assert!(src.contains(&format!("pub const {name}: f32 = {v:?};")), "{name}");
    }
    for c in [&t.status.ok, &t.status.warn, &t.status.err, &t.status.info] {
        assert!(src.contains(&format!("hex(\"{c}\")")), "status {c}");
    }
}

#[test]
fn on_accent_follows_appearance_rs() {
    let t = tokens();
    assert_eq!(t.on_accent_for("#f0ead6", "#d6c49a"), "#16161f");
    assert_eq!(t.on_accent_for("#8b5cf6", "#ec4899"), "#fff");
}

#[test]
fn hex_normalises() {
    assert_eq!(hex6("#000").as_deref(), Some("#000000"));
    assert_eq!(hex6("FFF").as_deref(), Some("#ffffff"));
    assert_eq!(hex6("#12345"), None);
    assert_eq!(rgb("#8b5cf6"), (139, 92, 246));
}

#[test]
fn bad_tokens_say_what_is_wrong() {
    let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root().join("design/tokens.json")).unwrap()).unwrap();
    v["themes"][1]["colors"]["bg"] = "teal".into();
    let e = Tokens::parse(&v.to_string()).unwrap_err();
    assert_eq!(e.to_string(), "Theme encore: bg = teal is not a hex colour.");
}

#[test]
fn design_out_is_up_to_date() {
    for (name, text) in outputs(&tokens()) {
        let on_disk = std::fs::read_to_string(root().join("design/out").join(name)).unwrap_or_default();
        assert!(on_disk == text, "design/out/{name} is stale: run `cargo run -p kit-tokens`");
    }
}
