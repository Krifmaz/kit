package {{android_package}}

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/** Which suite links open a page of this app, and which go to the browser. */
class SuiteLinksTest {
    /** A section's path of the web app's opens its page here. */
    @Test
    fun sectionPathsArePages() {
        assertEquals(Page.Home, SuiteLinks.pageOf("/#/"))
        assertEquals(Page.BugReports("Home"), SuiteLinks.pageOf("/#/bug-reports"))
        assertEquals(Page.Search, SuiteLinks.pageOf("/#/search?q=x"))
        assertNull(SuiteLinks.pageOf("/#/library"))
    }

    /** A hit's whole address opens a page only when it is this server's. */
    @Test
    fun addressesOfThisServerArePages() {
        assertEquals(Page.BugReports("Home"), SuiteLinks.pageOf("https://me.example/#/bug-reports", "https://me.example"))
        assertNull(SuiteLinks.pageOf("https://other.example/#/bug-reports", "https://me.example"))
        assertNull(SuiteLinks.pageOf("https://other.example/music", ""))
    }
}
