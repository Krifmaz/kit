//! kit-icons: one app glyph in, the app's whole icon set out, in the kit's look.
//!
//! An app draws only its glyph: a small SVG on a 24 × 24 grid (any `viewBox` works), painted with
//! `currentColor` and, where a second colour helps, `{accent-2}`. The kit puts it on its tile (the theme's
//! panel colours, rounded like every kit card) and renders every size browsers and phones ask for, so every
//! app's icon has the same shape, colours and margins.
//!
//! | File | What it is |
//! | --- | --- |
//! | `favicon.svg` | the tile as SVG, for the browser tab |
//! | `icon-32.png` | the tile at 32 px, for browsers without SVG favicons |
//! | `icon-192.png`, `icon-512.png` | the tile, for the web manifest (installed app, home screen) |
//! | `icon-maskable-512.png` | full-bleed square with the glyph inside the maskable safe zone (Android crops it) |
//! | `apple-touch-icon.png` | 180 px, opaque, square (iOS rounds it itself) |
//! | `manifest.webmanifest` | name, colours and the icons above |
//! | `head.html` | the `<link>` and `<meta>` tags to paste into the app's `index.html` |
//!
//! [`render`] makes them in memory; the `kit-icons` binary writes them to a folder.

use std::fmt;

use kit_tokens::Tokens;
use resvg::{tiny_skia, usvg};

/// How the glyph sits on the tile, in units of a 512 px tile.
const TILE: f64 = 512.0;
/// The tile's corner radius: the same share of the side as the kit's large card radius.
const RADIUS: f64 = 112.0;
/// The glyph's box on the rounded tile (favicon, PNGs, Apple touch icon): 60 % of the side.
const GLYPH: f64 = 0.60;
/// The glyph's box on the maskable icon: 50 % of the side, inside the 80 % safe circle.
const GLYPH_MASKABLE: f64 = 0.50;

/// The colours of one icon set, taken from a kit theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colors {
    /// The tile's top colour (the theme's `panel-2`).
    pub tile_top: String,
    /// The tile's bottom colour, also the manifest's background and theme colour (the theme's `bg`).
    pub tile_bottom: String,
    /// The tile's hairline edge (the theme's `border`).
    pub edge: String,
    /// The glyph's `currentColor` (the theme's `text`).
    pub glyph: String,
    /// The glyph's `{accent-2}` (the theme's `accent-2`).
    pub accent: String,
}

impl Colors {
    /// The colours of theme `id` (`encore` is the kit's default).
    pub fn from_theme(tokens: &Tokens, id: &str) -> Result<Colors, Error> {
        let theme = tokens.theme(id).ok_or_else(|| Error(format!("The tokens have no theme called \"{id}\".")))?;
        let get = |key: &str| theme.colors.get(key).cloned().ok_or_else(|| Error(format!("Theme \"{id}\" has no \"{key}\" colour.")));
        Ok(Colors {
            tile_top: get("panel-2")?,
            tile_bottom: get("bg")?,
            edge: get("border")?,
            glyph: get("text")?,
            accent: get("accent-2")?,
        })
    }
}

/// What the app tells the manifest about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// The app's name, as people see it ("Kit Demo").
    pub name: String,
    /// A short name for home screens; the name when it is not given.
    pub short_name: Option<String>,
    /// Where the installed app opens; `/` when it is not given.
    pub start_url: Option<String>,
}

/// Something that stopped the icon set from being made, in plain words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// One file of the icon set: its name and its bytes.
pub type File = (&'static str, Vec<u8>);

/// Every file of the icon set for `glyph` (the glyph's SVG source), in the order of the table above.
pub fn render(glyph: &str, colors: &Colors, app: &App) -> Result<Vec<File>, Error> {
    let glyph = Glyph::parse(glyph, colors)?;
    let rounded = tile_svg(&glyph, colors, Shape::Rounded);
    let mut files: Vec<File> = vec![("favicon.svg", rounded.clone().into_bytes())];
    files.push(("icon-32.png", png(&rounded, 32)?));
    files.push(("icon-192.png", png(&rounded, 192)?));
    files.push(("icon-512.png", png(&rounded, 512)?));
    files.push(("icon-maskable-512.png", png(&tile_svg(&glyph, colors, Shape::Maskable), 512)?));
    files.push(("apple-touch-icon.png", png(&tile_svg(&glyph, colors, Shape::Square), 180)?));
    files.push(("manifest.webmanifest", manifest(colors, app).into_bytes()));
    files.push(("head.html", head(colors).into_bytes()));
    Ok(files)
}

/// The glyph, checked and with its colours filled in.
struct Glyph {
    /// The glyph's `viewBox`.
    view_box: String,
    /// What is inside its `<svg>` element.
    body: String,
}

impl Glyph {
    fn parse(source: &str, colors: &Colors) -> Result<Glyph, Error> {
        // it must be an SVG usvg can read on its own, so a broken glyph fails here and not in a browser
        usvg::Tree::from_str(&fill(source, colors), &usvg::Options::default())
            .map_err(|e| Error(format!("The glyph is not an SVG that can be drawn: {e}.")))?;
        let open = source.find("<svg").ok_or_else(|| Error("The glyph has no <svg> element.".into()))?;
        let open_end = source[open..].find('>').map(|i| open + i).ok_or_else(|| Error("The glyph's <svg> tag is not closed.".into()))?;
        let close = source.rfind("</svg>").ok_or_else(|| Error("The glyph has no closing </svg>.".into()))?;
        if close <= open_end {
            return Err(Error("The glyph's <svg> element is empty.".into()));
        }
        let tag = &source[open..open_end];
        let view_box = attribute(tag, "viewBox").unwrap_or_else(|| "0 0 24 24".into());
        Ok(Glyph { view_box, body: fill(source[open_end + 1..close].trim(), colors) })
    }
}

/// `name="…"` from one tag, if it is there.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let len = tag[start..].find('"')?;
    Some(tag[start..start + len].to_string())
}

