package dev.krifmaz.kit.demo

import android.app.Application
import coil3.ImageLoader
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.bugs.BugReportsApi
import dev.krifmaz.kit.compose.bugs.CrashLog
import dev.krifmaz.kit.compose.bugs.CrashLogHost
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.images.buildKitImageLoader
import dev.krifmaz.kit.compose.perf.KitPerf
import dev.krifmaz.kit.compose.update.UpdateRules
import dev.krifmaz.kit.compose.update.UpdateSource
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

/**
 * The demo's process: sets the kit up the way an app does (crash log first, then its name, version and where its
 * updates come from) and holds the in-memory server every screen talks to. It is the [CrashLogHost] too, so a crash
 * from "Crash the app" reaches Diagnostics → App crashes at the next start.
 */
class DemoApp :
    Application(),
    CrashLogHost {
    /** The server on the phone ([DemoServer]). */
    lateinit var server: DemoServer
        private set

    private lateinit var bugs: BugReportsApi

    /** The bug report calls. */
    override val bugReports: BugReportsApi get() = bugs

    /** The image loader for the reports' pictures, through the demo server. */
    val imageLoader: ImageLoader by lazy { buildKitImageLoader(this, server.http) }

    /** Lives as long as the app: crash reports are sent in it. */
    override val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default)

    /** The server's API. */
    override val api: KitApi get() = server.kit

    /** The demo has one phone. */
    override val deviceId: String = "demo-phone"

    /** The demo is always signed in. */
    override suspend fun signedIn(): Boolean = true

    /** Sets up the kit and sends crash reports left from earlier runs. */
    override fun onCreate() {
        CrashLog.install(this)
        KitPerf.strictModeInDebug(BuildConfig.DEBUG)
        super.onCreate()
        KitApp.init(name = NAME, versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE)
        // the updater looks at the kit's own releases; the demo is never published, so it only ever looks
        UpdateRules.source = UpdateSource(owner = "Krifmaz", repo = "kit", apkPrefix = "kit-demo-", releasePackage = "")
        val schema = assets.open("settings-schema.json").bufferedReader().use { it.readText() }
        server = DemoServer(DemoBackend(schema, BuildConfig.VERSION_NAME))
        bugs = BugReportsApi(server.kit, server.http)
        CrashLog.uploadPending(this, this)
    }

    /** The demo's name. */
    companion object {
        /** The app name the kit shows ("Kit demo 0.1.0", "<name>-Android/<version>"). */
        const val NAME = "Kit demo"
    }
}
