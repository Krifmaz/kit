//! Every output carries the tokens' values, and design/out is up to date.

use super::*;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn tokens() -> Tokens {
    Tokens::load(&root().join("design/tokens.json")).expect("tokens.json loads")
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
    s.split('}')
        .filter_map(|block| {
            let (sel, body) = block.split_once('{')?;
            let decls =
                body.split(';').filter_map(|d| d.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
            Some((sel.trim().to_string(), decls))
        })
        .collect()
}

/// The value a theme's block, or failing that `:root`, gives a variable: what the browser resolves.
fn resolved(blocks: &[(String, Vec<(String, String)>)], selector: &str, var: &str) -> Option<String> {
    let find = |sel: &str| blocks.iter().find(|b| b.0 == sel).and_then(|b| b.1.iter().find(|(k, _)| k == var).map(|(_, v)| v.clone()));
    find(selector).or_else(|| find(":root"))
}

#[test]
fn css_resolves_every_theme_to_its_tokens() {
    let t = tokens();
    let blocks = css_blocks(&css(&t));
    assert_eq!(blocks[0].0, ":root", "the default theme is :root");
    assert_eq!(blocks.len(), t.themes.len());
    for th in &t.themes {
        for k in SURFACE.iter().chain(ACCENT.iter()) {
            assert_eq!(resolved(&blocks, &th.css_selector, &format!("--{k}")).as_deref(), Some(th.colors[*k].as_str()), "{} --{k}", th.id);
        }
        let (r, g, b) = rgb(&th.colors["accent"]);
        assert_eq!(resolved(&blocks, &th.css_selector, "--accent-rgb"), Some(format!("{r}, {g}, {b}")), "{} --accent-rgb", th.id);
        assert_eq!(resolved(&blocks, &th.css_selector, "--shadow").as_deref(), Some(th.shadow.as_str()), "{} --shadow", th.id);
        let scheme = if th.dark { "dark" } else { "light" };
        assert_eq!(resolved(&blocks, &th.css_selector, "color-scheme").as_deref(), Some(scheme), "{} color-scheme", th.id);
    }
    for (var, v) in [("--ok", &t.status.ok), ("--warn", &t.status.warn), ("--err", &t.status.err), ("--info", &t.status.info)] {
        assert_eq!(resolved(&blocks, ":root", var).as_deref(), Some(v.as_str()));
    }
}

#[test]
fn css_theme_blocks_hold_only_what_differs() {
    let t = tokens();
    let blocks = css_blocks(&css(&t));
    let root = &blocks[0].1;
    for b in &blocks[1..] {
        for (k, v) in &b.1 {
            assert_ne!(root.iter().find(|(rk, _)| rk == k).map(|(_, rv)| rv), Some(v), "{} repeats {k}", b.0);
        }
    }
}

#[test]
fn compose_has_every_palette_accent_preset_shape_and_type_role() {
    let t = tokens();
    let kt = compose(&t);
    for th in &t.themes {
        let name = camel(&th.id);
        let start = kt.find(&format!("val {name}: KitPalette = KitPalette(")).unwrap_or_else(|| panic!("palette {name}"));
        let body = &kt[start..start + kt[start..].find("\n    )").expect("end")];
        for (f, k) in [("bg", "bg"), ("text", "text"), ("accent", "accent"), ("accent2", "accent-2"), ("onAccent", "on-accent")] {
            let want = format!("{f} = Color(0xFF{}),", hex6(&th.colors[k]).unwrap()[1..].to_ascii_uppercase());
            assert!(body.contains(&want), "{name}: {want}");
        }
        assert!(body.contains(&format!("dark = {},", th.dark)));
    }
    for a in &t.accents {
        assert!(kt.contains(&format!("KitAccent(\"{}\", \"{}\", \"{}\", \"{}\")", a.id, a.name, a.a, a.b)), "accent {}", a.id);
    }
    for p in &t.presets {
        assert!(kt.contains(&format!("KitPreset(\"{}\", \"{}\", \"{}\", \"{}\"", p.id, p.name, p.theme, p.accent)), "preset {}", p.id);
    }
    for (k, v) in &t.compose.shapes {
        assert!(kt.contains(&format!("{k} = RoundedCornerShape({v}.dp)")), "shape {k}");
    }
    for r in &t.compose.typography {
        assert!(kt.contains(&format!("    {} = TextStyle(", r.role)), "type role {}", r.role);
    }
}

/// Every string literal in `src`, in order.
fn strings(src: &str) -> Vec<String> {
    src.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

#[test]
fn iced_tokens_cover_every_theme_and_fall_back_to_the_default() {
    let t = tokens();
    let rs = iced(&t);
    let arms = |fun: &str| {
        let i = rs.find(&format!("pub fn {fun}(")).expect(fun);
        rs[i..i + rs[i..].find("\n}").unwrap()].lines().filter(|l| l.contains("=>")).map(str::trim).map(String::from).collect::<Vec<_>>()
    };
    let surface = arms("surface");
    let accent = arms("accent");
    for th in &t.themes {
        let id = if th.id == "encore" { "_".to_string() } else { format!("\"{}\"", th.id) };
        let s = surface.iter().find(|l| l.starts_with(&format!("{id} =>"))).unwrap_or_else(|| panic!("surface {id}"));
        let want: Vec<String> = SURFACE.iter().map(|k| hex6(&th.colors[*k]).unwrap()).collect();
        assert_eq!(strings(s).into_iter().filter(|x| x.starts_with('#')).collect::<Vec<_>>(), want, "surface {id}");
        let a = accent.iter().find(|l| l.starts_with(&format!("{id} =>"))).unwrap_or_else(|| panic!("accent {id}"));
        let want: Vec<String> = ACCENT.iter().map(|k| hex6(&th.colors[*k]).unwrap()).collect();
        assert_eq!(strings(a).into_iter().filter(|x| x.starts_with('#')).collect::<Vec<_>>(), want, "accent {id}");
    }
    let picker: Vec<String> = t.theme_picker.iter().flat_map(|c| [c.id.clone(), c.name.clone()]).collect();
    let i = rs.find("pub const THEMES").unwrap();
    assert_eq!(strings(&rs[i..i + rs[i..].find("];").unwrap()]), picker);
}

#[test]
fn flutter_has_a_palette_per_theme() {
    let t = tokens();
    let dart = flutter(&t);
    for th in &t.themes {
        assert!(dart.contains(&format!("static const KitPalette {} = KitPalette(", th.id)), "{}", th.id);
    }
    assert!(dart.contains("ThemeData kitThemeData(KitPalette p)"));
}

#[test]
fn on_accent_is_dark_on_light_accents() {
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
    let text = std::fs::read_to_string(root().join("design/tokens.json")).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["themes"][1]["colors"]["bg"] = "teal".into();
    assert_eq!(Tokens::parse(&v.to_string()).unwrap_err().to_string(), "Theme dark: bg = teal is not a hex colour.");
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["presets"][0]["accent"] = "nope".into();
    assert_eq!(Tokens::parse(&v.to_string()).unwrap_err().to_string(), "Preset encore names a missing accent nope.");
}

#[test]
fn design_out_is_up_to_date() {
    for (name, text) in outputs(&tokens()) {
        let on_disk = std::fs::read_to_string(root().join("design/out").join(name)).unwrap_or_default();
        assert!(on_disk == text, "design/out/{name} is stale: run `cargo run -p kit-tokens`");
    }
}

#[test]
fn the_scales_reach_every_output() {
    let t = tokens();
    let css = css(&t);
    for want in [
        "--space-4: 16px;",
        "--text-3xl: 36px;",
        "--touch: 44px;",
        "--tab-bar: 64px;",
        "--shadow-2:",
        "--dur-pop: .18s;",
        "--glass-blur: 18px;",
    ] {
        assert!(css.contains(want), "{want}");
    }
    // the light theme gets the lighter elevations
    let light = &css[css.find(":root[data-theme=\"light\"]").expect("light block")..];
    assert!(light.contains("--shadow-1: 0 1px 2px rgba(20, 20, 50, .08);"));
    let kt = compose(&t);
    for want in [
        "object KitSpace",
        "val S4 = 16.dp",
        "object KitSizes",
        "val Touch = 44.dp",
        "val Breakpoint = 768.dp",
        "object KitMotion",
        "const val POP = 180",
        "const val MAX = 180",
    ] {
        assert!(kt.contains(want), "{want}");
    }
    let rs = iced(&t);
    assert!(rs.contains("pub const TOUCH: f32 = 44.0;") && rs.contains("pub const SPACE: [f32; 10]"));
    // no animation token runs longer than the scale allows
    let ms = |s: &str| (s.trim_end_matches('s').parse::<f64>().unwrap_or(9.0) * 1000.0).round() as u32;
    for d in [&t.motion.fast, &t.motion.base, &t.motion.pop] {
        assert!(ms(d) <= t.scale.max_motion_ms, "{d}");
    }
}
