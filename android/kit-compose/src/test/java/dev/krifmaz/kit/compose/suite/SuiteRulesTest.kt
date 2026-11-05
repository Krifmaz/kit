package dev.krifmaz.kit.compose.suite

import dev.krifmaz.kit.compose.core.KitJson
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/** Suite: reading the sidebar and search answers, moving and hiding entries, the search path. */
class SuiteRulesTest {
    private fun o(s: String) = KitJson.parseToJsonElement(s).jsonObject

    private val answer = o(
        """{"items":[
          {"key":"me/a","app":"me","app_name":"Me","title":"A","icon":"M0 0","url":"https://me.test/a","path":"/a","local":true},
          {"key":"other/b","app":"other","app_name":"Other","title":"B","url":"https://other.test/b","path":"/b"},
          {"key":"other/c","app":"other","app_name":"Other","title":"C","hidden":true},
          {"title":"no key is left out"}]}""",
    )

    @Test
    fun theSidebarReadsAndMoves() {
        val items = SuiteRules.itemsOf(answer)
        assertEquals(listOf("me/a", "other/b", "other/c"), items.map { it.key })
        assertTrue(items[0].local)
        assertEquals("Other", items[1].appName)
        assertEquals(listOf("me/a", "other/b"), SuiteRules.visible(items).map { it.key })
        val moved = SuiteRules.toggle(SuiteRules.move(SuiteRules.move(items, "other/c", -1), "me/a", -1), "other/c")
        assertEquals(listOf("me/a", "other/c", "other/b"), moved.map { it.key })
        val prefs = SuiteRules.prefsOf(moved)
        assertEquals("""{"order":["me/a","other/c","other/b"],"hidden":[]}""", prefs.toString())
        assertEquals(listOf("me/a", "other/c", "other/b"), SuiteRules.orderOf(prefs))
        assertEquals(items, SuiteRules.move(items, "nope", 1))
        assertTrue(SuiteRules.itemsOf(null).isEmpty())
    }

    @Test
    fun searchGroupsRead() {
        val g = SuiteRules.groupsOf(
            o(
                """{"groups":[
                  {"app":"x","app_name":"X","local":true,"results":[{"title":"T","subtitle":"S","url":"https://x.test/1"}]},
                  {"app":"y","app_name":"Y","results":[],"error":"Y can't be reached."}]}""",
            ),
        )
        assertEquals(2, g.size)
        assertEquals("https://x.test/1", g[0].results[0].url)
        assertEquals("Y can't be reached.", g[1].error)
        assertEquals("/api/kit/suite/search?scope=others&q=a%20b%C3%A9", SuiteRules.searchPath(" a bé ", "others"))
        assertEquals("Related in Y", SuiteRules.fill(SuiteWords().relatedIn, "app" to "Y"))
    }
}
