package dev.krifmaz.kit.compose.offline

import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.nio.file.Files

/** The offline outbox: generic changes, how they merge, the cap, the owner, the saved document and the words. */
class OutboxTest {
    private val ana = Outbox.account("https://app.example.com/", "ana")

    private fun vote(id: String, up: Boolean, at: Long) = OutAction("vote", id, buildJsonObject { put("up", up) }, at)

    private fun comment(id: String, at: Long) = OutAction("comment", id, buildJsonObject { put("text", "hi") }, at)

    @Test
    fun theOwnerIsServerAndUsername() {
        assertEquals("https://app.example.com|ana", ana)
    }

    @Test
    fun latestWinsReplacesTheWaitingChangeForTheSameKey() {
        val o = Outbox(ana).add(vote("1", true, 100), OutMerge.Latest).add(vote("1", false, 101), OutMerge.Latest)
        assertEquals(listOf(vote("1", false, 101)), o.actions)
        // the same change again changes nothing
        assertEquals(o, o.add(vote("1", false, 200), OutMerge.Latest))
    }

    @Test
    fun eachChangeIsKeptOncePerSecond() {
        val o = Outbox(ana).add(comment("1", 100)).add(comment("1", 100)).add(comment("1", 400))
        assertEquals(2, o.size)
        // no kind or no key: ignored
        assertEquals(o, o.add(OutAction("", "1")))
        assertEquals(o, o.add(OutAction("comment", "")))
    }

    @Test
    fun aKindIsCappedTheOldestGoFirst() {
        var o = Outbox(ana).add(vote("x", true, 1), OutMerge.Latest)
        for (i in 1..12) o = o.add(comment("c", i.toLong()), max = 10)
        assertEquals(10, o.of("comment").size)
        assertEquals(3L, o.of("comment").first().at)
        // another kind is not touched by the cap
        assertEquals(1, o.of("vote").size)
    }

    @Test
    fun anotherAccountsOutboxIsDropped() {
        val o = Outbox(ana).add(comment("1", 5))
        assertTrue(o.belongsTo(ana))
        assertFalse(o.belongsTo("https://app.example.com|bob"))
        assertFalse(Outbox("").belongsTo(""))
        assertTrue(o.own("https://app.example.com|bob").isEmpty)
        assertEquals(o, o.own(ana))
    }

    @Test
    fun whatWentIsForgottenAndAChangedOneStays() {
        val o = Outbox(ana).add(comment("1", 5)).add(comment("2", 6)).add(vote("3", true, 7), OutMerge.Latest)
        val now = o.add(vote("3", false, 9), OutMerge.Latest)
        val left = now.forget(listOf(comment("1", 5), vote("3", true, 7)))
        assertEquals(listOf(comment("2", 6), vote("3", false, 9)), left.actions)
    }

    @Test
    fun countsAreByKindInOrder() {
        val o = Outbox(ana).add(comment("1", 1)).add(vote("2", true, 2), OutMerge.Latest).add(comment("3", 3))
        assertEquals(mapOf("comment" to 2, "vote" to 1), o.counts())
    }

    @Test
    fun theDocumentReadsBack() {
        val o = Outbox(ana).add(comment("1", 1792310400)).add(OutAction("rename", "9", buildJsonObject { put("name", JsonPrimitive("New")) }, 1792310460))
        assertEquals(o, Outbox.parse(o.toJson().toString()))
        assertEquals(Outbox(), Outbox.parse("not json"))
        assertEquals(Outbox(), Outbox.parse(null))
    }

    @Test
    fun theWordsUseTheAppsNouns() {
        val nouns = mapOf("comment" to ("comment" to "comments"), "vote" to ("vote" to "votes"))
        assertEquals("", Outbox.countsText(emptyMap(), nouns))
        assertEquals("1 comment", Outbox.countsText(mapOf("comment" to 1), nouns))
        assertEquals("3 comments and 1 vote", Outbox.countsText(mapOf("comment" to 3, "vote" to 1), nouns))
        assertEquals("1 comment, 2 votes and 1 change", Outbox.countsText(mapOf("comment" to 1, "vote" to 2, "x" to 1), nouns))
        assertEquals("2 changes", Outbox.countsText(mapOf("x" to 2)))
        assertEquals("Back online", Outbox.backOnlineText(emptyMap()))
        assertEquals("Back online – 3 comments sent", Outbox.backOnlineText(mapOf("comment" to 3), nouns))
    }

    @Test
    fun theFileKeepsEveryChange() {
        val dir = Files.createTempDirectory("outbox").toFile()
        try {
            val f = File(dir, "outbox.json")
            val box = OutboxFile(f)
            box.update { it.own(ana).add(comment("1", 10)) }
            box.update { it.own(ana).add(vote("2", true, 11), OutMerge.Latest) }
            val again = OutboxFile(f).read()
            assertEquals(2, again.size)
            assertEquals(ana, again.owner)
        } finally {
            dir.deleteRecursively()
        }
    }
}
