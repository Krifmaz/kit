//! Encore's design tokens and the generator that turns them into every platform's theme.
//!
//! `design/tokens.json` defines Encore's look once: the four themes (default, encore, oled, light), the
//! status colours, the accents and presets of Settings → Appearance, radii, type, spacing and motion. This
//! crate reads it ([`Tokens::load`] / [`Tokens::parse`]) and renders four outputs:
//!
//! | Output | Function | File in `design/out/` |
//! | --- | --- | --- |
//! | CSS variables | [`css`] | `tokens.css` |
//! | Compose theme (Kotlin) | [`compose`] | `KitTokens.kt` |
//! | Iced theme (Rust) | [`iced`] | `iced_tokens.rs` |
//! | Flutter ThemeData (Dart) | [`flutter`] | `kit_tokens.dart` |
//!
//! The tests compare the output with Encore's current theme sources (copies in `design/fixtures/encore`),
//! so moving Encore onto the tokens changes nothing visually.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// The whole `tokens.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct Tokens {
    /// The file format's version (1).
    pub version: u32,
    /// `--ok`, `--warn`, `--err`, `--info`: the same in every theme.
    pub status: Status,
    /// The themes, the default (`:root`) first.
    pub themes: Vec<Theme>,
    /// The accent `:root` wears before the app applies the account's look.
    pub default_accent: String,
    /// The signed-out look (`:root[data-brand]`).
    pub brand: Brand,
    /// How the text colour on an accent is chosen.
    pub on_accent: OnAccent,
    /// Settings → Appearance accents, in their order.
    pub accents: Vec<Accent>,
    /// The custom accent's colour before one is picked.
    pub default_custom: String,
    /// Settings → Appearance presets, in their order.
    pub presets: Vec<Preset>,
    /// The theme picker's entries, in their order.
    pub theme_picker: Vec<Choice>,
    /// The faded background's mix amounts.
    pub fade: Fade,
    /// Layout sizes (`--content`, `--sidebar`, `--player`, `--player-h`).
    pub layout: Layout,
    /// Corner radii by use.
    pub radius: BTreeMap<String, String>,
    /// The web's type scale.
    pub typography: Typography,
    /// The spacing steps in px.
    pub spacing: Vec<u32>,
    /// Durations and keyframes.
    pub motion: Motion,
    /// Android specifics.
    pub compose: Compose,
    /// Desktop specifics.
    pub iced: Iced,
}

/// The status colours.
#[derive(Clone, Debug, Deserialize)]
pub struct Status {
    /// success
    pub ok: String,
    /// warning
    pub warn: String,
    /// error
    pub err: String,
    /// information
    pub info: String,
}

/// One theme's surface colours.
#[derive(Clone, Debug, Deserialize)]
pub struct Theme {
    /// `default`, `encore`, `oled` or `light`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The CSS selector of its block.
    pub css_selector: String,
    /// The comment at the top of its CSS block.
    pub note: Option<String>,
    /// A dark theme (`color-scheme: dark`).
    pub dark: bool,
    /// `bg`, `bg-2`, `panel`, `panel-2`, `border`, `text`, `muted`, `faint`.
    pub colors: BTreeMap<String, String>,
    /// `--shadow`.
    pub shadow: String,
}

/// The signed-out look.
#[derive(Clone, Debug, Deserialize)]
pub struct Brand {
    /// The accent id it wears.
    pub accent: String,
    /// Its CSS selector.
    pub css_selector: String,
    /// The comment at the top of its CSS block.
    pub note: String,
}

/// Dark or light text on an accent.
#[derive(Clone, Debug, Deserialize)]
pub struct OnAccent {
    /// Above this mean luminance of the two accent colours the text is dark.
    pub threshold: f64,
    /// The dark text colour.
    pub dark: String,
    /// The light text colour.
    pub light: String,
}

/// One accent: the two colours of its gradient.
#[derive(Clone, Debug, Deserialize)]
pub struct Accent {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The first colour.
    pub a: String,
    /// The second colour.
    pub b: String,
}

