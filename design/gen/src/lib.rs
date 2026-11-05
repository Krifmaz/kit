//! The kit's design tokens and the generator that turns them into every platform's theme.
//!
//! `design/tokens.json` defines the look once: the four themes (`encore`, the default; `dark`, `oled`, `light`), the
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
//! The tests check each output against the tokens and that `design/out/` is up to date.

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
    /// The themes, the default (`encore`, `:root`) first.
    pub themes: Vec<Theme>,
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
    /// Layout sizes (`--content`, `--sidebar`, `--player`, `--player-h`; `--player` is the room an app keeps for a
    /// bar along the bottom, 0 when it has none).
    pub layout: Layout,
    /// Corner radii by use.
    pub radius: BTreeMap<String, String>,
    /// The web's type scale.
    pub typography: Typography,
    /// The spacing steps in px.
    pub spacing: Vec<u32>,
    /// Durations and keyframes.
    pub motion: Motion,
    /// The design system's scales: spacing, type, touch target, bars, breakpoints, elevations.
    pub scale: Scale,
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

/// One theme's colours.
#[derive(Clone, Debug, Deserialize)]
pub struct Theme {
    /// `encore` (the default), `dark`, `oled` or `light`.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The CSS selector of its block.
    pub css_selector: String,
    /// The comment at the top of its CSS block.
    pub note: Option<String>,
    /// A dark theme (`color-scheme: dark`).
    pub dark: bool,
    /// `bg`, `bg-2`, `panel`, `panel-2`, `border`, `text`, `muted`, `faint`, `accent`, `accent-2`, `on-accent`.
    pub colors: BTreeMap<String, String>,
    /// `--shadow`.
    pub shadow: String,
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
    /// `--player`: the room kept under the page for a bar along the bottom (0 for none).
    pub player: String,
    /// `--player-h`: that bar's height when it shows.
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

/// The design system's scales, the same on every client.
#[derive(Clone, Debug, Deserialize)]
pub struct Scale {
    /// Spacing steps in px by name (`--space-1` = 4px …).
    pub space: BTreeMap<String, u32>,
    /// The type scale in px by name (`--text-xs` … `--text-3xl`).
    pub text: BTreeMap<String, u32>,
    /// The smallest touch target in px (`--touch`).
    pub touch: u32,
    /// The phone's top app bar height in px (`--app-bar`).
    pub app_bar: u32,
    /// The phone's bottom tab bar height in px, without the safe area (`--tab-bar`).
    pub tab_bar: u32,
    /// The tablet's navigation rail width in px (`--rail`).
    pub rail: u32,
    /// Below this width in px an app is laid out for phones (tab bar, app bar, sheets).
    pub breakpoint: u32,
    /// From this width in px the full sidebar shows instead of the rail.
    pub wide: u32,
    /// `--shadow-1` … `--shadow-3` on dark themes.
    pub elevation: BTreeMap<String, String>,
    /// `--shadow-1` … `--shadow-3` on light themes.
    pub elevation_light: BTreeMap<String, String>,
    /// The blur of the glass bars in px (`--glass-blur`).
    pub glass_blur: u32,
    /// No animation runs longer than this, in ms.
    pub max_motion_ms: u32,
}

/// Android specifics: Material shapes and type roles.
#[derive(Clone, Debug, Deserialize)]
pub struct Compose {
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

/// A theme's own accent colours, in every output's order.
pub const ACCENT: [&str; 3] = ["accent", "accent-2", "on-accent"];

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

