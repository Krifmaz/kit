package dev.krifmaz.kit.compose.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** The same cases as kit-leptos `ui` (shell, stats, art, media), so the web and Android agree. */
class DesignRulesTest {
    @Test
    fun aSectionIsActiveOnItselfAndUnderIt() {
        val lib = KitNavItem("#/library", "Library", "library")
        assertTrue(lib.isActive("#/library"))
        assertTrue(lib.isActive("#/library/3"))
        assertTrue(lib.isActive("#/library?sort=new"))
        assertFalse(lib.isActive("#/libraryx"))
        assertFalse(lib.isActive("#/"))
        val home = KitNavItem("#/", "Home", "house")
        assertTrue(home.isActive("#/") && home.isActive("") && home.isActive("#"))
        assertFalse(home.isActive("#/library"))
    }

    @Test
    fun moreThanFiveSectionsPutTheRestUnderMore() {
        assertEquals(4 to false, DesignRules.tabSplit(4, 5))
        assertEquals(5 to false, DesignRules.tabSplit(5, 5))
        assertEquals(4 to true, DesignRules.tabSplit(6, 5))
        assertEquals(3 to true, DesignRules.tabSplit(9, 4))
        assertEquals(3 to false, DesignRules.tabSplit(3, 9))
    }

    @Test
    fun changesReadAsPeopleWriteThem() {
        assertEquals("+12%" to "up", DesignRules.deltaText(0.12))
        assertEquals("−3%" to "down", DesignRules.deltaText(-0.034))
        assertEquals("0%" to "flat", DesignRules.deltaText(0.001))
    }

    @Test
    fun initialsAreTheFirstLettersOfTwoWords() {
        assertEquals("NL", DesignRules.initials("northern lights over the bay"))
        assertEquals("QH", DesignRules.initials("  (quiet) hours "))
        assertEquals("X", DesignRules.initials("x"))
        assertEquals("", DesignRules.initials(""))
    }

    @Test
    fun hueOfASeedIsTheWebs() {
        // the web: h = seed * 2654435761 (u32, wrapping); a = h % 360; b = (a + 40 + (h >> 9) % 80) % 360
        for (seed in listOf(0, 1, 7, 24, 1000, Int.MAX_VALUE)) {
            val h = (seed.toLong() * 2_654_435_761L) and 0xFFFFFFFFL
            val a = (h % 360).toInt()
            assertEquals(a to ((a + 40 + (h shr 9) % 80) % 360).toInt(), DesignRules.hues(seed))
        }
        assertEquals(DesignRules.hues(7), DesignRules.hues(7))
    }

    @Test
    fun progressIsKeptBetweenNothingAndAll() {
        assertEquals(0.0, DesignRules.unit(-1.0), 0.0)
        assertEquals(1.0, DesignRules.unit(2.0), 0.0)
        assertEquals(0.0, DesignRules.unit(Double.NaN), 0.0)
        assertEquals(0.25, DesignRules.unit(0.25), 0.0)
    }
}
