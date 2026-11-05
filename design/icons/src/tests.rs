use super::*;

fn colors() -> Colors {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tokens.json");
    Colors::from_theme(&Tokens::load(&root).expect("tokens"), "encore").expect("encore")
}

fn app() -> App {
    App { name: "Kit Demo".into(), short_name: None, start_url: None, base: None }
}

const GLYPH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="8" fill="currentColor"/><circle cx="12" cy="12" r="3" fill="{accent-2}"/></svg>"#;

#[test]
fn encore_colours_come_from_the_tokens() {
    let c = colors();
    assert_eq!(c.tile_bottom, "#161617");
    assert_eq!(c.glyph, "#f0ead6");
    assert_eq!(c.accent, "#d6c49a");
}

#[test]
fn every_file_is_made_in_order() {
    let names: Vec<_> = render(GLYPH, &colors(), &app()).expect("render").into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        names,
        [
            "favicon.svg",
            "icon-32.png",
            "icon-192.png",
            "icon-512.png",
            "icon-maskable-512.png",
            "apple-touch-icon.png",
            "manifest.webmanifest",
            "head.html"
        ]
    );
}

fn pixmap(files: &[File], name: &str) -> tiny_skia::Pixmap {
    let bytes = &files.iter().find(|(n, _)| *n == name).expect("file").1;
    tiny_skia::Pixmap::decode_png(bytes).expect("png")
}

#[test]
fn pngs_have_their_sizes_and_shapes() {
    let files = render(GLYPH, &colors(), &app()).expect("render");
    for (name, px) in [("icon-32.png", 32), ("icon-192.png", 192), ("icon-512.png", 512), ("apple-touch-icon.png", 180)] {
        let p = pixmap(&files, name);
        assert_eq!((p.width(), p.height()), (px, px), "{name}");
    }
    // the rounded tile is see-through in its corners; the Apple and maskable icons are opaque everywhere
    assert_eq!(pixmap(&files, "icon-512.png").pixel(0, 0).expect("px").alpha(), 0);
    assert_eq!(pixmap(&files, "apple-touch-icon.png").pixel(0, 0).expect("px").alpha(), 255);
    assert_eq!(pixmap(&files, "icon-maskable-512.png").pixel(0, 0).expect("px").alpha(), 255);
}

#[test]
fn the_glyph_is_drawn_in_the_theme_colours() {
    let files = render(GLYPH, &colors(), &app()).expect("render");
    let p = pixmap(&files, "icon-512.png");
    // the centre is the {accent-2} dot, a ring further out is currentColor (cream)
    let centre = p.pixel(256, 256).expect("px");
    assert_eq!((centre.red(), centre.green(), centre.blue()), (0xd6, 0xc4, 0x9a));
    let ring = p.pixel(256, 256 - 100).expect("px");
    assert_eq!((ring.red(), ring.green(), ring.blue()), (0xf0, 0xea, 0xd6));
    let svg = String::from_utf8(files[0].1.clone()).expect("utf8");
    assert!(!svg.contains("currentColor") && !svg.contains("{accent-2}"));
}

#[test]
fn the_maskable_glyph_stays_in_the_safe_zone() {
    let files = render(GLYPH, &colors(), &app()).expect("render");
    let p = pixmap(&files, "icon-maskable-512.png");
    // the glyph's box is the middle 50 %, inside the 80 % safe circle: nothing cream outside a 0.4 × 512 radius
    for (x, y) in [(256, 256 - 210), (256 + 210, 256), (60, 60)] {
        let c = p.pixel(x, y).expect("px");
        assert_ne!((c.red(), c.green(), c.blue()), (0xf0, 0xea, 0xd6), "({x}, {y})");
    }
}

#[test]
fn the_manifest_names_the_app_and_its_icons() {
    let files = render(GLYPH, &colors(), &App { short_name: Some("Demo".into()), ..app() }).expect("render");
    let text = String::from_utf8(files[6].1.clone()).expect("utf8");
    let m: serde_json::Value = serde_json::from_str(&text).expect("json");
    assert_eq!(m["name"], "Kit Demo");
    assert_eq!(m["short_name"], "Demo");
    assert_eq!(m["start_url"], "/");
    assert_eq!(m["theme_color"], "#161617");
    assert_eq!(m["icons"][2]["purpose"], "maskable");
    assert_eq!(m["icons"][0]["src"], "icon-192.png");
}

#[test]
fn rendering_is_stable() {
    assert_eq!(render(GLYPH, &colors(), &app()), render(GLYPH, &colors(), &app()));
}

#[test]
fn a_broken_glyph_says_so() {
    let e = render("<svg", &colors(), &app()).expect_err("broken");
    assert!(e.0.starts_with("The glyph is not an SVG"), "{e}");
    assert!(render("not svg at all", &colors(), &app()).is_err());
}

#[test]
fn an_unknown_theme_says_so() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tokens.json");
    let e = Colors::from_theme(&Tokens::load(&root).expect("tokens"), "neon").expect_err("unknown");
    assert_eq!(e.0, "The tokens have no theme called \"neon\".");
}

#[test]
fn head_points_at_the_icon_folder() {
    let files = render(GLYPH, &colors(), &app()).expect("render");
    let text = String::from_utf8(files[7].1.clone()).expect("utf8");
    assert!(text.contains(r#"<link rel="manifest" href="/icons/manifest.webmanifest">"#), "{text}");
    let files = render(GLYPH, &colors(), &App { base: Some("/app-icons".into()), ..app() }).expect("render");
    let text = String::from_utf8(files[7].1.clone()).expect("utf8");
    assert!(text.contains(r#"<link rel="icon" href="/app-icons/favicon.svg" type="image/svg+xml">"#), "{text}");
    assert!(text.contains(r##"<meta name="theme-color" content="#161617">"##), "{text}");
}
