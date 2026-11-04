package dev.krifmaz.kit.compose.bugs

import coil3.ImageLoader

/**
 * What the bug report screens need from the app: its calls, its image loader (which carries the session, for the
 * reports' pictures), the server's address for a file's path, who is looking, and the app's navigation.
 */
interface BugReportsHost {
    /** The bug report calls for this app's session. */
    val bugReports: BugReportsApi

    /** The app's image loader; it must carry the session (the pictures are behind the server's sign-in). */
    val imageLoader: ImageLoader

    /** The full URL of a server path ("/api/bug-reports/…/files/…"), for the pictures. */
    fun url(path: String): String

    /** Whether the person looking is an admin (blocking, called off the main thread; may throw when offline). */
    fun isAdmin(): Boolean

    /** Opens the list of bug reports; [from] names the page it was opened from (a new report sends it along). */
    fun openList(from: String)

    /** Opens one report. */
    fun openReport(id: Long)

    /** Goes back from a bug report screen. */
    fun back()
}