/// One preset: a theme, an accent and a background.
#[derive(Clone, Debug, Deserialize)]
pub struct Preset {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The theme picker id.
    pub theme: String,
    /// The accent id, or `custom`.
    pub accent: String,
    /// The custom colour for a `custom` accent.
    pub custom: Option<String>,
    /// `solid` or `fade`.
    pub background: String,
}

/// An entry of a picker.
#[derive(Clone, Debug, Deserialize)]
pub struct Choice {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
}

/// The faded background's mix amounts.
#[derive(Clone, Debug, Deserialize)]
pub struct Fade {
    /// `bg` on dark themes.
    pub bg_dark: f64,
    /// `bg` on the light theme.
    pub bg_light: f64,
    /// `bg-2` on dark themes.
    pub bg2_dark: f64,
    /// `bg-2` on the light theme.
    pub bg2_light: f64,
}

/// Layout sizes.
#[derive(Clone, Debug, Deserialize)]
pub struct Layout {
    /// `--content`.
    pub content: String,
    /// The comment after `--content`.
    pub content_note: String,
    /// `--sidebar`.
    pub sidebar: String,
    /// `--player`.
    pub player: String,
    /// `--player-h`.
    #[serde(rename = "player-h")]
    pub player_h: String,
}

/// The web's type scale.
#[derive(Clone, Debug, Deserialize)]
pub struct Typography {
    /// The body font stack.
    pub font_family: String,
    /// The monospace stack.
    pub mono_family: String,
    /// Body text.
    pub body: TypeStep,
    /// `h1`.
    pub h1: TypeStep,
    /// `h2`.
    pub h2: TypeStep,
    /// `h3`.
    pub h3: TypeStep,
    /// `.small`.
    pub small: TypeStep,
}

/// One step of the type scale.
#[derive(Clone, Debug, Deserialize)]
pub struct TypeStep {
    /// Size in px.
    pub size: u32,
    /// Weight, when set.
    pub weight: Option<u32>,
    /// Line height as a factor, when set.
    pub line_height: Option<f64>,
    /// Letter spacing in em, when set.
    pub letter_spacing_em: Option<f64>,
}

/// Durations and keyframes.
#[derive(Clone, Debug, Deserialize)]
pub struct Motion {
    /// Hover and small changes.
    pub fast: String,
    /// The usual transition.
    pub base: String,
    /// Toasts and dialogs popping in.
    pub pop: String,
    /// Slow fades.
    pub slow: String,
    /// Keyframes by name.
    pub keyframes: BTreeMap<String, String>,
}

/// Android specifics: where Encore's Compose theme differs from the web's tokens today.
#[derive(Clone, Debug, Deserialize)]
pub struct Compose {
    /// The Kotlin name of each theme's palette in Encore's Theme.kt.
    pub palette_names: BTreeMap<String, String>,
    /// Per theme: colours that differ on Android.
    pub overrides: BTreeMap<String, BTreeMap<String, String>>,
    /// Material shape sizes in dp.
    pub shapes: BTreeMap<String, u32>,
    /// The Material type scale.
    pub typography: Vec<ComposeType>,
}

/// One Material type role.
#[derive(Clone, Debug, Deserialize)]
pub struct ComposeType {
    /// The role (`titleLarge` …).
    pub role: String,
    /// Size in sp.
    pub size: f64,
    /// Line height in sp.
    pub line: f64,
    /// `Bold`, `SemiBold` or `Normal`.
    pub weight: String,
    /// Letter spacing in sp.
    pub tracking: f64,
}

/// Desktop specifics.
#[derive(Clone, Debug, Deserialize)]
pub struct Iced {
    /// The desktop app's id for each theme.
    pub theme_ids: BTreeMap<String, String>,
    /// `RADIUS`.
    pub radius: f32,
    /// `SIDEBAR`.
    pub sidebar: f32,
    /// `PLAYER_H`.
    pub player_h: f32,
}

