package dev.krifmaz.kit.compose.theme

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test

/**
 * The kit's theme equals Encore's Android theme (clients/android/…/ui/theme/Theme.kt): the same palettes per theme,
 * the same accent maths (web-rs/src/appearance.rs) and the same theme ids.
 */
class KitThemeTest {
    @Test
    fun encorePaletteIsTheLogoColours() {
        val p = KitPalettes.Encore
        assertEquals(Color(0xFF161617), p.bg)
        assertEquals(Color(0xFF1C1C1E), p.bg2)
        assertEquals(Color(0xFF222224), p.panel)
        assertEquals(Color(0xFF2A2A2D), p.panel2)
        assertEquals(Color(0xFF353538), p.border)
        assertEquals(Color(0xFFF0EAD6), p.text)
        assertEquals(Color(0xFFAAA493), p.muted)
        assertEquals(Color(0xFF77725F), p.faint)
        assertEquals(Color(0xFFF0EAD6), p.accent)
        assertEquals(Color(0xFFD6C49A), p.accent2)
        assertEquals(Color(0xFF161617), p.onAccent)
    }

    @Test
    fun themesResolveLikeEncore() {
        assertSame(KitPalettes.Encore, basePaletteOf(KitTheme.Encore, systemDark = false))
        assertSame(KitPalettes.Default, basePaletteOf(KitTheme.Dark, systemDark = false))
        assertSame(KitPalettes.Oled, basePaletteOf(KitTheme.Oled, systemDark = true))
        assertSame(KitPalettes.Light, basePaletteOf(KitTheme.Light, systemDark = true))
        assertSame(KitPalettes.Default, basePaletteOf(KitTheme.System, systemDark = true))
        assertSame(KitPalettes.Light, basePaletteOf(KitTheme.System, systemDark = false))
        assertEquals(KitTheme.Encore, KitTheme.of("nonsense"))
        assertEquals(KitTheme.Oled, KitTheme.of("oled"))
        // the logo accent keeps the theme's own colours (charcoal on Light)
        assertSame(KitPalettes.Light, kitPaletteOf(KitLook(theme = KitTheme.Light), systemDark = false))
    }

    @Test
    fun appearanceColours() {
        assertEquals("#ff7a00", AppearanceRules.cleanHex("FF7A00"))
        assertEquals("#aabbcc", AppearanceRules.cleanHex("#abc"))
        assertNull(AppearanceRules.cleanHex("#abcd"))
        assertNull(AppearanceRules.cleanHex("orange"))
        // the same partner colour as web-rs/src/appearance.rs
        assertEquals("#ebff00", AppearanceRules.partner("#ff7a00"))
        assertEquals("#e5e5e5", AppearanceRules.partner("#e5e5e5"))
        assertEquals("#e75cf6", AppearanceRules.partner("#8b5cf6"))
        assertEquals("#8b5cf6" to "#ec4899", AppearanceRules.accentHex("violet", "#000000"))
        assertEquals("#ff7a00" to "#ebff00", AppearanceRules.accentHex("custom", "#ff7a00"))
    }
}
