package dev.krifmaz.kit.compose.notices

import dev.krifmaz.kit.compose.core.KitJson
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Notice banners: reading the answer, which banners show, Check again's toast and when to ask again. */
class NoticeRulesTest {
    private fun o(s: String) = KitJson.parseToJsonElement(s).jsonObject

    @Test
    fun theAnswerReads() {
        val list = NoticeRules.noticesOf(
            o(
                """{"notices":[
                  {"ok":false,"text":"The data folder is read-only – check the disk","checking":false,"id":"storage","severity":"err"},
                  {"ok":true,"text":"","id":"mail"},
                  {"text":"no ok field reads as fine","id":"x"}]}""",
            ),
        )
        assertEquals(listOf("storage", "mail", "x"), list.map { it.id })
        assertEquals(Severity.Err, list[0].severity)
        assertEquals(Severity.Warn, list[1].severity) // missing: warn
        assertTrue(list[2].ok)
        // one notice object on its own reads as one
        assertEquals(1, NoticeRules.noticesOf(o("""{"ok":false,"text":"t"}""")).size)
        assertTrue(NoticeRules.noticesOf(o("{}")).isEmpty())
        assertTrue(NoticeRules.noticesOf(null).isEmpty())
    }

    @Test
    fun aBannerShowsWhileANoticeFailsWithASentence() {
        assertEquals("Disk full", Notice(ok = false, text = "Disk full").banner)
        assertNull(Notice(ok = true, text = "Disk full").banner)
        assertNull(Notice(ok = false, text = "").banner)
        val all = listOf(Notice(ok = false, text = "a", id = "1"), Notice(ok = true, id = "2"), Notice(ok = false, text = "c", id = "3"))
        assertEquals(listOf("1", "3"), NoticeRules.banners(all, admin = true).map { it.id })
        assertTrue(NoticeRules.banners(all, admin = false).isEmpty())
    }

    @Test
    fun checkAgainSaysHowItWent() {
        val words = CheckWords("The folder still can't be written", "Still checking the folder", "The folder can be written")
        assertEquals("The folder still can't be written" to "warn", NoticeRules.checkToast(Notice(ok = false), words))
        assertEquals("Still checking the folder" to "warn", NoticeRules.checkToast(Notice(ok = true, checking = true), words))
        assertEquals("The folder can be written" to "ok", NoticeRules.checkToast(Notice(ok = true), words))
        assertEquals("Fixed – all good again" to "ok", NoticeRules.checkToast(Notice(ok = true)))
    }

    @Test
    fun aFailingNoticeIsAskedForAtEveryLookElseEveryMinute() {
        val poll = NoticePoll()
        assertEquals(listOf(false, false, false, true, false), (1..5).map { poll.tick(failing = false) })
        assertTrue(poll.tick(failing = true))
        assertEquals(60_000L, NoticeRules.TICK_MS * NoticeRules.QUIET_TICKS)
    }
}
