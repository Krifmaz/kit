package dev.krifmaz.kit.demo

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.bugs.BugReportsEntry
import dev.krifmaz.kit.compose.bugs.BugReportsHost
import dev.krifmaz.kit.compose.bugs.CrashLog
import dev.krifmaz.kit.compose.bugs.CrashLogReport
import dev.krifmaz.kit.compose.bugs.NewBugReportSheet
import dev.krifmaz.kit.compose.core.formatSize
import dev.krifmaz.kit.compose.details.CopyIconButton
import dev.krifmaz.kit.compose.details.DetailsDialog
import dev.krifmaz.kit.compose.dialogs.Confirm
import dev.krifmaz.kit.compose.dialogs.Confirms
import dev.krifmaz.kit.compose.notices.CheckWords
import dev.krifmaz.kit.compose.notices.NoticeBanners
import dev.krifmaz.kit.compose.notices.rememberNotices
import dev.krifmaz.kit.compose.theme.KitAppearance
import dev.krifmaz.kit.compose.theme.KitTheme
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.update.UpdatesSection
import dev.krifmaz.kit.compose.uploads.DropZone
import dev.krifmaz.kit.compose.uploads.FileLimits
import dev.krifmaz.kit.compose.uploads.PickedFile
import dev.krifmaz.kit.compose.whatsnew.ChangelogBackend
import dev.krifmaz.kit.compose.whatsnew.WhatsNewPopup
import dev.krifmaz.kit.compose.whatsnew.WhatsNewRelease
import dev.krifmaz.kit.compose.widgets.Badge
import dev.krifmaz.kit.compose.widgets.CardBox
import dev.krifmaz.kit.compose.widgets.ChoiceRow
import dev.krifmaz.kit.compose.widgets.EmptyState
import dev.krifmaz.kit.compose.widgets.Gutter
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.KitField
import dev.krifmaz.kit.compose.widgets.MenuEntry
import dev.krifmaz.kit.compose.widgets.MoreButton
import dev.krifmaz.kit.compose.widgets.RowDivider
import dev.krifmaz.kit.compose.widgets.SectionHead
import dev.krifmaz.kit.compose.widgets.SmallButton
import dev.krifmaz.kit.compose.widgets.Spinner
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The themes of the picker, in its order, with their names. */
private val THEMES = listOf(KitTheme.Encore to "Encore", KitTheme.Dark to "Dark", KitTheme.Oled to "OLED", KitTheme.Light to "Light", KitTheme.System to "System")

/** What a demo upload takes: up to 3 pictures of 8 MB. */
private val PICTURES = FileLimits(maxFiles = 3, maxBytes = 8L * 1024 * 1024, per = "upload")

