package dev.krifmaz.kit.compose.theme

import androidx.compose.foundation.LocalIndication
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp

/**
 * The themes of Settings → Appearance (the web's `data-theme`): Encore (charcoal and cream, the default), Dark (the
 * original blue-black), OLED black, Light, and System (Dark or Light, following the device).
 *
 * Also the name of the theme composable: `KitTheme(theme = KitTheme.Encore) { App() }` (DESIGN.md §15).
 *
 * @property id the id the web and the account prefs use ("encore", "dark", "oled", "light", "system").
 */
enum class KitTheme(val id: String) {
    /** Charcoal and cream, the logo's colours (`:root[data-theme="encore"]`). */
    Encore("encore"),

    /** The original blue-black dark theme (`:root`). */
    Dark("dark"),

    /** True black, so OLED pixels switch off (`:root[data-theme="oled"]`). */
    Oled("oled"),

    /** The light theme (`:root[data-theme="light"]`). */
    Light("light"),

    /** Dark or Light, following the device. */
    System("system"),
    ;

    /** Lookup by [id]. */
    companion object {
        /** The theme with this [id]; Encore for an unknown or missing one. */
        fun of(id: String?): KitTheme = entries.firstOrNull { it.id == id } ?: Encore
    }
}

/**
 * The account's look (the web's appearance keys, Settings → Appearance).
 *
 * @property theme the theme.
 * @property accent an accent id of [KitAppearance.ACCENTS] (logo, violet, encore, …) or "custom".
 * @property custom the custom accent colour, "#rrggbb" (used when [accent] is "custom").
 * @property background "solid", or "fade" (a faint wash of the accent in the page colours).
 */
data class KitLook(val theme: KitTheme = KitTheme.Encore, val accent: String = "logo", val custom: String = KitAppearance.DEFAULT_CUSTOM, val background: String = "solid")

/** The colour maths of Settings → Appearance (web-rs/src/appearance.rs), shared by every app. */
object AppearanceRules {
    /** "#abc" / "abc" / "#aabbcc" as "#aabbcc"; null when it is not a hex colour (the web's /^#([0-9a-f]{3}|[0-9a-f]{6})$/i). */
    fun cleanHex(input: String): String? {
        var v = input.trim()
        if (!v.startsWith("#")) v = "#$v"
        val d = v.substring(1)
        if ((d.length != 3 && d.length != 6) || !d.all { it.isDigit() || it.lowercaseChar() in 'a'..'f' }) return null
        return if (d.length == 3) "#" + d.flatMap { listOf(it, it) }.joinToString("").lowercase() else v.lowercase()
    }

    private fun rgb(hex: String): Triple<Int, Int, Int> {
        val h = hex.trimStart('#')
        val full = if (h.length == 3) h.flatMap { listOf(it, it) }.joinToString("") else h
        val n = full.toIntOrNull(16) ?: 0
        return Triple((n shr 16) and 255, (n shr 8) and 255, n and 255)
    }

    /** A "#rrggbb" (or "#rgb") colour as a Compose [Color]; black when it is not hex. */
    fun colorOf(hex: String): Color {
        val (r, g, b) = rgb(hex)
        return Color(r, g, b)
    }

    private fun luminance(hex: String): Double {
        val (r, g, b) = rgb(hex)

        fun lin(v: Int): Double {
            val x = v / 255.0
            return if (x <= 0.03928) x / 12.92 else Math.pow((x + 0.055) / 1.055, 2.4)
        }
        return lin(r) * 0.2126 + lin(g) * 0.7152 + lin(b) * 0.0722
    }

    /** A custom colour gets a partner shifted around the colour wheel for the gradient (appearance.rs partner). */
    fun partner(hex: String): String {
        val (ri, gi, bi) = rgb(hex)
        val r = ri / 255.0
        val g = gi / 255.0
        val b = bi / 255.0
        val max = maxOf(r, g, b)
        val min = minOf(r, g, b)
        val l = (max + min) / 2
        var h = 0.0
        var s = 0.0
        if (max != min) {
            val d = max - min
            s = if (l > 0.5) d / (2 - max - min) else d / (max + min)
            h =
                when (max) {
                    r -> (g - b) / d + if (g < b) 6 else 0
                    g -> (b - r) / d + 2
                    else -> (r - g) / d + 4
                }
            h /= 6
        }
        h = (h + 0.1) % 1.0
        val q = if (l < 0.5) l * (1 + s) else l + s - l * s
        val p = 2 * l - q

        fun f(t0: Double): Double {
            val t = ((t0 + 1) % 1.0)
            return when {
                t < 1.0 / 6 -> p + (q - p) * 6 * t
                t < 0.5 -> q
                t < 2.0 / 3 -> p + (q - p) * (2.0 / 3 - t) * 6
                else -> p
            }
        }

        fun c(v: Double): Int = Math.round(v * 255).toInt().coerceIn(0, 255)
        return "#%02x%02x%02x".format(c(f(h + 1.0 / 3)), c(f(h)), c(f(h - 1.0 / 3)))
    }

    /** The accent's two colours as hex (a preset's, or the custom colour and its partner). */
    fun accentHex(accent: String, custom: String): Pair<String, String> {
        val preset = KitAppearance.ACCENTS.firstOrNull { it.id == accent }
        if (preset != null) return preset.a to preset.b
        val a = cleanHex(custom) ?: KitAppearance.DEFAULT_CUSTOM
        return a to partner(a)
    }

