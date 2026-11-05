package dev.krifmaz.kit.demo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import dev.krifmaz.kit.compose.bugs.BugReportScreen
import dev.krifmaz.kit.compose.bugs.BugReportsApi
import dev.krifmaz.kit.compose.bugs.BugReportsHost
import dev.krifmaz.kit.compose.bugs.BugReportsScreen
import dev.krifmaz.kit.compose.dialogs.ConfirmHost
import dev.krifmaz.kit.compose.dialogs.rememberConfirms
import dev.krifmaz.kit.compose.offline.OfflineBanner
import dev.krifmaz.kit.compose.perf.KitPerf
import dev.krifmaz.kit.compose.signup.SignupAdminScreen
import dev.krifmaz.kit.compose.theme.KitTheme
import dev.krifmaz.kit.compose.toasts.ToastHost
import dev.krifmaz.kit.compose.update.Updater
import dev.krifmaz.kit.compose.whatsnew.ApiChangelogBackend
import dev.krifmaz.kit.compose.whatsnew.ChangelogScreen
import dev.krifmaz.kit.compose.widgets.BackBar
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** The demo: every kit part, on one scrolling screen and the pages it opens. */
class DemoActivity : ComponentActivity() {
    /** Sets up the screen and the debug checks. */
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as DemoApp
        setContent {
            var theme by rememberSaveable { mutableStateOf(KitTheme.Encore.id) }
            var accent by rememberSaveable { mutableStateOf("") }
            KitTheme(theme = KitTheme.of(theme), accent = accent) {
                Box(Modifier.fillMaxSize().safeDrawingPadding()) {
                    DemoRoot(app, DemoLook(theme, accent, { theme = it }, { accent = it }))
                    ToastHost(bottomPadding = 16.dp)
                }
            }
        }
        KitPerf.jankStatsInDebug(this, BuildConfig.DEBUG)
    }

    /** Back from "Install unknown apps": the updater goes on with an install. */
    override fun onResume() {
        super.onResume()
        Updater.onResume(this)
    }
}

/**
 * The theme picker's state.
 *
 * @property theme the theme's id ([KitTheme.id]).
 * @property accent the accent's id, "" for the theme's own.
 * @property setTheme picks a theme.
 * @property setAccent picks an accent.
 */
class DemoLook(val theme: String, val accent: String, val setTheme: (String) -> Unit, val setAccent: (String) -> Unit)

/** The demo's pages. */
sealed interface Page {
    /** Every part on one screen. */
    data object Home : Page

    /** Settings → What's new. */
    data object Changelog : Page

    /** The bug report list, opened from [from]. */
    data class Bugs(val from: String) : Page

    /** One bug report. */
    data class Bug(val id: Long) : Page

    /** The server's settings, drawn from its schema. */
    data object Settings : Page

    /** Diagnostics, with the app's crash reports. */
    data object Diagnostics : Page

    /** Sign-up: who can sign up, invites and accounts waiting. */
    data object Signup : Page
}

/** What the bug report screens need, from the demo's server and its pages. */
class DemoBugHost(private val app: DemoApp, private val open: (Page) -> Unit, private val close: () -> Unit) : BugReportsHost {
    override val bugReports: BugReportsApi get() = app.bugReports
    override val imageLoader: ImageLoader get() = app.imageLoader

    override fun url(path: String): String = app.server.kit.url(path)

    override fun isAdmin(): Boolean = true

    override fun openList(from: String) = open(Page.Bugs(from))

    override fun openReport(id: Long) = open(Page.Bug(id))

    override fun back() = close()
}

/** The page on top of the stack, the offline banner above it, and the confirm dialogs. */
@Composable
private fun DemoRoot(app: DemoApp, look: DemoLook) {
    val stack = remember { mutableStateListOf<Page>(Page.Home) }
    val open: (Page) -> Unit = { stack.add(it) }
    val close: () -> Unit = { if (stack.size > 1) stack.removeAt(stack.lastIndex) }
    val host = remember(app) { DemoBugHost(app, open, close) }
    val changelog = remember(app) { ApiChangelogBackend(app.server.kit) }
    val offline = remember(app) { DemoOffline(app.server) }
    val confirms = rememberConfirms()
    val scope = rememberCoroutineScope()
    // offline mode's check: now, then every 30 seconds while offline (every minute otherwise)
    LaunchedEffect(offline) {
        while (true) {
            offline.check()
            delay(if (offline.active) 30_000 else 60_000)
        }
    }
    if (stack.size > 1) BackHandler(onBack = close)
    Column(Modifier.fillMaxSize()) {
        OfflineBanner(
            active = offline.active,
            trying = offline.trying,
            title = "Offline",
            line = "What you change waits on the phone · tap for details",
            message = "The demo server isn't answering. What you change is kept on the phone and sent once it answers again.",
            reason = offline.reason.orEmpty(),
            status = offline.status,
            onRetry = { scope.launch { offline.check() } },
        )
        when (val page = stack.last()) {
            Page.Home -> DemoHome(app, look, offline, confirms, changelog, host, open)

            Page.Changelog -> Column(Modifier.fillMaxSize()) {
                BackBar("What's new", close)
                ChangelogScreen(changelog)
            }

            is Page.Bugs -> BugReportsScreen(host, page.from)

            is Page.Bug -> BugReportScreen(host, page.id)

            Page.Settings -> DemoSettings(app, close)

            Page.Diagnostics -> DemoDiagnostics(app, close)

            Page.Signup -> SignupAdminScreen(app.server.kit, DemoServer.BASE, onBack = close)
        }
    }
    ConfirmHost(confirms)
}
