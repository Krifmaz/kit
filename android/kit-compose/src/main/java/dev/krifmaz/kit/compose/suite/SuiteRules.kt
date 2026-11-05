package dev.krifmaz.kit.compose.suite

import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.core.strings
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive

/**
 * One entry of the shared sidebar (contracts/suite.md): a section of this app or of a linked app.
 *
 * @property key `<app id>/<section id>`.
 * @property app the app's id.
 * @property appName the app's name.
 * @property title the entry's words.
 * @property icon SVG path data in a 24×24 box.
 * @property url where it opens: the section's full URL.
 * @property path the section's path in its app.
 * @property hidden the person hid it (it shows only while they edit the sidebar).
 * @property local it belongs to the app showing the sidebar.
 */
data class SidebarItem(
    val key: String,
    val app: String = "",
    val appName: String = "",
    val title: String = "",
    val icon: String = "",
    val url: String = "",
    val path: String = "",
    val hidden: Boolean = false,
    val local: Boolean = false,
)

/** One hit of a search (a path or full URL in [url]). */
data class SearchHit(val title: String, val subtitle: String = "", val url: String = "")

/** One app's part of a search across apps: its hits, or why it could not be searched ([error]). */
data class SearchGroup(
    val app: String,
    val appName: String = "",
    val icon: String = "",
    val local: Boolean = false,
    val results: List<SearchHit> = emptyList(),
    val error: String = "",
)

/** The words of the suite's screens, the same as kit-rules' `SuiteWords`; `{app}` and `{query}` are filled in. */
data class SuiteWords(
    val edit: String = "Edit sidebar",
    val done: String = "Done",
    val hide: String = "Hide",
    val show: String = "Show",
    val moveUp: String = "Move up",
    val moveDown: String = "Move down",
    val reset: String = "Reset order",
    val searchPlaceholder: String = "Search every app",
    val noResults: String = "Nothing found for “{query}”.",
    val relatedIn: String = "Related in {app}",
    val signInWith: String = "Sign in with {app}",
)

/** The suite's rules, without Android (the unit tests run them on the JVM); the same as kit-rules' `suite`. */
object SuiteRules {
    /** The sidebar's path under the server's API (`GET` and `PUT`). */
    const val SIDEBAR_PATH = "/api/kit/suite/sidebar"

    /** The sign-on answer's path. */
    const val SSO_PATH = "/api/kit/sso"

    /** The entries of a sidebar answer (`{items, apps}`). */
    fun itemsOf(o: JsonObject?): List<SidebarItem> = o?.arr("items").objectsOf().map {
        SidebarItem(
            key = it.str("key"),
            app = it.str("app"),
            appName = it.str("app_name"),
            title = it.str("title"),
            icon = it.str("icon"),
            url = it.str("url"),
            path = it.str("path"),
            hidden = it.bool("hidden"),
            local = it.bool("local"),
        )
    }.filter { it.key.isNotEmpty() }

    /** The groups of a search answer (`{groups}`). */
    fun groupsOf(o: JsonObject?): List<SearchGroup> = o?.arr("groups").objectsOf().map { g ->
        SearchGroup(
            app = g.str("app"),
            appName = g.str("app_name"),
            icon = g.str("icon"),
            local = g.bool("local"),
            results = g.arr("results").objectsOf().map { SearchHit(it.str("title"), it.str("subtitle"), it.str("url")) },
            error = g.str("error"),
        )
    }

    /** The entries that show while the person is not editing. */
    fun visible(items: List<SidebarItem>): List<SidebarItem> = items.filter { !it.hidden }

    /** [items] with entry [key] moved one place up (`by = -1`) or down (`by = 1`); the same at either end. */
    fun move(items: List<SidebarItem>, key: String, by: Int): List<SidebarItem> {
        val i = items.indexOfFirst { it.key == key }
        val j = i + by
        if (i < 0 || j < 0 || j >= items.size) return items
        return items.toMutableList().also {
            val t = it[i]
            it[i] = it[j]
            it[j] = t
        }
    }

    /** [items] with entry [key] hidden, or shown again. */
    fun toggle(items: List<SidebarItem>, key: String): List<SidebarItem> = items.map { if (it.key == key) it.copy(hidden = !it.hidden) else it }

    /** The `PUT` body that keeps exactly this order and these hidden entries. */
    fun prefsOf(items: List<SidebarItem>): JsonObject = JsonObject(
        mapOf(
            "order" to JsonArray(items.map { JsonPrimitive(it.key) }),
            "hidden" to JsonArray(items.filter { it.hidden }.map { JsonPrimitive(it.key) }),
        ),
    )

    /** The order a `PUT` body names (for a demo server). */
    fun orderOf(prefs: JsonObject): List<String> = prefs.arr("order").strings()

    /** [s] percent-encoded for a query value (everything but `A-Z a-z 0-9 - . _ ~`). */
    fun encode(s: String): String = buildString {
        for (b in s.toByteArray(Charsets.UTF_8)) {
            val c = b.toInt() and 0xff
            val ch = c.toChar()
            if (ch in 'A'..'Z' || ch in 'a'..'z' || ch in '0'..'9' || ch == '-' || ch == '.' || ch == '_' || ch == '~') {
                append(ch)
            } else {
                append('%').append("0123456789ABCDEF"[c shr 4]).append("0123456789ABCDEF"[c and 15])
            }
        }
    }

    /** The path of a search across apps for [q]: `scope` `all`, `self` or `others`. */
    fun searchPath(q: String, scope: String = "all"): String = "/api/kit/suite/search?scope=$scope&q=${encode(q.trim())}"

    /** [template] with each `{name}` filled. */
    fun fill(template: String, vararg vars: Pair<String, String>): String = vars.fold(template) { t, (k, v) -> t.replace("{$k}", v) }
}
