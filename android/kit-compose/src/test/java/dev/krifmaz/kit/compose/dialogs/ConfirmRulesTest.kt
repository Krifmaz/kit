package dev.krifmaz.kit.compose.dialogs

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** When a confirm dialog's button works, and its words. */
class ConfirmRulesTest {
    @Test
    fun aPlainConfirmAlwaysWorks() {
        val c = Confirm("Delete this note?", "It can't be undone.", "Delete", danger = true)
        assertTrue(ConfirmRules.canConfirm(c, ""))
        assertEquals("Cancel", c.cancelLabel)
    }

    @Test
    fun typeToConfirmNeedsTheExactWord() {
        val c = Confirm("Delete the project?", okLabel = "Delete", danger = true, typeToConfirm = "Garden")
        assertFalse(ConfirmRules.canConfirm(c, ""))
        assertFalse(ConfirmRules.canConfirm(c, "garden")) // case counts
        assertFalse(ConfirmRules.canConfirm(c, "Gard"))
        assertTrue(ConfirmRules.canConfirm(c, "Garden"))
        assertTrue(ConfirmRules.canConfirm(c, "  Garden ")) // spaces around it don't
        assertEquals("Type \"Garden\" to confirm", ConfirmRules.typePrompt("Garden"))
    }
}
