package dev.krifmaz.kit.compose.bugs

import android.app.Application
import android.net.Uri
import dev.krifmaz.kit.compose.KitApp

/**
 * "Send crash log" (from the app's crash reports): the phone's crash log ([CrashLog.exportText]) as a bug report of the
 * kind `crash`, so it lands in the admin's Bug reports under Crash logs (and in its .txt export, written in whole).
 * The description is Android's exit records; the whole log is the attached .txt. Blocking: call it off the main thread.
 * Gives the new report's id.
 */
object CrashLogReport {
    /** The server takes text files up to 8 MB; the log is cut well before that (by characters: it stays UTF-8). */
    private const val MAX_CHARS = 3_000_000

    /** The server's limit for a description is 20,000 characters. */
    private const val MAX_BODY = 18_000

    /**
     * Sends the crash log through [reports] as a `crash` report, with [note] first; gives the new report's id.
     * [page] is the report's "page" (where it was sent from, "Settings" by default).
     */
    fun send(app: Application, reports: BugReportsApi, note: String = "", page: String = "Settings"): Long {
        val file = CrashLog.exportText(app)
        val bytes = file.readText().let { if (it.length > MAX_CHARS) it.take(MAX_CHARS) + "\n… (cut)" else it }.toByteArray()
        val summary = CrashLog.latestSummary(app).ifBlank { "sent from the app" }
        val body = buildString {
            if (note.isNotBlank()) append(note.trim()).append("\n\n")
            append("Why Android closed the app (newest first):\n")
            append(CrashLog.exitHistory(app).ifBlank { "(none recorded)\n" })
            append("\nThe whole log is in the attached file.")
        }.take(MAX_BODY)
        return reports.create(
            kind = "crash",
            title = BugRules.crashTitle(summary),
            body = body,
            page = page,
            appVersion = "${KitApp.versionName} (${KitApp.versionCode})",
            device = CrashLog.deviceLine(),
            files = listOf(Attachment(Uri.fromFile(file), file.name, "text/plain", bytes.size.toLong(), bytes)),
            resolver = app.contentResolver,
        )
    }
}
