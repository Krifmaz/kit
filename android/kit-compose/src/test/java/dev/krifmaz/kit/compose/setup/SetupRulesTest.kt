package dev.krifmaz.kit.compose.setup

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The setup rules, the same cases as kit-rules' `setup` tests. */
class SetupRulesTest {
    private fun obj(s: String): JsonObject = Json.parseToJsonElement(s).jsonObject

    @Test
    fun theScreenAndItsForm() {
        val w = SetupWords()
        assertTrue(SetupRules.needed(obj("""{"needed":true}""")))
        assertFalse(SetupRules.needed(obj("""{"needed":false}""")))
        assertFalse(SetupRules.needed(obj("""{"detail":"Not found"}""")))
        assertFalse(SetupRules.needed(null))
        assertEquals(w.needCode, SetupRules.checkForm(" ", "ana", "password1", "password1"))
        assertEquals(w.needName, SetupRules.checkForm("ABCD", " ", "password1", "password1"))
        assertEquals("Use a password of at least 8 characters", SetupRules.checkForm("ABCD", "ana", "short", "short"))
        assertEquals(w.mismatch, SetupRules.checkForm("ABCD", "ana", "password1", "password2"))
        assertNull(SetupRules.checkForm("ABCD", "ana", "password1", "password1"))
        assertEquals(
            obj("""{"code":"abcd-efgh","username":"ana","password":" pw ","email":"a@b.co"}"""),
            SetupRules.body(" abcd-efgh ", " ana ", " pw ", " a@b.co "),
        )
    }
}