/// Puts the theme's colours in for `{accent-2}` and `currentColor`.
fn fill(svg: &str, colors: &Colors) -> String {
    svg.replace("{accent-2}", &colors.accent).replace("currentColor", &colors.glyph)
}

/// The three ways the tile is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// Rounded corners, transparent outside them.
    Rounded,
    /// A full square (iOS rounds it).
    Square,
    /// A full square with a smaller glyph (Android crops it to its own shape).
    Maskable,
}

fn tile_svg(glyph: &Glyph, colors: &Colors, shape: Shape) -> String {
    let (rx, edge) = match shape {
        Shape::Rounded => (
            RADIUS,
            format!(
                r#"<rect x="4" y="4" width="504" height="504" rx="{r}" fill="none" stroke="{c}" stroke-width="8"/>"#,
                r = RADIUS - 4.0,
                c = colors.edge
            ),
        ),
        Shape::Square | Shape::Maskable => (0.0, String::new()),
    };
    let share = if shape == Shape::Maskable { GLYPH_MASKABLE } else { GLYPH };
    let size = TILE * share;
    let at = (TILE - size) / 2.0;
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">"#,
            r#"<defs><linearGradient id="kit-tile" x1="0" y1="0" x2="0" y2="1">"#,
            r#"<stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient></defs>"#,
            r#"<rect width="512" height="512" rx="{rx}" fill="url(#kit-tile)"/>{edge}"#,
            r#"<svg x="{at}" y="{at}" width="{size}" height="{size}" viewBox="{vb}" color="{glyph}">{body}</svg>"#,
            "</svg>\n"
        ),
        top = colors.tile_top,
        bottom = colors.tile_bottom,
        rx = rx,
        edge = edge,
        at = at,
        size = size,
        vb = glyph.view_box,
        glyph = colors.glyph,
        body = glyph.body,
    )
}

fn png(svg: &str, px: u32) -> Result<Vec<u8>, Error> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| Error(format!("The icon could not be drawn: {e}.")))?;
    let mut pixmap = tiny_skia::Pixmap::new(px, px).ok_or_else(|| Error(format!("No room for a {px} px icon.")))?;
    let scale = px as f32 / tree.size().width();
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| Error(format!("The {px} px icon could not be saved as PNG: {e}.")))
}

fn manifest(colors: &Colors, app: &App) -> String {
    let icon =
        |src: &str, size: &str, purpose: &str| serde_json::json!({ "src": src, "sizes": size, "type": "image/png", "purpose": purpose });
    let value = serde_json::json!({
        "name": app.name,
        "short_name": app.short_name.as_deref().unwrap_or(&app.name),
        "start_url": app.start_url.as_deref().unwrap_or("/"),
        "display": "standalone",
        "background_color": colors.tile_bottom,
        "theme_color": colors.tile_bottom,
        "icons": [
            icon("icon-192.png", "192x192", "any"),
            icon("icon-512.png", "512x512", "any"),
            icon("icon-maskable-512.png", "512x512", "maskable"),
        ],
    });
    // serde_json keeps keys sorted; that is stable, which is what --check needs
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_default();
    text.push('\n');
    text
}

fn head(colors: &Colors) -> String {
    format!(
        concat!(
            "<!-- the kit's icon set (kit-icons): served from /icons/ -->\n",
            "<link rel=\"icon\" href=\"/icons/favicon.svg\" type=\"image/svg+xml\">\n",
            "<link rel=\"icon\" href=\"/icons/icon-32.png\" sizes=\"32x32\" type=\"image/png\">\n",
            "<link rel=\"apple-touch-icon\" href=\"/icons/apple-touch-icon.png\">\n",
            "<link rel=\"manifest\" href=\"/icons/manifest.webmanifest\">\n",
            "<meta name=\"theme-color\" content=\"{}\">\n"
        ),
        colors.tile_bottom
    )
}

#[cfg(test)]
mod tests;
