package dev.krifmaz.kit.compose.theme

import androidx.compose.ui.graphics.Color
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test

/** The theme: the generated palettes per theme id, the System theme, the accents and the colour maths. */
class KitThemeTest {
    @Test
    fun theDefaultThemeIsCharcoalAndCream() {
        val p = KitPalettes.Encore
        assertEquals(Color(0xFF161617), p.bg)
        assertEquals(Color(0xFFF0EAD6), p.text)
        assertEquals(Color(0xFFF0EAD6), p.accent)
        assertEquals(Color(0xFFD6C49A), p.accent2)
        assertEquals(Color(0xFF16161F), p.onAccent)
        assertEquals(KitTheme.Encore, KitLook().theme)
    }

    @Test
    fun themesResolveByIdAndSystemFollowsTheDevice() {
        assertSame(KitPalettes.Encore, basePaletteOf(KitTheme.Encore, systemDark = false))
        assertSame(KitPalettes.Dark, basePaletteOf(KitTheme.Dark, systemDark = false))
        assertSame(KitPalettes.Oled, basePaletteOf(KitTheme.Oled, systemDark = true))
        assertSame(KitPalettes.Light, basePaletteOf(KitTheme.Light, systemDark = true))
        assertSame(KitPalettes.Dark, basePaletteOf(KitTheme.System, systemDark = true))
        assertSame(KitPalettes.Light, basePaletteOf(KitTheme.System, systemDark = false))
        assertEquals(KitTheme.Encore, KitTheme.of("nonsense"))
        assertEquals(KitTheme.Encore, KitTheme.of(null))
        assertEquals(listOf("encore", "dark", "oled", "light", "system"), KitTheme.entries.map { it.id })
    }

    @Test
    fun noAccentChosenKeepsTheThemesOwn() {
        for (t in KitTheme.entries) {
            assertSame(basePaletteOf(t, systemDark = true), kitPaletteOf(KitLook(theme = t), systemDark = true))
            assertSame(basePaletteOf(t, systemDark = true), kitPaletteOf(KitLook(theme = t, accent = "theme"), systemDark = true))
        }
    }

    @Test
    fun aChosenAccentAndTheFadedBackground() {
        val violet = kitPaletteOf(KitLook(theme = KitTheme.Encore, accent = "violet"), systemDark = true)
        assertEquals(Color(0xFF8B5CF6), violet.accent)
        assertEquals(Color(0xFFEC4899), violet.accent2)
        assertEquals(Color.White, violet.onAccent)
        assertEquals(KitPalettes.Encore.bg, violet.bg) // solid: the page colours stay
        val faded = kitPaletteOf(KitLook(theme = KitTheme.Encore, accent = "violet", background = "fade"), systemDark = true)
        assertNotEquals(KitPalettes.Encore.bg, faded.bg)
        val custom = kitPaletteOf(KitLook(theme = KitTheme.Light, accent = "custom", custom = "#ff7a00"), systemDark = false)
        assertEquals(Color(0xFFFF7A00), custom.accent)
        assertEquals(Color(0xFF16161F), custom.onAccent) // dark text on a light accent
        assertEquals(listOf("cream", "violet", "pastel", "lilac", "pink", "sky", "teal", "green", "amber", "red"), KitAppearance.ACCENTS.map { it.id })
    }

    @Test
    fun appearanceColours() {
        assertEquals("#ff7a00", AppearanceRules.cleanHex("FF7A00"))
        assertEquals("#aabbcc", AppearanceRules.cleanHex("#abc"))
        assertNull(AppearanceRules.cleanHex("#abcd"))
        assertNull(AppearanceRules.cleanHex("orange"))
        // the partner is shifted around the colour wheel (the same maths as kit-rules and kit-iced)
        assertEquals("#ebff00", AppearanceRules.partner("#ff7a00"))
        assertEquals("#e5e5e5", AppearanceRules.partner("#e5e5e5"))
        assertEquals("#e75cf6", AppearanceRules.partner("#8b5cf6"))
        assertEquals("#8b5cf6" to "#ec4899", AppearanceRules.accentHex("violet", "#000000"))
        assertEquals("#ff7a00" to "#ebff00", AppearanceRules.accentHex("custom", "#ff7a00"))
    }
}
