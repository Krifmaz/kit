package dev.krifmaz.kit.compose

/**
 * Who the app is, for the words and requests the kit's parts make on its behalf. Set once at start-up (in
 * `Application.onCreate`), before any kit screen shows:
 *
 * ```kotlin
 * KitApp.init(name = "Encore", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE,
 *     userAgent = "Encore-Android/${BuildConfig.VERSION_NAME}")
 * ```
 */
object KitApp {
    /** The app's name as users see it ("Encore"): in messages like "Encore needs access to the phone's storage…". */
    @Volatile
    var name: String = "App"
        private set

    /** The installed version name ("0.31.2", or "0.0.1-dev" for a local build). */
    @Volatile
    var versionName: String = "0.0.1-dev"
        private set

    /** The installed version code. */
    @Volatile
    var versionCode: Int = 1
        private set

    /** The User-Agent of the kit's own requests (update checks, downloads) and of bug reports' "device" line. */
    @Volatile
    var userAgent: String = "Kit-Android"
        private set

    /**
     * Sets who the app is.
     *
     * @param name the app's name as users see it.
     * @param versionName the installed version name (BuildConfig.VERSION_NAME).
     * @param versionCode the installed version code (BuildConfig.VERSION_CODE).
     * @param userAgent the User-Agent for the kit's requests.
     */
    fun init(name: String, versionName: String, versionCode: Int, userAgent: String) {
        this.name = name
        this.versionName = versionName
        this.versionCode = versionCode
        this.userAgent = userAgent
    }
}
