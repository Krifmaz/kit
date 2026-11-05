package {{android_package}}

/**
 * Where a link of the suite (a shared sidebar entry or a search hit) goes in this app: its page for a path of the
 * web app's (`/#/bug-reports`, or a whole address on [base] with one), null for anything else (another app, opened
 * in the browser).
 */
object SuiteLinks {
    /** The page of a section path or a hit's address; [base] is this server's address. */
    fun pageOf(link: String, base: String = ""): Page? {
        val path = when {
            link.startsWith("/") -> link
            base.isNotEmpty() && link.startsWith(base.trimEnd('/') + "/") -> link.removePrefix(base.trimEnd('/'))
            else -> return null
        }
        val page = path.removePrefix("/").removePrefix("#").substringBefore('?').trimEnd('/')
        return when (page) {
            "" -> Page.Home
            "/bug-reports" -> Page.BugReports(from = "Home")
            "/search" -> Page.Search
            else -> null
        }
    }
}
