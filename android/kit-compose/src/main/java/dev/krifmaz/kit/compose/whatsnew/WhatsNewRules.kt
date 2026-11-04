package dev.krifmaz.kit.compose.whatsnew

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.KitRoutes
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.core.strings
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.io.IOException

/**
 * One release in the What's new popup (`GET /api/changelog/new`, contracts/changelog.md).
 *
 * @property version the release's version ("0.36.4").
 * @property title "<App> <version>", or what follows the version in the notes' heading.
 * @property summary the paragraph before the first section ("" when there is none).
 * @property highlights at most 6 short lines (the bold leads of the bullets, else the first bullets).
 */
data class WhatsNewRelease(val version: String, val title: String, val summary: String, val highlights: List<String>)

/**
 * The answer of `GET /api/changelog/new`.
 *
 * @property version this server's version.
 * @property popup the account's popup switch (`prefs.whats_new_popup`).
 * @property show `popup` and at least one unseen release: the client shows the popup.
 * @property releases the unseen releases, newest first.
 */
data class ChangelogNew(val version: String, val popup: Boolean, val show: Boolean, val releases: List<WhatsNewRelease>)

/**
 * One part of a release's notes as the server sends it (`k`: h, p, li, row).
 *
 * @property k the kind: "h" (a heading), "p" (a paragraph), "li" (a bullet), "row" (a table row).
 * @property text the words of a heading, paragraph or bullet.
 * @property lead a bullet's bold start ("" when it has none).
 * @property cells a table row's cells.
 * @property head whether a table row is the header row.
 */
data class ChangelogBlock(val k: String, val text: String = "", val lead: String = "", val cells: List<String> = emptyList(), val head: Boolean = false)

/**
 * One release in Settings → What's new (`GET /api/changelog`).
 *
 * @property version the release's version.
 * @property title the release's title.
 * @property summary the paragraph before the first section.
 * @property highlights the popup's short lines.
 * @property blocks the notes, part by part.
 */
data class ChangelogRelease(val version: String, val title: String, val summary: String, val highlights: List<String>, val blocks: List<ChangelogBlock>)

/**
 * The answer of `GET /api/changelog`: every release with notes, newest first.
 *
 * @property version this server's version.
 * @property popup the account's popup switch.
 * @property releases the releases.
 */
data class Changelog(val version: String, val popup: Boolean, val releases: List<ChangelogRelease>)

/** A run of a release's notes as it is drawn: bullets and table rows grouped into their lists and tables. */
sealed interface NoteGroup {
    /** A `### Heading`. */
    data class Heading(val text: String) : NoteGroup

    /** A paragraph. */
    data class Paragraph(val text: String) : NoteGroup

    /** Bullets in a row, as one list. */
    data class Bullets(val items: List<ChangelogBlock>) : NoteGroup

    /** Table rows in a row, as one table. */
    data class Table(val rows: List<ChangelogBlock>) : NoteGroup
}

/**
 * The rules and words of What's new (the same rules as kit-rules' `whats_new`): the popup on the first start after the server
 * was updated (once shown, `POST /api/changelog/seen` marks this version seen for the account), and Settings →
 * What's new, the full changelog with the switch that turns the popup off. Hotfix builds share their release's notes,
 * so they never bring the popup back on their own (the server decides; the client only follows `show`).
 */
object WhatsNewRules {
    /** The popup's title. */
    const val TITLE = "What's new"

    /** The popup's switch. */
    const val POPUP_SWITCH = "Show this after updates"

    /** The popup's button to Settings → What's new. */
    const val FULL_CHANGELOG = "Full changelog"

    /** The popup's close button. */
    const val CLOSE = "Close"

    /** Settings → What's new: the switch. */
    const val SETTINGS_SWITCH = "Show what's new after updates"

    /** Settings → What's new with no notes. */
    const val NO_NOTES = "No release notes in this build."

    /** The toast after the switch was turned on. */
    const val TOAST_ON = "What's new will show after updates"

    /** The toast after the switch was turned off. */
    const val TOAST_OFF = "What's new won't show after updates – it stays in Settings"

