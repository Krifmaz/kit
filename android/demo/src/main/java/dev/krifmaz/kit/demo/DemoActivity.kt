package dev.krifmaz.kit.demo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.details.CopyIconButton
import dev.krifmaz.kit.compose.details.DetailsDialog
import dev.krifmaz.kit.compose.perf.KitPerf
import dev.krifmaz.kit.compose.theme.KitTheme
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.ToastHost
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.whatsnew.ChangelogScreen
import dev.krifmaz.kit.compose.whatsnew.WhatsNewPopup
import dev.krifmaz.kit.compose.widgets.BackBar
import dev.krifmaz.kit.compose.widgets.Badge
import dev.krifmaz.kit.compose.widgets.CardBox
import dev.krifmaz.kit.compose.widgets.ChoiceRow
import dev.krifmaz.kit.compose.dialogs.ConfirmDialog
import dev.krifmaz.kit.compose.widgets.EmptyState
import dev.krifmaz.kit.compose.widgets.Gutter
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.KitField
import dev.krifmaz.kit.compose.widgets.RowDivider
import dev.krifmaz.kit.compose.widgets.SectionHead
import dev.krifmaz.kit.compose.widgets.SmallButton
import dev.krifmaz.kit.compose.widgets.Spinner

/** The demo: every kit-compose component on one scrolling screen, in the Encore theme. */
class DemoActivity : ComponentActivity() {
    /** Sets up the kit, the debug checks and the screen. */
    override fun onCreate(savedInstanceState: Bundle?) {
        KitPerf.strictModeInDebug(BuildConfig.DEBUG)
        super.onCreate(savedInstanceState)
        KitApp.init(name = "Kit demo", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE, userAgent = "KitDemo/${BuildConfig.VERSION_NAME}")
        setContent {
            KitTheme(theme = KitTheme.Encore) {
                Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                    DemoApp()
                    ToastHost(bottomPadding = 16.dp)
                }
            }
        }
        KitPerf.jankStatsInDebug(this, BuildConfig.DEBUG)
    }
}

/** The demo's two pages: the components, and Settings → What's new. */
@Composable
private fun DemoApp() {
    var changelog by remember { mutableStateOf(false) }
    if (changelog) {
        BackHandler { changelog = false }
        Column(Modifier.fillMaxSize()) {
            BackBar("What's new") { changelog = false }
            ChangelogScreen(DemoChangelog)
        }
    } else {
        Components(onChangelog = { changelog = true })
    }
}

/** Every component, top to bottom. The list's resource id ("demo-list") is what the benchmarks scroll. */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun Components(onChangelog: () -> Unit) {
    val p = MaterialTheme.palette
    var details by remember { mutableStateOf(false) }
    var confirm by remember { mutableStateOf(false) }
    var whatsNew by remember { mutableStateOf(false) }
    var tab by remember { mutableIntStateOf(0) }
    var field by remember { mutableStateOf("") }
    Column(
        Modifier
            .fillMaxSize()
            .semantics { testTagsAsResourceId = true }
            .testTag("demo-list")
            .verticalScroll(rememberScrollState()),
    ) {
        SectionHead("Kit demo", subtitle = "Every kit-compose component, in the Encore theme")

        SectionHead("Buttons and badges")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            KitButton("Primary", primary = true) { Toasts.show("Primary tapped") }
            KitButton("Button") { Toasts.show("Button tapped") }
        }
        Row(Modifier.padding(Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
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
        ChoiceRow(listOf("Albums", "Artists", "Songs"), tab, { tab = it })

        SectionHead("Toasts")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Info") { Toasts.show("Saved") }
            SmallButton("OK") { Toasts.show("Done", "ok") }
            SmallButton("Error") { Toasts.show("The server answered 502 Bad Gateway – it may be restarting", "err") }
            SmallButton("Undo") { Toasts.show("Moved to the Trash", action = "Undo", onAction = { Toasts.show("Restored", "ok") }) }
        }

        SectionHead("Details and dialogs")
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SmallButton("Details") { details = true }
            SmallButton("Confirm") { confirm = true }
            SmallButton("What's new") { whatsNew = true }
            SmallButton("Changelog", onClick = onChangelog)
            CopyIconButton("Text to copy")
        }

        SectionHead("Card and field")
        Column(Modifier.padding(horizontal = Gutter), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            CardBox {
                Text("A card (the web's .card)", color = p.text)
            }
            KitField(field, { field = it }, placeholder = "A field (the web's .input)", modifier = Modifier.fillMaxWidth())
        }
        RowDivider(Modifier.padding(top = 16.dp))
        EmptyState(R.drawable.kit_ic_cloud_off, "An empty state", "With a line below it")

        // TODO(track C): update (UpdatesSection), bugs (bug report sheet, crash log), offline (banner, reasons),
        // settings (SchemaForm, DiagnosticsScreen) — added once those packages land.
        SectionHead("More", subtitle = "Updates, bug reports, offline and settings forms come next")
        Box(Modifier.height(400.dp)) // room to scroll, for the benchmarks
    }

    if (details) {
        DetailsDialog("Details", "java.io.IOException: unexpected end of stream on http://192.168.1.10:8080/...\n\tat okhttp3...") { details = false }
    }
    if (confirm) {
        ConfirmDialog("Delete this?", "It can't be undone.", "Delete", danger = true, onDismiss = { confirm = false }) { Toasts.show("Deleted", "ok") }
    }
    if (whatsNew) {
        WhatsNewPopup(
            releases = DemoChangelog.new().releases,
            onPopupChange = { Toasts.show(if (it) "What's new will show after updates" else "What's new won't show after updates – it stays in Settings", "ok") },
            onFullChangelog = {
                whatsNew = false
                onChangelog()
            },
            onDismiss = { whatsNew = false },
        )
    }
}
