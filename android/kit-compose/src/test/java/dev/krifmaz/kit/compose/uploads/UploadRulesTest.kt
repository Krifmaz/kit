package dev.krifmaz.kit.compose.uploads

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Which files an upload takes (picker and drop alike), the words for the others, and the file limits. */
class UploadRulesTest {
    @Test
    fun acceptReadsEndingsAndTypes() {
        assertTrue(UploadRules.accepts("", "anything.bin", ""))
        assertTrue(UploadRules.accepts(".zip,application/zip", "Backup.ZIP", ""))
        assertTrue(UploadRules.accepts(".zip,application/zip", "x", "application/zip"))
        assertFalse(UploadRules.accepts(".zip,application/zip", "notes.txt", "text/plain"))
        assertTrue(UploadRules.accepts("image/*", "a", "image/png"))
        assertFalse(UploadRules.accepts("image/*", "a.png", "")) // no type: only an ending can match
        assertFalse(UploadRules.accepts("image/*", "a", "imagex/png"))
        assertEquals(".zip, .db", UploadRules.acceptHint(".zip, application/zip, .db"))
        assertEquals("", UploadRules.acceptHint("image/*"))
    }

    @Test
    fun aDropIsSplitAndRefusedFilesAreNamed() {
        val files = listOf("a.zip" to "application/zip", "b.txt" to "text/plain", "c.zip" to "")
        val (many, refused) = UploadRules.choose(files, ".zip", multiple = true, { it.first }, { it.second })
        assertEquals(listOf("a.zip", "c.zip"), many.map { it.first })
        assertEquals(listOf("b.txt"), refused)
        assertEquals("b.txt can't be uploaded here (it takes .zip)", UploadRules.refusedMessage(refused, ".zip", true, 2))
        assertEquals("info", UploadRules.refusedKind(2))

        val (one, extra) = UploadRules.choose(files, ".zip", multiple = false, { it.first }, { it.second })
        assertEquals(1, one.size)
        assertEquals(UploadRules.ONLY_ONE, UploadRules.refusedMessage(extra, ".zip", false, 1))

        assertEquals("2 files can't be uploaded here", UploadRules.refusedMessage(listOf("x", "y"), "image/*", true, 0))
        assertEquals("err", UploadRules.refusedKind(0))
        assertEquals("", UploadRules.refusedMessage(emptyList(), "", true, 3))
    }

    @Test
    fun limitsSayWhatDoesntFit() {
        val l = FileLimits.BUG_REPORT
        assertEquals("Up to 6 files, 8 MB each.", l.summary())
        val (taken, said) = l.add(4, listOf("a.png" to 10L, "big.png" to 9L * 1024 * 1024, "b.png" to 1L, "c.png" to 1L))
        assertEquals(listOf(0, 2), taken)
        assertEquals(listOf("big.png is over 8 MB", "At most 6 files per report"), said)
        assertEquals("At most 3 files per upload", FileLimits(3, 1024 * 1024).tooMany())
    }
}