/**
 * Every kit part, top to bottom. The list's resource id ("demo-list") is what the benchmarks scroll.
 *
 * @param app the demo's process (its server).
 * @param look the theme picker's state.
 * @param offline offline mode.
 * @param confirms the confirm dialogs.
 * @param changelog What's new, from the demo server.
 * @param host the bug report screens' host.
 * @param open opens a page.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun DemoHome(app: DemoApp, look: DemoLook, offline: DemoOffline, confirms: Confirms, changelog: ChangelogBackend, host: BugReportsHost, open: (Page) -> Unit) {
    val p = MaterialTheme.palette
    val scope = rememberCoroutineScope()
    val notices = rememberNotices(app.server.kit, admin = true)
    var details by remember { mutableStateOf(false) }
    var whatsNew by remember { mutableStateOf<List<WhatsNewRelease>?>(null) }
    var newReport by remember { mutableStateOf(false) }
    var tab by remember { mutableIntStateOf(0) }
    var field by remember { mutableStateOf("") }
    var picked by remember { mutableStateOf<List<PickedFile>>(emptyList()) }
    var serverDown by remember { mutableStateOf(!app.server.backend.up) }
    var sending by remember { mutableStateOf(false) }
    Column(
        Modifier
            .fillMaxSize()
            .semantics { testTagsAsResourceId = true }
            .testTag("demo-list")
            .verticalScroll(rememberScrollState()),
    ) {
        NoticeBanners(notices, admin = true, words = mapOf("mail" to CheckWords(fixed = "The mail server answers again")))
        SectionHead("Kit demo", subtitle = "Every part of the kit, on one screen")

        // ---- look
        SectionHead("Theme", subtitle = "Four themes, System, and the accents")
        ChoiceRow(THEMES.map { it.second }, THEMES.indexOfFirst { it.first.id == look.theme }.coerceAtLeast(0), { look.setTheme(THEMES[it].first.id) })
        Row(Modifier.horizontalScroll(rememberScrollState()).padding(horizontal = Gutter, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Badge("Theme's own", kind = if (look.accent.isEmpty()) "accent" else "", onClick = { look.setAccent("") })
            for (a in KitAppearance.ACCENTS) Badge(a.name, kind = if (look.accent == a.id) "accent" else "", onClick = { look.setAccent(a.id) })
        }

        // ---- the design system
        SectionHead("Design system", subtitle = "Bottom bar, covers, carousels, stats, empty states")
        Row(Modifier.padding(horizontal = Gutter)) { KitButton("Open the design system", primary = true) { open(Page.Design) } }

        // ---- widgets
        SectionHead("Buttons and badges")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            KitButton("Primary", primary = true) { Toasts.show("Primary tapped") }
            KitButton("Button") { Toasts.show("Button tapped") }
            MoreButton(listOf(MenuEntry("Rename") { Toasts.show("Rename tapped") }, null, MenuEntry("Delete", danger = true) { Toasts.show("Delete tapped") }))
        }
        Row(Modifier.padding(Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            SmallButton("Small") {}
            SmallButton("Danger", danger = true) {}
            Spinner()
        }
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            Badge("muted")
            Badge("ok", kind = "ok")
            Badge("warn", kind = "warn")
            Badge("err", kind = "err")
            Badge("info", kind = "info")
            Badge("accent", kind = "accent")
        }
        ChoiceRow(listOf("Inbox", "Drafts", "Sent"), tab, { tab = it })

        // ---- toasts
        SectionHead("Toasts", subtitle = "Errors stay until closed and have Copy; the same message twice merges")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Info") { Toasts.show("Saved") }
            SmallButton("OK") { Toasts.show("Done", "ok") }
            SmallButton("Error") { Toasts.show("The server answered 502 Bad Gateway – it may be restarting", "err") }
            SmallButton("Undo") { Toasts.show("Moved to the Trash", action = "Undo", onAction = { Toasts.show("Restored", "ok") }) }
        }

        // ---- dialogs
        SectionHead("Details and confirm dialogs")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            SmallButton("Details") { details = true }
            SmallButton("Confirm") {
                confirms.ask(Confirm("Sign out?", "You can sign in again any time.", okLabel = "Sign out")) { Toasts.show("Signed out", "ok") }
            }
            SmallButton("Type to confirm", danger = true) {
                confirms.ask(
                    Confirm("Delete the account?", "Every note and file goes with it. This can't be undone.", okLabel = "Delete", danger = true, typeToConfirm = "delete"),
                ) {
                    Toasts.show("Deleted", "ok")
                }
            }
            CopyIconButton("Text to copy")
        }

        // ---- What's new
        SectionHead("What's new", subtitle = "The popup after an update, and the whole changelog")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("What's new") {
                scope.launch {
                    try {
                        whatsNew = withContext(Dispatchers.IO) { changelog.new() }.releases
                    } catch (e: Exception) {
                        Toasts.show(e.message ?: "Something went wrong", "err")
                    }
                }
            }
            SmallButton("Changelog") { open(Page.Changelog) }
        }

        // ---- updates
        SectionHead("Updates", subtitle = "Looks for a newer release of the app's own GitHub repository")
        UpdatesSection(Modifier.padding(horizontal = Gutter))

        // ---- notices
        SectionHead("Notice banners", subtitle = "A problem the server found, shown to admins at the top, with Check again")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Break the mail server") {
                app.server.backend.mailOk = false
                scope.launch { notices.refresh() }
            }
        }

        // ---- bug reports
        SectionHead("Bug reports and crash log")
        BugReportsEntry(host)
        Row(Modifier.padding(horizontal = Gutter, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("New report") { newReport = true }
            SmallButton("Send the crash log", enabled = !sending) {
                sending = true
                scope.launch {
                    try {
                        val id = withContext(Dispatchers.IO) { CrashLogReport.send(app, app.bugReports, page = "Kit demo") }
                        Toasts.show("Crash log sent as report #$id", "ok")
                    } catch (e: Exception) {
                        Toasts.show(e.message ?: "Something went wrong", "err")
                    } finally {
                        sending = false
                    }
                }
            }
            SmallButton("Crash the app", danger = true) {
                confirms.ask(Confirm("Crash the app?", "It closes now; at the next start the crash goes to Diagnostics → App crashes.", okLabel = "Crash", danger = true)) {
                    CrashLog.log("crash button")
                    throw IllegalStateException("A crash on purpose, from the demo's Crash the app")
                }
            }
        }

        // ---- offline
        SectionHead("Offline mode", subtitle = "The banner, why the server can't be reached, and changes that wait")
        Row(Modifier.fillMaxWidth().padding(horizontal = Gutter), verticalAlignment = Alignment.CenterVertically) {
            Text("Server down", color = p.text, modifier = Modifier.weight(1f))
            Switch(
                checked = serverDown,
                onCheckedChange = {
                    serverDown = it
                    app.server.backend.up = !it
                    scope.launch { offline.check() }
                },
            )
        }
        Row(Modifier.padding(horizontal = Gutter, vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Write a note") { offline.change(DemoOffline.NOTE, "note-${System.nanoTime()}") }
            SmallButton("Star it") { offline.change(DemoOffline.STAR, "item-1") }
        }

        // ---- settings
        SectionHead("Settings", subtitle = "Drawn from the server's schema; Diagnostics with the app's crashes; Sign-up with invites")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Server settings") { open(Page.Settings) }
            SmallButton("Diagnostics") { open(Page.Diagnostics) }
            SmallButton("Sign-up") { open(Page.Signup) }
        }

        // ---- uploads
        SectionHead("Drag and drop", subtitle = PICTURES.summary())
        DropZone(
            accept = "image/*",
            multiple = true,
            onFiles = { files ->
                val (taken, said) = PICTURES.add(picked.size, files.map { it.name to it.size })
                said.forEach { Toasts.show(it, "warn") }
                picked = picked + taken.map { files[it] }
            },
            modifier = Modifier.padding(horizontal = Gutter),
        )
        for (f in picked) Text("${f.name} · ${formatSize(f.size)}", color = p.muted, modifier = Modifier.padding(horizontal = Gutter, vertical = 2.dp))

        // ---- card, field, empty state
        SectionHead("Card and field")
        Column(Modifier.padding(horizontal = Gutter), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CardBox { Text("A card", color = p.text) }
            KitField(field, { field = it }, placeholder = "A field", modifier = Modifier.fillMaxWidth())
        }
        RowDivider(Modifier.padding(top = 16.dp))
        EmptyState(R.drawable.kit_ic_cloud_off, "An empty state", "With a line below it")
        Box(Modifier.height(400.dp)) // room to scroll, for the benchmarks
    }

    if (details) {
        DetailsDialog("Details", "java.io.IOException: unexpected end of stream on https://demo.invalid/...\n\tat okhttp3...") { details = false }
    }
    whatsNew?.let { releases ->
        WhatsNewPopup(
            releases = releases,
            onPopupChange = { on ->
                scope.launch {
                    withContext(Dispatchers.IO) { runCatching { changelog.setPopup(on) } }
                    Toasts.show(if (on) "What's new will show after updates" else "What's new won't show after updates – it stays in Settings", "ok")
                }
            },
            onFullChangelog = {
                whatsNew = null
                open(Page.Changelog)
            },
            onDismiss = {
                whatsNew = null
                scope.launch { withContext(Dispatchers.IO) { runCatching { changelog.markSeen() } } }
            },
        )
    }
    if (newReport) NewBugReportSheet(host, page = "Kit demo") { newReport = false }
}
