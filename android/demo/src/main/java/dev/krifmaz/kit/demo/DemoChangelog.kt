package dev.krifmaz.kit.demo

import dev.krifmaz.kit.compose.whatsnew.Changelog
import dev.krifmaz.kit.compose.whatsnew.ChangelogBackend
import dev.krifmaz.kit.compose.whatsnew.ChangelogBlock
import dev.krifmaz.kit.compose.whatsnew.ChangelogNew
import dev.krifmaz.kit.compose.whatsnew.ChangelogRelease
import dev.krifmaz.kit.compose.whatsnew.WhatsNewRelease

/** A changelog kept in memory, so the demo shows What's new without a server. */
object DemoChangelog : ChangelogBackend {
    @Volatile
    private var popup = true

    private val release =
        ChangelogRelease(
            version = "0.1.0",
            title = "Kit demo 0.1.0",
            summary = "The first release of the kit.",
            highlights = listOf("Encore's theme", "Toasts with Copy", "What's new"),
            blocks =
            listOf(
                ChangelogBlock("p", text = "Everything that makes Encore feel good, for every app."),
                ChangelogBlock("h", text = "Added"),
                ChangelogBlock("li", lead = "Theme:", text = "generated from the design tokens"),
                ChangelogBlock("li", lead = "Toasts:", text = "errors stay with Copy, duplicates merge"),
                ChangelogBlock("row", cells = listOf("Part", "Platform"), head = true),
                ChangelogBlock("row", cells = listOf("kit-compose", "Android")),
            ),
        )

    /** The one release, as new. */
    override fun new(): ChangelogNew = ChangelogNew("0.1.0", popup, popup, listOf(WhatsNewRelease(release.version, release.title, release.summary, release.highlights)))

    /** Nothing to remember in the demo. */
    override fun markSeen() {}

    /** The one release. */
    override fun all(): Changelog = Changelog("0.1.0", popup, listOf(release))

    /** Keeps the switch for this run. */
    override fun setPopup(on: Boolean) {
        popup = on
    }
}