/// A token file that could not be read, with the reason in plain words.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// The order of the surface colours in every output.
pub const SURFACE: [&str; 8] = ["bg", "bg-2", "panel", "panel-2", "border", "text", "muted", "faint"];

impl Tokens {
    /// Read and check a `tokens.json`.
    pub fn load(path: &Path) -> Result<Tokens, Error> {
        let text = std::fs::read_to_string(path).map_err(|e| Error(format!("Could not read {}: {e}", path.display())))?;
        Tokens::parse(&text)
    }

    /// Parse and check the text of a `tokens.json`.
    pub fn parse(text: &str) -> Result<Tokens, Error> {
        let t: Tokens = serde_json::from_str(text).map_err(|e| Error(format!("tokens.json does not parse: {e}")))?;
        t.check()?;
        Ok(t)
    }

    /// Every theme has every surface colour, every colour is a hex colour, and every id a preset or the brand
    /// names exists.
    fn check(&self) -> Result<(), Error> {
        if self.themes.first().map(|t| t.id.as_str()) != Some("default") {
            return Err(Error("The first theme must be \"default\" (the :root block).".into()));
        }
        for t in &self.themes {
            for k in SURFACE {
                let v = t.colors.get(k).ok_or_else(|| Error(format!("Theme {} has no colour {k}.", t.id)))?;
                hex6(v).ok_or_else(|| Error(format!("Theme {}: {k} = {v} is not a hex colour.", t.id)))?;
            }
        }
        for a in &self.accents {
            for c in [&a.a, &a.b] {
                hex6(c).ok_or_else(|| Error(format!("Accent {}: {c} is not a hex colour.", a.id)))?;
            }
        }
        for id in [&self.default_accent, &self.brand.accent] {
            self.accent(id).ok_or_else(|| Error(format!("No accent {id}.")))?;
        }
        for p in &self.presets {
            if p.accent != "custom" && self.accent(&p.accent).is_none() {
                return Err(Error(format!("Preset {} names a missing accent {}.", p.id, p.accent)));
            }
        }
        Ok(())
    }