    /** The toast after the popup switch was saved. */
    fun popupSaved(on: Boolean): String = if (on) TOAST_ON else TOAST_OFF

    /** Settings → What's new: the line under the heading. */
    fun settingsIntro(app: String, version: String): String =
        "This server runs $app $version. After an update, a short list of what changed shows once on the website and in the desktop app."

    /** Whether the popup shows: the server says so and there is something to show. */
    fun shouldShow(new: ChangelogNew): Boolean = new.show && new.releases.isNotEmpty()

    /** A release's notes as drawn: bullets grouped into lists, table rows into tables; unknown kinds are left out. */
    fun group(blocks: List<ChangelogBlock>): List<NoteGroup> {
        val out = ArrayList<NoteGroup>()
        var run = ArrayList<ChangelogBlock>()
        var open = ""

        fun close() {
            when (open) {
                "li" -> out.add(NoteGroup.Bullets(run))
                "row" -> out.add(NoteGroup.Table(run))
            }
            run = ArrayList()
            open = ""
        }
        for (b in blocks) {
            val want = if (b.k == "li" || b.k == "row") b.k else ""
            if (want != open) close()
            when (b.k) {
                "h" -> out.add(NoteGroup.Heading(b.text))

                "p" -> out.add(NoteGroup.Paragraph(b.text))

                "li", "row" -> {
                    open = b.k
                    run.add(b)
                }
            }
        }
        close()
        return out
    }

    /** A popup release from its JSON. */
    fun releaseOf(o: JsonObject): WhatsNewRelease = WhatsNewRelease(o.str("version"), o.str("title"), o.str("summary"), o.arr("highlights").strings())

    /** `GET /api/changelog/new` from its JSON. */
    fun newOf(o: JsonObject): ChangelogNew = ChangelogNew(o.str("version"), o.bool("popup"), o.bool("show"), o.arr("releases").objectsOf().map(::releaseOf))

    /** A block from its JSON. */
    fun blockOf(o: JsonObject): ChangelogBlock = ChangelogBlock(o.str("k"), o.str("text"), o.str("lead"), o.arr("cells").strings(), o.bool("head"))

    /** `GET /api/changelog` from its JSON. */
    fun changelogOf(o: JsonObject): Changelog = Changelog(
        o.str("version"),
        o.bool("popup"),
        o.arr("releases").objectsOf().map { r ->
            ChangelogRelease(r.str("version"), r.str("title"), r.str("summary"), r.arr("highlights").strings(), r.arr("blocks").objectsOf().map(::blockOf))
        },
    )
}

/**
 * The server side of What's new, implemented by the app (with its own HTTP client and session). Every call blocks and
 * throws on failure, with a message to show as it is; the kit calls them off the main thread.
 */
interface ChangelogBackend {
    /** `GET /api/changelog/new` (parse with [WhatsNewRules.newOf]). */
    fun new(): ChangelogNew

    /** `POST /api/changelog/seen`: this server's version is seen for the account. */
    fun markSeen()

    /** `GET /api/changelog` (parse with [WhatsNewRules.changelogOf]). */
    fun all(): Changelog

    /** `PUT /api/auth/prefs` with `{"whats_new_popup": on}`. */
    fun setPopup(on: Boolean)
}

/**
 * The [ChangelogBackend] of an app whose server answers at the kit's paths ([KitApp.routes]): the app passes its
 * [KitApi] and nothing else.
 *
 * @param api the app's server API (carries the session).
 * @param routes where the changelog calls go.
 */
class ApiChangelogBackend(private val api: KitApi, private val routes: KitRoutes = KitApp.routes) : ChangelogBackend {
    private fun obj(path: String): JsonObject = api.get(path) as? JsonObject ?: throw IOException("Unexpected reply from the server")

    override fun new(): ChangelogNew = WhatsNewRules.newOf(obj(routes.changelogNew))

    override fun markSeen() {
        api.post(routes.changelogSeen, JsonObject(emptyMap()))
    }

    override fun all(): Changelog = WhatsNewRules.changelogOf(obj(routes.changelog))

    override fun setPopup(on: Boolean) {
        api.put(routes.prefs, buildJsonObject { put("whats_new_popup", on) })
    }
}
