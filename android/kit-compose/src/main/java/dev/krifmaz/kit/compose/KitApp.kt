package dev.krifmaz.kit.compose

/**
 * Who the app is, for the words, files and requests the kit's parts make on its behalf. Set once at start-up (in
 * `Application.onCreate`), before any kit screen shows:
 *
 * ```kotlin
 * KitApp.init(name = "Notes", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE)
 * ```
 *
 * Every part reads it when it runs, so nothing the kit says or stores names another app.
 */
object KitApp {
    /** The app's name as users see it ("Notes"): in messages like "Notes needs access to the phone's storage…". */
    @Volatile
    var name: String = "App"
        private set

    /**
     * The app's id: lower-case letters, digits and `_` ("notes"). It prefixes what the kit keeps on the phone (its
     * preferences files, "notes_update") and the names of the files it exports ("notes-bug-reports-2026-11-04.txt").
     */
    @Volatile
    var id: String = "app"
        private set

    /** The installed version name ("1.4.2", or "0.0.1-dev" for a local build). */
    @Volatile
    var versionName: String = "0.0.1-dev"
        private set

    /** The installed version code. */
    @Volatile
    var versionCode: Int = 1
        private set

    /** The User-Agent of the kit's own requests (update checks, downloads) and of bug reports' "device" line. */
    @Volatile
    var userAgent: String = "App-Android"
        private set

    /** The server paths the kit's parts call ([KitRoutes]'s defaults unless the app sets its own). */
    @Volatile
    var routes: KitRoutes = KitRoutes()
        private set

    /**
     * Sets who the app is.
     *
     * @param name the app's name as users see it.
     * @param versionName the installed version name (BuildConfig.VERSION_NAME).
     * @param versionCode the installed version code (BuildConfig.VERSION_CODE).
     * @param userAgent the User-Agent for the kit's requests; "`<Name>`-Android/`<version>`" by default.
     * @param id the app's id for stored files and exports; made from [name] by default ([idOf]).
     * @param routes the server paths; the kit's defaults by default.
     */
    fun init(
        name: String,
        versionName: String,
        versionCode: Int,
        userAgent: String = "${name.filter { it.isLetterOrDigit() }.ifEmpty { "App" }}-Android/$versionName",
        id: String = idOf(name),
        routes: KitRoutes = KitRoutes(),
    ) {
        this.name = name
        this.versionName = versionName
        this.versionCode = versionCode
        this.userAgent = userAgent
        this.id = idOf(id)
        this.routes = routes
    }

    /** An id made from a name: lower case, every run of other characters as one `_`, never empty ("My Notes" → "my_notes"). */
    fun idOf(name: String): String = name.lowercase().replace(Regex("[^a-z0-9]+"), "_").trim('_').ifEmpty { "app" }
}

/**
 * The server paths the kit's parts call. The defaults are the kit server's (contracts/); an app whose server puts
 * them elsewhere passes its own to [KitApp.init]. Every path starts with "/" and has no trailing "/".
 *
 * @property health the health check (`GET`): offline mode's probe.
 * @property changelog the full changelog (`GET`).
 * @property changelogNew what is new since the account last looked (`GET`).
 * @property changelogSeen marks the server's version seen (`POST`).
 * @property prefs the account's preferences (`PUT {"whats_new_popup": on}`).
 * @property notices the notice banners (`GET`).
 * @property bugReports the bug reports (`GET`, `POST`, `/{id}`, `/{id}/messages`, `/{id}/status`, `/unread`, `/export`).
 * @property diagnostics the diagnostics checks (`GET`).
 * @property clientCrash the app crash reports (`GET`, `POST`, `DELETE`).
 * @property settingsSchema the settings schema (`GET`).
 * @property settings the settings' values (`GET`, `PUT`).
 * @property settingsSecret one saved secret, shown to the owner (`GET /{key}`).
 * @property settingsTest a settings card's Test button (`POST`; `{test}` is the card's check id).
 * @property signup the sign-up routes' base (`GET` who can sign up, `POST` a sign-up, `/invite`, `/verify`, `/settings`, `/invites`, `/pending`).
 * @property setup first-run setup (`GET` whether "Create your account" shows, `POST` the owner with the setup code).
 */
data class KitRoutes(
    val health: String = "/api/health",
    val changelog: String = "/api/changelog",
    val changelogNew: String = "/api/changelog/new",
    val changelogSeen: String = "/api/changelog/seen",
    val prefs: String = "/api/auth/prefs",
    val notices: String = "/api/notices",
    val bugReports: String = "/api/bug-reports",
    val diagnostics: String = "/api/diagnostics",
    val clientCrash: String = "/api/crash-reports",
    val settingsSchema: String = "/api/settings/schema",
    val settings: String = "/api/settings",
    val settingsSecret: String = "/api/settings/secret",
    val settingsTest: String = "/api/diagnostics/{test}/test",
    val signup: String = "/api/signup",
    val setup: String = "/api/setup",
)