    /// The theme with this id.
    pub fn theme(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    /// The accent with this id.
    pub fn accent(&self, id: &str) -> Option<&Accent> {
        self.accents.iter().find(|a| a.id == id)
    }

    /// The text colour on an accent made of `a` and `b` (appearance.rs `apply`).
    pub fn on_accent_for(&self, a: &str, b: &str) -> &str {
        if (luminance(a) + luminance(b)) / 2.0 > self.on_accent.threshold {
            &self.on_accent.dark
        } else {
            &self.on_accent.light
        }
    }

    /// One theme's colours as Android draws them: the surface colours, then the Compose overrides.
    pub fn compose_palette(&self, theme: &Theme) -> BTreeMap<String, String> {
        let mut m: BTreeMap<String, String> = theme.colors.iter().map(|(k, v)| (k.clone(), hex6(v).unwrap_or_default())).collect();
        if let Some(o) = self.compose.overrides.get(&theme.id) {
            for (k, v) in o {
                m.insert(k.clone(), hex6(v).unwrap_or_default());
            }
        }
        m
    }
}

/// `#abc`, `abc` or `#aabbcc` as `#aabbcc` (lower case); `None` when it is not a hex colour.
pub fn hex6(s: &str) -> Option<String> {
    let d = s.trim().trim_start_matches('#');
    if !(d.len() == 3 || d.len() == 6) || !d.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let d = d.to_ascii_lowercase();
    Some(if d.len() == 3 { format!("#{}", d.chars().flat_map(|c| [c, c]).collect::<String>()) } else { format!("#{d}") })
}

/// The red, green and blue of a hex colour (black when it is not one).
pub fn rgb(s: &str) -> (u8, u8, u8) {
    let h = hex6(s).unwrap_or_else(|| "#000000".into());
    let n = u32::from_str_radix(&h[1..], 16).unwrap_or(0);
    ((n >> 16) as u8, (n >> 8) as u8, n as u8)
}

/// Relative luminance (WCAG), as appearance.rs computes it.
pub fn luminance(s: &str) -> f64 {
    let (r, g, b) = rgb(s);
    let lin = |v: u8| {
        let x = f64::from(v) / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    lin(r) * 0.2126 + lin(g) * 0.7152 + lin(b) * 0.0722
}

/// `r, g, b` for a CSS `--accent-rgb` variable.
fn rgb_list(s: &str) -> String {
    let (r, g, b) = rgb(s);
    format!("{r}, {g}, {b}")
}

/// The CSS variables: Encore's `:root` block and one block per theme (only what differs from `:root`), with
/// the brand block after the `encore` theme, as in Encore's app.css.
pub fn css(t: &Tokens) -> String {
    let mut o = String::from("/* Generated by design/gen from design/tokens.json. Do not edit by hand. */\n");
    let root = &t.themes[0];
    let acc = t.accent(&t.default_accent).expect("checked");
    o.push_str(":root {\n");
    for k in SURFACE {
        let _ = writeln!(o, "  --{k}: {};", root.colors[k]);
    }
    let _ = writeln!(o, "  --accent: {};", acc.a);
    let _ = writeln!(o, "  --accent-2: {};", acc.b);
    let _ = writeln!(o, "  --accent-rgb: {};", rgb_list(&acc.a));
    let _ = writeln!(o, "  --accent-2-rgb: {};", rgb_list(&acc.b));
    let _ = writeln!(o, "  --on-accent: {};", t.on_accent_for(&acc.a, &acc.b));
    o.push_str("  --grad: linear-gradient(135deg, var(--accent), var(--accent-2));\n");
    let _ = writeln!(o, "  --content: {};   /* {} */", t.layout.content, t.layout.content_note);
    let _ = writeln!(o, "  --ok: {};", t.status.ok);
    let _ = writeln!(o, "  --warn: {};", t.status.warn);
    let _ = writeln!(o, "  --err: {};", t.status.err);
    let _ = writeln!(o, "  --info: {};", t.status.info);
    let _ = writeln!(o, "  --radius: {};", t.radius.get("card").map(String::as_str).unwrap_or("14px"));
    let _ = writeln!(o, "  --shadow: {};", root.shadow);
    let _ = writeln!(o, "  --sidebar: {};", t.layout.sidebar);
    let _ = writeln!(o, "  --player: {};", t.layout.player);
    let _ = writeln!(o, "  --player-h: {};", t.layout.player_h);
    let _ = writeln!(o, "  color-scheme: {};", if root.dark { "dark" } else { "light" });
    o.push_str("}\n");
    for th in &t.themes[1..] {
        let _ = writeln!(o, "{} {{", th.css_selector);
        if let Some(n) = &th.note {
            let _ = writeln!(o, "  /* {n} */");
        }
        for k in SURFACE {
            if th.colors[k] != root.colors[k] {
                let _ = writeln!(o, "  --{k}: {};", th.colors[k]);
            }
        }
        if th.shadow != root.shadow {
            let _ = writeln!(o, "  --shadow: {};", th.shadow);
        }
        if th.dark != root.dark {
            let _ = writeln!(o, "  color-scheme: {};", if th.dark { "dark" } else { "light" });
        }
        o.push_str("}\n");
        if th.id == "encore" {
            let b = t.accent(&t.brand.accent).expect("checked");
            let _ = writeln!(o, "{} {{", t.brand.css_selector);
            let _ = writeln!(o, "  /* {} */", t.brand.note);
            let _ = writeln!(o, "  --accent: {};", b.a);
            let _ = writeln!(o, "  --accent-2: {};", b.b);
            let _ = writeln!(o, "  --accent-rgb: {};", rgb_list(&b.a));
            let _ = writeln!(o, "  --accent-2-rgb: {};", rgb_list(&b.b));
            let _ = writeln!(o, "  --on-accent: {};", t.on_accent_for(&b.a, &b.b));
            o.push_str("}\n");
        }
    }
    o
}

/// `#aabbcc` as Kotlin `Color(0xFFAABBCC)`.
fn kt_color(s: &str) -> String {
    format!("Color(0xFF{})", hex6(s).unwrap_or_default()[1..].to_ascii_uppercase())
}

/// A Kotlin/Dart float literal (`-0.5`, `0`, `0.2`).
fn num(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// The Compose theme: Kotlin palettes, accents, presets, shapes and type scale in package
/// `dev.krifmaz.kit.compose.theme`, equal to Encore's current `Theme.kt` and `Type.kt`.
pub fn compose(t: &Tokens) -> String {
    let mut o = String::new();
    o.push_str("// Generated by design/gen from design/tokens.json. Do not edit by hand.\n");
    o.push_str("package dev.krifmaz.kit.compose.theme\n\n");
    o.push_str("import androidx.compose.foundation.shape.RoundedCornerShape\n");
    o.push_str("import androidx.compose.material3.Shapes\n");
    o.push_str("import androidx.compose.material3.Typography\n");
    o.push_str("import androidx.compose.runtime.Immutable\n");
    o.push_str("import androidx.compose.ui.graphics.Brush\n");
    o.push_str("import androidx.compose.ui.graphics.Color\n");
    o.push_str("import androidx.compose.ui.text.TextStyle\n");
    o.push_str("import androidx.compose.ui.text.font.FontFamily\n");
    o.push_str("import androidx.compose.ui.text.font.FontWeight\n");
    o.push_str("import androidx.compose.ui.unit.dp\n");
    o.push_str("import androidx.compose.ui.unit.sp\n\n");
    o.push_str("/** The status colours (`--ok`, `--warn`, `--err`, `--info`): the same in every theme. */\n");
    o.push_str("object KitStatusColors {\n");
    for (n, v) in [("Ok", &t.status.ok), ("Warn", &t.status.warn), ("Err", &t.status.err), ("Info", &t.status.info)] {
        let _ = writeln!(o, "    val {n} = {}", kt_color(v));
    }
    o.push_str("}\n\n");
    o.push_str("/** One theme's colours (the web's CSS variables of that theme). */\n");
    o.push_str("@Immutable\ndata class KitPalette(\n");
    for f in ["bg", "bg2", "panel", "panel2", "border", "text", "muted", "faint", "accent", "accent2", "onAccent"] {
        let _ = writeln!(o, "    val {f}: Color,");
    }
    o.push_str("    val dark: Boolean,\n) {\n");
    o.push_str("    /** `--grad`: accent into accent-2. */\n");
    o.push_str("    val gradient: Brush get() = Brush.linearGradient(listOf(accent, accent2))\n}\n\n");
    o.push_str("/** The palettes, by theme. */\nobject KitPalettes {\n");
    for th in &t.themes {
        let p = t.compose_palette(th);
        let name = camel(&th.id);
        let _ = writeln!(o, "    /** The {} theme ({}). */", th.name, th.css_selector.replace('"', "'"));
        let _ = writeln!(o, "    val {name}: KitPalette = KitPalette(");
        for (f, k) in [
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
        ] {
            let _ = writeln!(o, "        {f} = {},", kt_color(&p[k]));
        }
        let _ = writeln!(o, "        dark = {},\n    )", th.dark);
    }
    o.push_str("}\n\n");
    o.push_str("/** One accent of Settings → Appearance: the two colours of its gradient. */\n");
    o.push_str("data class KitAccent(val id: String, val name: String, val a: String, val b: String)\n\n");
    o.push_str("/** One preset of Settings → Appearance: a theme, an accent and a background. */\n");
    o.push_str("data class KitPreset(val id: String, val name: String, val theme: String, val accent: String, val custom: String?, val background: String)\n\n");
    o.push_str("/** The accents and presets of Settings → Appearance, in their order. */\nobject KitAppearance {\n");
    o.push_str("    val ACCENTS: List<KitAccent> = listOf(\n");
    for a in &t.accents {
        let _ = writeln!(o, "        KitAccent(\"{}\", \"{}\", \"{}\", \"{}\"),", a.id, a.name, a.a, a.b);
    }
    o.push_str("    )\n\n    val PRESETS: List<KitPreset> = listOf(\n");
    for p in &t.presets {
        let custom = p.custom.as_ref().map(|c| format!("\"{c}\"")).unwrap_or_else(|| "null".into());
        let _ = writeln!(
            o,
            "        KitPreset(\"{}\", \"{}\", \"{}\", \"{}\", {custom}, \"{}\"),",
            p.id, p.name, p.theme, p.accent, p.background
        );
    }
    let _ = writeln!(o, "    )\n\n    const val DEFAULT_CUSTOM = \"{}\"\n}}\n", t.default_custom);
    o.push_str("/** Corner radii: cards, tiles and menus, buttons and inputs, thumbs, sheets. */\n");
    o.push_str("val KitShapes: Shapes = Shapes(\n");
    for k in ["extraSmall", "small", "medium", "large", "extraLarge"] {
        let _ = writeln!(o, "    {k} = RoundedCornerShape({}.dp),", t.compose.shapes.get(k).copied().unwrap_or(0));
    }
    o.push_str(")\n\nprivate val Sans = FontFamily.SansSerif\n\n");
    o.push_str("/** The web player's type scale as Material roles. */\nval KitTypography: Typography = Typography(\n");
    for r in &t.compose.typography {
        let tracking = if r.tracking < 0.0 { format!("({})", num(r.tracking)) } else { num(r.tracking) };
        let _ = writeln!(o, "    {} = TextStyle(fontFamily = Sans, fontSize = {}.sp, lineHeight = {}.sp, fontWeight = FontWeight.{}, letterSpacing = {tracking}.sp),",
            r.role, num(r.size), num(r.line), r.weight);
    }
    o.push_str(")\n");
    o
}

/// `oled` → `Oled`, `default` → `Default`.
fn camel(id: &str) -> String {
    let mut c = id.chars();
    c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
}

/// The Iced theme tokens: a Rust module with the theme picker, accents, presets, every theme's surface
/// colours and the status colours, equal to Encore's desktop `clients/desktop/src/theme.rs`.
pub fn iced(t: &Tokens) -> String {
    let mut o = String::new();
    o.push_str("// Generated by design/gen from design/tokens.json. Do not edit by hand.\n");
    o.push_str("// kit-iced includes this file; the colours are `#rrggbb` strings its `hex` turns into colours.\n\n");
    let _ = writeln!(o, "/// `--radius`.\npub const RADIUS: f32 = {:?};", t.iced.radius);
    let _ = writeln!(o, "/// `--sidebar`.\npub const SIDEBAR: f32 = {:?};", t.iced.sidebar);
    let _ = writeln!(o, "/// `--player-h`.\npub const PLAYER_H: f32 = {:?};\n", t.iced.player_h);
    let _ = writeln!(
        o,
        "/// `(id, name)` of the themes the Appearance page offers, in its order.\npub const THEMES: [(&str, &str); {}] = [",
        t.theme_picker.len()
    );
    for c in &t.theme_picker {
        let _ = writeln!(o, "    (\"{}\", \"{}\"),", c.id, c.name);
    }
    o.push_str("];\n\n");
    let _ = writeln!(o, "/// `(id, name, colour, second colour)`: the accents of Settings → Appearance.\npub const ACCENTS: [(&str, &str, &str, &str); {}] = [", t.accents.len());
    for a in &t.accents {
        let _ = writeln!(o, "    (\"{}\", \"{}\", \"{}\", \"{}\"),", a.id, a.name, a.a, a.b);
    }
    o.push_str("];\n\n");
    let _ = writeln!(o, "/// `(id, name, theme, accent, custom colour, background)`: the presets of Settings → Appearance.\npub const PRESETS: [(&str, &str, &str, &str, &str, &str); {}] = [", t.presets.len());
    for p in &t.presets {
        let _ = writeln!(
            o,
            "    (\"{}\", \"{}\", \"{}\", \"{}\", \"{}\", \"{}\"),",
            p.id,
            p.name,
            p.theme,
            p.accent,
            p.custom.as_deref().unwrap_or(""),
            p.background
        );
    }
    o.push_str("];\n\n");
    let _ =
        writeln!(o, "/// The custom accent's colour before one is picked.\npub const DEFAULT_CUSTOM: &str = \"{}\";\n", t.default_custom);
    o.push_str("/// `(ok, warn, err, info)`.\n");
    let _ = writeln!(
        o,
        "pub const STATUS: (&str, &str, &str, &str) = (\"{}\", \"{}\", \"{}\", \"{}\");\n",
        t.status.ok, t.status.warn, t.status.err, t.status.info
    );
    let _ = writeln!(o, "/// The dark text on light accents and the light text on dark ones, and the luminance between them.\npub const ON_ACCENT: (&str, &str, f32) = (\"{}\", \"{}\", {:?});\n",
        hex6(&t.on_accent.dark).unwrap_or_default(), hex6(&t.on_accent.light).unwrap_or_default(), t.on_accent.threshold as f32);
    o.push_str("/// `(bg, bg-2, panel, panel-2, border, text, muted, faint)` of a theme id (`encore`, `dark`, `oled`,\n/// `light`); any other id is `dark`.\n");
    o.push_str("pub fn surface(theme: &str) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {\n    match theme {\n");
    let arm = |th: &Theme| {
        let c: Vec<String> = SURFACE.iter().map(|k| format!("\"{}\"", hex6(&th.colors[*k]).unwrap_or_default())).collect();
        format!("({})", c.join(", "))
    };
    for th in &t.themes[1..] {
        let id = t.iced.theme_ids.get(&th.id).cloned().unwrap_or_else(|| th.id.clone());
        let _ = writeln!(o, "        \"{id}\" => {},", arm(th));
    }
    let _ = writeln!(o, "        _ => {},\n    }}\n}}", arm(&t.themes[0]));
    o
}

/// The Flutter theme: Dart palettes and a `ThemeData` per theme (for Photovault's Ente fork).
pub fn flutter(t: &Tokens) -> String {
    let dc = |s: &str| format!("Color(0xFF{})", hex6(s).unwrap_or_default()[1..].to_ascii_uppercase());
    let mut o = String::new();
    o.push_str("// Generated by design/gen from design/tokens.json. Do not edit by hand.\n");
    o.push_str("import 'package:flutter/material.dart';\n\n");
    o.push_str("/// One theme's colours (the web's CSS variables of that theme).\n");
    o.push_str("@immutable\nclass KitPalette extends ThemeExtension<KitPalette> {\n");
    let fields =
        ["bg", "bg2", "panel", "panel2", "border", "text", "muted", "faint", "accent", "accent2", "onAccent", "ok", "warn", "err", "info"];
    o.push_str("  const KitPalette({\n");
    for f in fields {
        let _ = writeln!(o, "    required this.{f},");
    }
    o.push_str("    required this.dark,\n  });\n\n");
    for f in fields {
        let _ = writeln!(o, "  /// `{f}`.\n  final Color {f};");
    }
    o.push_str("  /// A dark theme.\n  final bool dark;\n\n");
    o.push_str("  @override\n  KitPalette copyWith({bool? dark}) => KitPalette(\n");
    for f in fields {
        let _ = writeln!(o, "        {f}: {f},");
    }
    o.push_str("        dark: dark ?? this.dark,\n      );\n\n");
    o.push_str("  @override\n  KitPalette lerp(ThemeExtension<KitPalette>? other, double t) {\n    if (other is! KitPalette) return this;\n    return KitPalette(\n");
    for f in fields {
        let _ = writeln!(o, "      {f}: Color.lerp({f}, other.{f}, t)!,");
    }
    o.push_str("      dark: t < 0.5 ? dark : other.dark,\n    );\n  }\n}\n\n");
    o.push_str("/// The palettes, by theme, with the logo accent.\nclass KitPalettes {\n  KitPalettes._();\n\n");
    let logo = t.accent(&t.brand.accent).expect("checked");
    for th in &t.themes {
        let on = t.on_accent_for(&logo.a, &logo.b).to_string();
        let _ = writeln!(o, "  /// The {} theme.\n  static const KitPalette {} = KitPalette(", th.name, th.id);
        for (f, k) in [
            ("bg", "bg"),
            ("bg2", "bg-2"),
            ("panel", "panel"),
            ("panel2", "panel-2"),
            ("border", "border"),
            ("text", "text"),
            ("muted", "muted"),
            ("faint", "faint"),
        ] {
            let _ = writeln!(o, "    {f}: {},", dc(&th.colors[k]));
        }
        let _ = writeln!(o, "    accent: {},\n    accent2: {},\n    onAccent: {},", dc(&logo.a), dc(&logo.b), dc(&on));
        let _ = writeln!(
            o,
            "    ok: {},\n    warn: {},\n    err: {},\n    info: {},",
            dc(&t.status.ok),
            dc(&t.status.warn),
            dc(&t.status.err),
            dc(&t.status.info)
        );
        let _ = writeln!(o, "    dark: {},\n  );\n", th.dark);
    }
    o.push_str("}\n\n");
    o.push_str("/// Material `ThemeData` for a palette: Encore's colours, radii and type sizes.\n");
    o.push_str("ThemeData kitThemeData(KitPalette p) {\n");
    o.push_str("  final scheme = ColorScheme(\n    brightness: p.dark ? Brightness.dark : Brightness.light,\n");
    o.push_str("    primary: p.accent,\n    onPrimary: p.onAccent,\n    secondary: p.accent2,\n    onSecondary: p.onAccent,\n");
    o.push_str("    error: p.err,\n    onError: const Color(0xFFFFFFFF),\n    surface: p.bg2,\n    onSurface: p.text,\n  );\n");
    let card = t.radius.get("card").and_then(|r| r.trim_end_matches("px").parse::<u32>().ok()).unwrap_or(14);
    let input = t.radius.get("input").and_then(|r| r.trim_end_matches("px").parse::<u32>().ok()).unwrap_or(10);
    o.push_str("  return ThemeData(\n    useMaterial3: true,\n    colorScheme: scheme,\n    scaffoldBackgroundColor: p.bg,\n");
    let _ = writeln!(
        o,
        "    cardTheme: CardThemeData(color: p.panel, shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular({card}))),"
    );
    let _ = writeln!(
        o,
        "    inputDecorationTheme: InputDecorationTheme(border: OutlineInputBorder(borderRadius: BorderRadius.circular({input}))),"
    );
    let ty = &t.typography;
    let _ = writeln!(o, "    textTheme: const TextTheme(\n      headlineMedium: TextStyle(fontSize: {}, fontWeight: FontWeight.w700),\n      titleLarge: TextStyle(fontSize: {}, fontWeight: FontWeight.w700),\n      titleMedium: TextStyle(fontSize: {}, fontWeight: FontWeight.w600),\n      bodyMedium: TextStyle(fontSize: {}, height: {}),\n      bodySmall: TextStyle(fontSize: {}),\n    ),",
        ty.h1.size, ty.h2.size, ty.h3.size, ty.body.size, num(ty.body.line_height.unwrap_or(1.5)), ty.small.size);
    o.push_str("    extensions: [p],\n  );\n}\n");
    o
}

/// Every output with its file name in `design/out/`.
pub fn outputs(t: &Tokens) -> Vec<(&'static str, String)> {
    vec![("tokens.css", css(t)), ("KitTokens.kt", compose(t)), ("iced_tokens.rs", iced(t)), ("kit_tokens.dart", flutter(t))]
}

#[cfg(test)]
mod tests;
