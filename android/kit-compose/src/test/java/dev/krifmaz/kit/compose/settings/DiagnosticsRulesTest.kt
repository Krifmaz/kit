package dev.krifmaz.kit.compose.settings

import dev.krifmaz.kit.compose.core.KitJson
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** The words of Settings → Diagnostics: Copy report and the app crash reports. */
class DiagnosticsRulesTest {
    private fun obj(s: String) = KitJson.parseToJsonElement(s) as JsonObject

    @Test
    fun diagnosticsReport() {
        val d = obj(
            """{"version":"0.29.0","uid":1000,"checks":[{"kind":"service","name":"Mail server","ok":false,"request":"GET /api","status":401,"ms":12,"detail":"Unauthorized","body":"nope"}],
               "settings":[{"key":"base_url","source":"env","env":"APP_BASE_URL","value":"https://x"},{"key":"app_name","value":null}],
               "machine":{"cpus":4}}""",
        )
        val text = DiagnosticsRules.report(d, listOf("", "Data folders:"), { m -> listOf("CPUs" to m["cpus"].toString()) }, appName = "Notes")
        assertTrue(
            text.startsWith(
                "Notes 0.29.0 diagnostics (uid 1000)\nFAIL Mail server – GET /api → HTTP 401 (12 ms)\n     Unauthorized\n     body: nope\n\nData folders:\n\nServer:\n  CPUs: 4\n",
            ),
        )
        assertTrue(text.contains("  base_url = https://x  [env APP_BASE_URL]"))
        assertTrue(text.contains("  app_name = null  [saved]"))
        assertEquals("Killed by the system", DiagnosticsRules.crashKind("signaled"))
        assertEquals("Crash", DiagnosticsRules.crashKind(""))
        // an app crash as its dialog and Copy have it: what, where, when, then the whole report
        assertEquals(
            "java.lang.IllegalStateException: boom\nCrash\nPixel 8 · 0.30.0 · Android 16 · caz\n<time>\n\nat a.b.C.d(C.kt:1)\nat e.f.G.h(G.kt:2)",
            DiagnosticsRules.crashText(
                "java.lang.IllegalStateException: boom",
                "Crash",
                "Pixel 8 · 0.30.0 · Android 16 · caz",
                "<time>",
                "at a.b.C.d(C.kt:1)\nat e.f.G.h(G.kt:2)\n",
            ),
        )
        assertEquals("Crash\nNot responding\n<time>", DiagnosticsRules.crashText("Crash", "Not responding", "", "<time>", "  "))
    }

    @Test
    fun javascriptReading() {
        val o = obj("""{"a":null,"b":3}""")
        assertEquals("null", DiagnosticsRules.jsStr(o, "a"))
        assertEquals("undefined", DiagnosticsRules.jsStr(o, "c"))
        assertEquals("3", DiagnosticsRules.jsStr(o, "b"))
        assertTrue(DiagnosticsRules.nullish(o, "a") && DiagnosticsRules.nullish(o, "c"))
    }
}