    /** Dark text on light accents, white on dark ones (appearance.rs apply: `--on-accent`). */
    fun onAccent(a: String, b: String): Color = if ((luminance(a) + luminance(b)) / 2 > 0.45) Color(0xFF16161F) else Color.White
}

/** The palette of [theme] alone (System resolves to Dark or Light by [systemDark]). */
fun basePaletteOf(theme: KitTheme, systemDark: Boolean): KitPalette = when (theme) {
    KitTheme.Encore -> KitPalettes.Encore
    KitTheme.Dark -> KitPalettes.Default
    KitTheme.Oled -> KitPalettes.Oled
    KitTheme.Light -> KitPalettes.Light
    KitTheme.System -> if (systemDark) KitPalettes.Default else KitPalettes.Light
}

/**
 * The palette for a look: the theme's tokens, the accent's colours (the logo's cream and gold keep the theme's own
 * accent, charcoal on Light) and, for the faded background, a faint wash of the accent in the page colours.
 */
fun kitPaletteOf(look: KitLook, systemDark: Boolean): KitPalette {
    val base = basePaletteOf(look.theme, systemDark)
    val accented =
        if (look.accent == "logo") {
            base
        } else {
            val (a, b) = AppearanceRules.accentHex(look.accent, look.custom)
            base.copy(accent = AppearanceRules.colorOf(a), accent2 = AppearanceRules.colorOf(b), onAccent = AppearanceRules.onAccent(a, b))
        }
    if (look.background != "fade") return accented
    return accented.copy(
        bg = lerp(accented.bg, accented.accent, if (accented.dark) 0.06f else 0.05f),
        bg2 = lerp(accented.bg2, accented.accent, if (accented.dark) 0.04f else 0.03f),
    )
}

/** Material 3's colour scheme for a palette: the palette's tokens in Material's roles (Encore's mapping). */
fun kitColorScheme(p: KitPalette): ColorScheme {
    val base = if (p.dark) darkColorScheme() else lightColorScheme()
    return base.copy(
        primary = p.accent,
        onPrimary = p.onAccent,
        primaryContainer = p.panel2,
        onPrimaryContainer = p.text,
        secondary = p.accent2,
        onSecondary = p.onAccent,
        secondaryContainer = p.panel2,
        onSecondaryContainer = p.text,
        tertiary = p.accent2,
        onTertiary = p.onAccent,
        background = p.bg,
        onBackground = p.text,
        surface = p.bg2,
        onSurface = p.text,
        surfaceVariant = p.panel,
        onSurfaceVariant = p.muted,
        surfaceContainerLowest = p.bg,
        surfaceContainerLow = p.bg2,
        surfaceContainer = p.panel,
        surfaceContainerHigh = p.panel2,
        surfaceContainerHighest = p.border,
        surfaceTint = Color.Transparent,
        outline = p.border,
        outlineVariant = p.faint,
        error = KitStatusColors.Err,
        onError = Color.White,
    )
}

/** The current palette (the web's CSS variables); [KitTheme] provides it. */
val LocalKitPalette = staticCompositionLocalOf { KitPalettes.Encore }

/** The current theme's web tokens (`MaterialTheme.colorScheme` carries the same colours for Material widgets). */
val MaterialTheme.palette: KitPalette
    @Composable
    @ReadOnlyComposable
    get() = LocalKitPalette.current

/**
 * The theme for a palette: [palette] as [LocalKitPalette], Material 3 with the kit's colours, type and shapes, the
 * focus ring on every clickable (for TV remotes and keyboards) and a root [Surface] for the content colour.
 *
 * @param palette the colours, usually from [kitPaletteOf].
 * @param content the app's UI.
 */
@Composable
fun KitTheme(palette: KitPalette, content: @Composable () -> Unit) {
    val scheme = remember(palette) { kitColorScheme(palette) }
    CompositionLocalProvider(LocalKitPalette provides palette) {
        // A Surface at the root sets the content colour for every Text (without it, Compose's default is black)
        MaterialTheme(colorScheme = scheme, typography = KitTypography, shapes = KitShapes) {
            // every clickable also outlines itself while a D-pad / keyboard focuses it (FocusRing.kt);
            // LocalIndication here is MaterialTheme's ripple, which the ring keeps underneath
            val ripple = LocalIndication.current
            val indication = remember(ripple, palette.accent) { FocusRingIndication(ripple, palette.accent) }
            CompositionLocalProvider(LocalIndication provides indication) {
                Surface(color = scheme.background, contentColor = scheme.onBackground, content = content)
            }
        }
    }
}

/**
 * The theme for a look: `KitTheme(theme = KitTheme.Encore) { App() }`.
 *
 * @param theme the theme ([KitTheme.System] follows the device).
 * @param accent the accent id, or "custom" with [custom].
 * @param custom the custom accent colour.
 * @param background "solid" or "fade".
 * @param content the app's UI.
 */
@Composable
fun KitTheme(
    theme: KitTheme = KitTheme.Encore,
    accent: String = "logo",
    custom: String = KitAppearance.DEFAULT_CUSTOM,
    background: String = "solid",
    content: @Composable () -> Unit,
) {
    val systemDark = isSystemInDarkTheme()
    val look = KitLook(theme, accent, custom, background)
    val palette = remember(look, systemDark) { kitPaletteOf(look, systemDark) }
    KitTheme(palette, content)
}
