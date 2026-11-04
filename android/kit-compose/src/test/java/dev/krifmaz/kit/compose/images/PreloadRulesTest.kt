package dev.krifmaz.kit.compose.images

import org.junit.Assert.assertEquals
import org.junit.Test

class PreloadRulesTest {
    @Test
    fun threeScreensAheadInTheScrollDirection() {
        assertEquals(20 until 50, PreloadRules.ahead(first = 10, visible = 10, count = 100, dir = 1))
        assertEquals(20 until 50, PreloadRules.ahead(first = 50, visible = 10, count = 100, dir = -1))
    }

    @Test
    fun afterAJumpTwoBehindAndThreeAhead() {
        assertEquals(30 until 80, PreloadRules.ahead(first = 50, visible = 10, count = 100, dir = 0))
    }

    @Test
    fun staysInsideTheList() {
        assertEquals(95 until 100, PreloadRules.ahead(first = 90, visible = 5, count = 100, dir = 1))
        assertEquals(0 until 3, PreloadRules.ahead(first = 3, visible = 10, count = 100, dir = -1))
    }
}