    /// Every theme has every colour, every colour is a hex colour, the default theme comes first, and every
    /// accent a preset names exists.
    fn check(&self) -> Result<(), Error> {
        if self.themes.first().map(|t| t.id.as_str()) != Some("encore") {
            return Err(Error("The first theme must be \"encore\" (the default, the :root block).".into()));
        }
        for t in &self.themes {
            for k in SURFACE.iter().chain(ACCENT.iter()) {
                let v = t.colors.get(*k).ok_or_else(|| Error(format!("Theme {} has no colour {k}.", t.id)))?;
                hex6(v).ok_or_else(|| Error(format!("Theme {}: {k} = {v} is not a hex colour.", t.id)))?;
            }
        }
        for a in &self.accents {
            for c in [&a.a, &a.b] {
                hex6(c).ok_or_else(|| Error(format!("Accent {}: {c} is not a hex colour.", a.id)))?;
            }
        }
        for p in &self.presets {
            if p.accent != "custom" && self.accent(&p.accent).is_none() {
                return Err(Error(format!("Preset {} names a missing accent {}.", p.id, p.accent)));
            }
            if p.theme != "system" && self.theme(&p.theme).is_none() {
                return Err(Error(format!("Preset {} names a missing theme {}.", p.id, p.theme)));
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

    /// The text colour on an accent made of `a` and `b`: dark on light accents, light on dark ones.
    pub fn on_accent_for(&self, a: &str, b: &str) -> &str {
        if (luminance(a) + luminance(b)) / 2.0 > self.on_accent.threshold {
            &self.on_accent.dark
        } else {
            &self.on_accent.light
        }
    }

    /// One theme's colours as `#rrggbb`, by token name.
    pub fn palette(&self, theme: &Theme) -> BTreeMap<String, String> {
        theme.colors.iter().map(|(k, v)| (k.clone(), hex6(v).unwrap_or_default())).collect()
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

/// The CSS variables: the default theme's `:root` block (with the status colours, radius and layout sizes)
/// and one `:root[data-theme="…"]` block per other theme with what differs from `:root`.
pub fn css(t: &Tokens) -> String {
    let mut o = String::from("/* Generated by design/gen from design/tokens.json. Do not edit by hand. */\n");
    let root = &t.themes[0];
    let accent_vars = |o: &mut String, th: &Theme, only_changed: bool| {
        let (a, b, on) = (&th.colors["accent"], &th.colors["accent-2"], &th.colors["on-accent"]);
        if only_changed && a == &root.colors["accent"] && b == &root.colors["accent-2"] && on == &root.colors["on-accent"] {
            return;
        }
        let _ = writeln!(o, "  --accent: {a};");
        let _ = writeln!(o, "  --accent-2: {b};");
        let _ = writeln!(o, "  --accent-rgb: {};", rgb_list(a));
        let _ = writeln!(o, "  --accent-2-rgb: {};", rgb_list(b));
        let _ = writeln!(o, "  --on-accent: {on};");
    };
    let _ = writeln!(o, "{} {{", root.css_selector);
    if let Some(n) = &root.note {
        let _ = writeln!(o, "  /* {n} */");
    }
    for k in SURFACE {
        let _ = writeln!(o, "  --{k}: {};", root.colors[k]);
    }
    accent_vars(&mut o, root, false);
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
    scale_vars(&mut o, t, root.dark);
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
        accent_vars(&mut o, th, true);
        if th.shadow != root.shadow {
            let _ = writeln!(o, "  --shadow: {};", th.shadow);
        }
        if th.dark != root.dark {
            for (k, v) in if th.dark { &t.scale.elevation } else { &t.scale.elevation_light } {
                let _ = writeln!(o, "  --shadow-{k}: {v};");
            }
            let _ = writeln!(o, "  color-scheme: {};", if th.dark { "dark" } else { "light" });
        }
        o.push_str("}\n");
    }
    o
}

/// The scale's variables for `:root`: spacing, type, sizes, elevations (of a dark or light default), blur and
/// motion.
fn scale_vars(o: &mut String, t: &Tokens, dark: bool) {
    let sc = &t.scale;
    let mut space: Vec<(&String, &u32)> = sc.space.iter().collect();
    space.sort_by_key(|(_, v)| **v);
    for (k, v) in space {
        let _ = writeln!(o, "  --space-{k}: {v}px;");
    }
    let mut text: Vec<(&String, &u32)> = sc.text.iter().collect();
    text.sort_by_key(|(_, v)| **v);
    for (k, v) in text {
        let _ = writeln!(o, "  --text-{k}: {v}px;");
    }
    let _ = writeln!(o, "  --touch: {}px;", sc.touch);
    let _ = writeln!(o, "  --app-bar: {}px;", sc.app_bar);
    let _ = writeln!(o, "  --tab-bar: {}px;", sc.tab_bar);
    let _ = writeln!(o, "  --rail: {}px;", sc.rail);
    for (k, v) in if dark { &sc.elevation } else { &sc.elevation_light } {
        let _ = writeln!(o, "  --shadow-{k}: {v};");
    }
    let _ = writeln!(o, "  --glass-blur: {}px;", sc.glass_blur);
    let _ = writeln!(o, "  --font: {};", t.typography.font_family);
    let _ = writeln!(o, "  --mono: {};", t.typography.mono_family);
    let _ = writeln!(o, "  --dur-fast: {};", t.motion.fast);
    let _ = writeln!(o, "  --dur-base: {};", t.motion.base);
    let _ = writeln!(o, "  --dur-pop: {};", t.motion.pop);
    o.push_str("  --ease: cubic-bezier(.2, .8, .2, 1);\n");
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
/// `dev.krifmaz.kit.compose.theme`.
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
        let p = t.palette(th);
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
    o.push_str(")\n\n");
    o.push_str("/** The spacing scale (`--space-N`). */\nobject KitSpace {\n");
    let mut space: Vec<(&String, &u32)> = t.scale.space.iter().collect();
    space.sort_by_key(|(_, v)| **v);
    for (k, v) in space {
        let _ = writeln!(o, "    val S{k} = {v}.dp");
    }
    o.push_str(
        "}\n\n/** The sizes of the design system: touch target, phone bars, rail and the phone/wide breakpoints. */\nobject KitSizes {\n",
    );
    let sc = &t.scale;
    for (n, v) in [
        ("Touch", sc.touch),
        ("AppBar", sc.app_bar),
        ("TabBar", sc.tab_bar),
        ("Rail", sc.rail),
        ("Breakpoint", sc.breakpoint),
        ("Wide", sc.wide),
    ] {
        let _ = writeln!(o, "    val {n} = {v}.dp");
    }
    o.push_str(
        "}\n\n/** Durations in ms (`--dur-fast`, `--dur-base`, `--dur-pop`); nothing runs longer than `MAX`. */\nobject KitMotion {\n",
    );
    let ms = |s: &str| (s.trim_end_matches('s').parse::<f64>().unwrap_or(0.0) * 1000.0).round() as u32;
    for (n, v) in [("FAST", ms(&t.motion.fast)), ("BASE", ms(&t.motion.base)), ("POP", ms(&t.motion.pop)), ("MAX", sc.max_motion_ms)] {
        let _ = writeln!(o, "    const val {n} = {v}");
    }
    o.push_str("}\n\nprivate val Sans = FontFamily.SansSerif\n\n");
    o.push_str("/** The web's type scale as Material roles. */\nval KitTypography: Typography = Typography(\n");
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
/// colours, each theme's accent and the status colours.
pub fn iced(t: &Tokens) -> String {
    let mut o = String::new();
    o.push_str("// Generated by design/gen from design/tokens.json. Do not edit by hand.\n");
    o.push_str("// kit-iced includes this file; the colours are `#rrggbb` strings its `hex` turns into colours.\n\n");
    let _ = writeln!(o, "/// `--radius`.\npub const RADIUS: f32 = {:?};", t.iced.radius);
    let _ = writeln!(o, "/// `--sidebar`.\npub const SIDEBAR: f32 = {:?};", t.iced.sidebar);
    let _ = writeln!(o, "/// `--player-h`.\npub const PLAYER_H: f32 = {:?};", t.iced.player_h);
    let mut space: Vec<u32> = t.scale.space.values().copied().collect();
    space.sort_unstable();
    let list: Vec<String> = space.iter().map(|v| format!("{:?}", *v as f32)).collect();
    let _ = writeln!(
        o,
        "/// The spacing scale (`--space-N`), smallest first.\npub const SPACE: [f32; {}] = [{}];",
        list.len(),
        list.join(", ")
    );
    let _ = writeln!(o, "/// `--touch`: the smallest touch target.\npub const TOUCH: f32 = {:?};\n", t.scale.touch as f32);
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
    o.push_str("/// `(bg, bg-2, panel, panel-2, border, text, muted, faint)` of a theme id (`encore`, `dark`, `oled`,\n/// `light`); any other id is `encore`, the default.\n");
    o.push_str("pub fn surface(theme: &str) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {\n    match theme {\n");
    let arm = |th: &Theme| {
        let c: Vec<String> = SURFACE.iter().map(|k| format!("\"{}\"", hex6(&th.colors[*k]).unwrap_or_default())).collect();
        format!("({})", c.join(", "))
    };
    for th in &t.themes[1..] {
        let _ = writeln!(o, "        \"{}\" => {},", th.id, arm(th));
    }
    let _ = writeln!(o, "        _ => {},\n    }}\n}}\n", arm(&t.themes[0]));
    o.push_str("/// `(accent, accent-2, on-accent)` of a theme id; any other id is `encore`, the default.\n");
    o.push_str("pub fn accent(theme: &str) -> (&'static str, &'static str, &'static str) {\n    match theme {\n");
    let acc = |th: &Theme| {
        let c: Vec<String> = ACCENT.iter().map(|k| format!("\"{}\"", hex6(&th.colors[*k]).unwrap_or_default())).collect();
        format!("({})", c.join(", "))
    };
    for th in &t.themes[1..] {
        let _ = writeln!(o, "        \"{}\" => {},", th.id, acc(th));
    }
    let _ = writeln!(o, "        _ => {},\n    }}\n}}", acc(&t.themes[0]));
    o
}

/// The Flutter theme: Dart palettes and a `ThemeData` per theme (for Flutter apps).
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
    o.push_str("/// The palettes, by theme, each with its own accent.\nclass KitPalettes {\n  KitPalettes._();\n\n");
    for th in &t.themes {
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
        let _ = writeln!(
            o,
            "    accent: {},\n    accent2: {},\n    onAccent: {},",
            dc(&th.colors["accent"]),
            dc(&th.colors["accent-2"]),
            dc(&th.colors["on-accent"])
        );
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
    o.push_str("/// Material `ThemeData` for a palette: the kit's colours, radii and type sizes.\n");
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
