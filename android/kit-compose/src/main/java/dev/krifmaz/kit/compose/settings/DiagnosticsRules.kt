package dev.krifmaz.kit.compose.settings

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.obj
import dev.krifmaz.kit.compose.core.str
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject

/**
 * The words and reading rules of Settings → Diagnostics (the same rules as kit-rules'
 * `diagnostics`): the check groups, the plain-text report of Copy report, and the app crash
 * reports' kinds and text. Plain Kotlin, checked by JUnit tests on its own.
 *
 * The answer it reads is the server's diagnostics (contracts/diagnostics.md): `version`, `uid`, `time`, `checks`
 * (`{name, kind, ok, request, status, ms, detail, body}`), optional `machine`, and `settings`
 * (`{key, source, env, value}`).
 */
object DiagnosticsRules {
    /** The check groups in their order: (kind, heading). */
    val CHECK_KINDS: List<Pair<String, String>> = listOf("service" to "Connections", "path" to "Folders & permissions", "tool" to "Tools in the container")

    /** App crash report kinds, the web's words. */
    fun crashKind(k: String): String = when (k) {
        "crash" -> "Crash"
        "native" -> "Native crash"
        "anr" -> "Not responding"
        "low_memory" -> "Closed for memory"
        "signaled" -> "Killed by the system"
        "web" -> "Web page"
        "" -> "Crash"
        else -> k
    }

    /**
     * An app crash as text, for its dialog and Copy: the first line, the kind, device /
     * app / os / user and the time, then the whole report.
     */
    fun crashText(first: String, kind: String, meta: String, time: String, text: String): String {
        val head = listOf(first, kind, meta, time).filter { it.isNotEmpty() }.joinToString("\n")
        return if (text.isBlank()) head else head + "\n\n" + text.trimEnd()
    }

    /**
     * The plain-text report of Copy report: the header, every check, the app's [extra] lines (e.g. the folders
     * it writes to), the Server block ([serverRows] of `machine`, when the answer has one) and where each setting comes from.
     *
     * @param last the diagnostics answer.
     * @param extra lines between the checks and the Server block.
     * @param serverRows the Server block's (label, value) rows for `machine`.
     * @param appName the app's name in the header ("Notes 1.4.2 diagnostics (uid 1000)").
     */
    fun report(
        last: JsonObject,
        extra: List<String> = emptyList(),
        serverRows: (JsonObject) -> List<Pair<String, String>> = { emptyList() },
        appName: String = KitApp.name,
    ): String {
        val lines = arrayListOf("$appName ${jsStr(last, "version")} diagnostics (uid ${jsStr(last, "uid")})")
        for (e in last.arr("checks") ?: JsonArray(emptyList())) {
            val c = e as? JsonObject ?: continue
            val ok = c.bool("ok")
            val status = if (truthy(c["status"])) " → HTTP ${jsStr(c, "status")}" else ""
            val ms = if (!nullish(c, "ms")) " (${jsStr(c, "ms")} ms)" else ""
            val body = if (truthy(c["body"]) && !ok) "\n     body: ${c.str("body").take(300)}" else ""
            lines.add("${if (ok) "OK  " else "FAIL"} ${jsStr(c, "name")} – ${c.str("request")}$status$ms\n     ${c.str("detail")}$body")
        }
        lines.addAll(extra)
        last.obj("machine")?.let { m ->
            lines.add("")
            lines.add("Server:")
            for ((k, v) in serverRows(m)) lines.add("  $k: $v")
        }
        lines.add("")
        lines.add("Settings:")
        for (e in last.arr("settings") ?: JsonArray(emptyList())) {
            val s = e as? JsonObject ?: continue
            lines.add("  ${jsStr(s, "key")} = ${jsStr(s, "value")}  [${if (s.str("source") == "env") "env ${jsStr(s, "env")}" else "saved"}]")
        }
        return lines.joinToString("\n")
    }

    /** JavaScript truthiness of a JSON value. */
    fun truthy(v: JsonElement?): Boolean = SchemaRules.truthy(v)

    /** A value as text ("" for null). */
    fun text(v: JsonElement?): String = SchemaRules.textOf(v)

    /** A missing key or null (`== null` in JavaScript). */
    fun nullish(o: JsonObject, key: String): Boolean = o[key] == null || o[key] is JsonNull

    /** `String(obj[key])` as JavaScript prints it (null → "null", missing → "undefined"). */
    fun jsStr(o: JsonObject, key: String): String = when (val v = o[key]) {
        null -> "undefined"
        is JsonNull -> "null"
        else -> text(v)
    }
}
