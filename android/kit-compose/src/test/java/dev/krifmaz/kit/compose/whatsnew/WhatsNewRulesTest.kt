package dev.krifmaz.kit.compose.whatsnew

import dev.krifmaz.kit.compose.core.parseJsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WhatsNewRulesTest {
    private fun li(text: String, lead: String = "") = ChangelogBlock("li", text = text, lead = lead)

    private fun row(vararg cells: String, head: Boolean = false) = ChangelogBlock("row", cells = cells.toList(), head = head)

    @Test
    fun bulletsAndRowsGroupIntoListsAndTables() {
        val blocks =
            listOf(
                ChangelogBlock("p", text = "Intro"),
                li("one", lead = "Offline:"),
                li("two"),
                ChangelogBlock("h", text = "Fixes"),
                li("three"),
                row("A", "B", head = true),
                row("1", "2"),
                ChangelogBlock("p", text = "End"),
            )
        assertEquals(
            listOf(
                NoteGroup.Paragraph("Intro"),
                NoteGroup.Bullets(listOf(li("one", lead = "Offline:"), li("two"))),
                NoteGroup.Heading("Fixes"),
                NoteGroup.Bullets(listOf(li("three"))),
                NoteGroup.Table(listOf(row("A", "B", head = true), row("1", "2"))),
                NoteGroup.Paragraph("End"),
            ),
            WhatsNewRules.group(blocks),
        )
    }

    @Test
    fun unknownKindsAreLeftOutAndEndARun() {
        val g = WhatsNewRules.group(listOf(li("a"), ChangelogBlock("x"), li("b")))
        assertEquals(listOf(NoteGroup.Bullets(listOf(li("a"))), NoteGroup.Bullets(listOf(li("b")))), g)
        assertEquals(emptyList<NoteGroup>(), WhatsNewRules.group(emptyList()))
    }

    @Test
    fun popupShowsOnlyWhenTheServerSaysSoAndThereIsSomething() {
        val r = WhatsNewRelease("0.36.4", "Encore 0.36.4", "", listOf("x"))
        assertTrue(WhatsNewRules.shouldShow(ChangelogNew("0.36.4", true, true, listOf(r))))
        assertFalse(WhatsNewRules.shouldShow(ChangelogNew("0.36.4", true, true, emptyList())))
        // a hotfix (or the popup switched off): the server sends show = false
        assertFalse(WhatsNewRules.shouldShow(ChangelogNew("0.36.4", false, false, listOf(r))))
    }

    @Test
    fun parsesTheContractsAnswers() {
        val new =
            WhatsNewRules.newOf(
                parseJsonObject(
                    """{"version":"0.36.4","popup":true,"show":true,"releases":[{"version":"0.36.4","title":"Encore 0.36.4","summary":"S","highlights":["a","b"]}]}""",
                )!!,
            )
        assertEquals(ChangelogNew("0.36.4", true, true, listOf(WhatsNewRelease("0.36.4", "Encore 0.36.4", "S", listOf("a", "b")))), new)
        val all =
            WhatsNewRules.changelogOf(
                parseJsonObject(
                    """{"version":"0.36.4","popup":false,"releases":[{"version":"0.36.4","title":"T","summary":"","highlights":[],
                    "blocks":[{"k":"li","lead":"L","text":"t"},{"k":"row","cells":["a","b"],"head":true}]}]}""",
                )!!,
            )
        assertFalse(all.popup)
        assertEquals(listOf(ChangelogBlock("li", text = "t", lead = "L"), ChangelogBlock("row", cells = listOf("a", "b"), head = true)), all.releases[0].blocks)
    }

    @Test
    fun wordsAreTheWebs() {
        assertEquals("What's new will show after updates", WhatsNewRules.popupSaved(true))
        assertEquals("What's new won't show after updates – it stays in Settings", WhatsNewRules.popupSaved(false))
        assertEquals(
            "This server runs Encore 0.36.4. After an update, a short list of what changed shows once on the website and in the desktop app.",
            WhatsNewRules.settingsIntro("Encore", "0.36.4"),
        )
    }
}
