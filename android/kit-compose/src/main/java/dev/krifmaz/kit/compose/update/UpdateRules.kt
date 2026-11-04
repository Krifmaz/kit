package dev.krifmaz.kit.compose.update

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.long
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.str
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull

/**
 * Where an app's releases are: the GitHub repository [owner]/[repo], the APK's file name ([apkPrefix] + version +
 * ".apk") and the package of the release builds ([releasePackage], the only one a release's APK can replace).
 *
 * @property owner the repository's owner ("Krifmaz").
 * @property repo the repository's name ("Encore").
 * @property apkPrefix what an APK's release file name starts with before the version ("Encore-android-").
 * @property releasePackage the application id of the release builds ("app.encore.client").
 */
data class UpdateSource(val owner: String, val repo: String, val apkPrefix: String, val releasePackage: String) {
    /** The sources the kit knows. */
    companion object {
        /** No source set yet: nothing is ever offered ([UpdateRules.isReleaseFile] refuses everything). */
        val NONE: UpdateSource = UpdateSource("", "", "", "")

        /** Encore's releases (Krifmaz/Encore, `Encore-android-<version>.apk`, app.encore.client). */
        val ENCORE: UpdateSource = UpdateSource("Krifmaz", "Encore", "Encore-android-", "app.encore.client")
    }
}

/**
 * The rules of the phone's own update, without Android (the unit tests run them on the JVM). [Updater] does the
 * network, the files and the installing. The app sets [source] once at start-up (and [KitApp] for its name).
 *
 * - The newest release is the newest full release (`v<version>`, [fullRelease]) of the source's release list:
 *   a hotfix (`hotfix-<version>`) can be GitHub's "Latest" but carries no apps. Its Android app is the asset
 *   `<apkPrefix><version>.apk` (Encore: `Encore-android-<version>.apk`). A release made without the apps carries
 *   the last APK over under its own version, so the version is read from the file name, not from the tag.
 * - Versions are MAJOR.MINOR.PATCH, compared by number. A build without a release number (`0.0.1-dev`, `dev.12`)
 *   counts every release as newer, as the server's update card does.
 * - Only files under the project's release downloads are used.
 * - The words are the web's (Settings → This computer, Diagnostics → Updates) and the desktop app's updater.
 */
object UpdateRules {
    /** Where the releases are; [UpdateSource.NONE] until the app sets it. */
    @Volatile
    var source: UpdateSource = UpdateSource.NONE

    /** GitHub's release list of the [source] (newest first). */
    val LATEST: String get() = "https://api.github.com/repos/${source.owner}/${source.repo}/releases?per_page=30"
    private val FULL_TAG = Regex("""^v\d+\.\d+\.\d+$""")

    /** The newest full release (tag v<major>.<minor>.<patch>, not a draft or prerelease) of GitHub's list, newest first. */
    fun fullRelease(list: JsonArray?): JsonObject? = list.objectsOf().firstOrNull {
        !it.bool("draft") && !it.bool("prerelease") && FULL_TAG.matches(it.str("tag_name").trim())
    }

    /** The releases page of the [source]. */
    val RELEASES: String get() = "https://github.com/${source.owner}/${source.repo}/releases"

    /** Every file the updater fetches is under this ([isReleaseFile]). */
    val DOWNLOADS: String get() = "https://github.com/${source.owner}/${source.repo}/releases/download/"
    private val DOWNLOAD_PATH: List<String> get() = listOf(source.owner, source.repo, "releases", "download")

    /** The most an APK download may take when the release does not give its size. */
    const val MAX_APK_BYTES = 300L * 1024 * 1024

    /** The checksum file of a release (release.yml writes SHA256SUMS over every file). */
    val SUMS = listOf("SHA256SUMS", "SHA256SUMS.txt")

    /** Release notes are cut to this many characters, as the server cuts them for its update card. */
    const val NOTES_MAX = 4000

    private val APK: Regex get() = Regex("^" + Regex.escape(source.apkPrefix) + """(\d+\.\d+\.\d+)\.apk$""")
    private val RELEASE_VERSION = Regex("""^\d+\.\d+\.\d+$""")
    private val HEX64 = Regex("""^[0-9a-fA-F]{64}$""")

    /** A release file. `sha256` is GitHub's own digest of the upload (lowercase hex), "" when it has none. */
    data class Asset(val name: String, val url: String, val size: Long, val sha256: String = "")

    /** A newer release: [version] is the APK's, [apk] the file to install, [sums] the release's SHA256SUMS. */
    data class Release(val version: String, val tag: String, val page: String, val notes: String, val notesCut: Boolean, val publishedAt: String, val apk: Asset, val sums: Asset?)

    /** What a look at GitHub's answer found. */
    sealed interface Check {
        /** A newer release for this phone. */
        data class Newer(val release: Release) : Check

        /** Nothing newer. */
        data object None : Check

        /** The check failed; [message] says why. */
        data class Error(val message: String) : Check
    }

    /** Where the updater is. */
    sealed interface Phase {
        /** Nothing going on. */
        data object Idle : Phase

        /** Looking at GitHub. */
        data object Checking : Phase

        /** The last check found nothing newer. */
        data object UpToDate : Phase

        /** [version] can be installed. */
        data class Available(val version: String) : Phase

        /** Downloading [version]: [got] of [total] bytes (0 when unknown). */
        data class Downloading(val version: String, val got: Long, val total: Long) : Phase

        /** Checking the downloaded [version]. */
        data class Verifying(val version: String) : Phase

        /** Checked and ready; Android first needs "Install unknown apps" allowed for the app. */
        data class NeedsPermission(val version: String) : Phase

        /** Handed to Android; `confirm` while Android waits for the person to confirm. */
        data class Installing(val version: String, val confirm: Boolean = false) : Phase

        /** It went wrong; [message] says what. */
        data class Failed(val message: String) : Phase
    }

    /** The updater is at work (no second check or install meanwhile). */
    fun busy(p: Phase): Boolean = p is Phase.Checking || p is Phase.Downloading || p is Phase.Verifying || p is Phase.Installing

    /** The line under Updates for a phase ("" for none); `size` formats a byte count. */
    fun describe(p: Phase, size: (Long) -> String): String = when (p) {
        Phase.Idle -> ""

        Phase.Checking -> "Looking for a new version…"

        Phase.UpToDate -> "${KitApp.name} is up to date."

        is Phase.Available -> "${KitApp.name} ${p.version} is ready to install."

        is Phase.Downloading ->
            if (p.total > 0) {
                "Downloading ${KitApp.name} ${p.version}: ${size(p.got)} of ${size(p.total)}…"
            } else {
                "Downloading ${KitApp.name} ${p.version}: ${size(p.got)}…"
            }

        is Phase.Verifying -> "Checking the download…"

        is Phase.NeedsPermission ->
            "To install ${KitApp.name} ${p.version}, allow ${KitApp.name} to install apps: turn on Allow from this source, then come back here."

        is Phase.Installing ->
            if (p.confirm) {
                "Confirm the update on the screen Android shows. ${KitApp.name} closes while it installs; open it again afterwards."
            } else {
                "Installing ${KitApp.name} ${p.version}. ${KitApp.name} closes while it installs; open it again afterwards."
            }

        is Phase.Failed -> p.message
    }

    /** A plain MAJOR.MINOR.PATCH (a release build); anything else is a build without a release number. */
    fun isReleaseVersion(v: String): Boolean = RELEASE_VERSION.matches(v.trim())

    /** The numeric dot parts of `v` ("v0.30.1" → [0, 30, 1]). */
    private fun parts(v: String): List<Long> = v.trim().trimStart('v', 'V').split('.').map { p -> p.takeWhile { it.isDigit() }.toLongOrNull() ?: 0L }

    /** -1, 0 or 1, by the numeric dot parts (missing parts count as 0). */
    fun compareVersions(a: String, b: String): Int {
        val x = parts(a)
        val y = parts(b)
        for (i in 0 until maxOf(x.size, y.size)) {
            val c = (x.getOrNull(i) ?: 0L).compareTo(y.getOrNull(i) ?: 0L)
            if (c != 0) return c
        }
        return 0
    }

    /** `latest` is newer than the running `current` (a build without a release number: any release is). */
    fun isNewer(latest: String, current: String): Boolean {
        if (!isReleaseVersion(latest)) return false
        if (!isReleaseVersion(current)) return true
        return compareVersions(latest, current) > 0
    }

    /**
     * Whether [url] is a file of the project's releases (`<DOWNLOADS><tag>/<file>`), read as OkHttp will fetch it:
     * a `..` or `%2e%2e` part that leads to another repository, another host or port, or a user name is refused.
     */
    fun isReleaseFile(url: String): Boolean {
        if (source.owner.isEmpty() || source.repo.isEmpty() || !url.startsWith(DOWNLOADS)) return false
        val u = url.toHttpUrlOrNull() ?: return false
        if (u.scheme != "https" || u.host != "github.com" || u.port != 443 || u.username.isNotEmpty() || u.password.isNotEmpty()) return false
        val p = u.pathSegments
        return p.size == DOWNLOAD_PATH.size + 2 && p.subList(0, DOWNLOAD_PATH.size) == DOWNLOAD_PATH &&
            p.drop(DOWNLOAD_PATH.size).all { it.isNotEmpty() && it != "." && it != ".." && '/' !in it && '\\' !in it }
    }

    /** The version in an APK's release file name, null for any other file. */
    fun apkVersion(name: String): String? = APK.matchEntire(name)?.groupValues?.get(1)

    /** GitHub's asset digest ("sha256:<hex>") as lowercase hex, "" when it is missing or another kind. */
    fun digestHex(digest: String): String {
        val hex = digest.trim().removePrefix("sha256:")
        return if (digest.trim().startsWith("sha256:") && HEX64.matches(hex)) hex.lowercase() else ""
    }

    /**
     * The newer release in the [fullRelease] of GitHub's answer ([Check.None] when there is none for this phone),
     * with its APK and checksum file, both only from the project's release downloads.
     */
    fun newest(release: JsonObject, current: String): Check {
        val tag = release.str("tag_name").trim()
        if (tag.isEmpty()) {
            val why = release.str("message").ifBlank { "no release found" }
            return Check.Error("Couldn't check for updates: $why")
        }
        if (release.bool("draft") || release.bool("prerelease")) return Check.None
        val assets = release.arr("assets").objectsOf()
            .map { Asset(it.str("name"), it.str("browser_download_url"), it.long("size"), digestHex(it.str("digest"))) }
            .filter { it.name.isNotEmpty() && isReleaseFile(it.url) }
        // the newest APK the release carries (a release without the apps carries the last one over)
        val apk = assets.mapNotNull { a -> apkVersion(a.name)?.let { v -> v to a } }
            .maxWithOrNull { a, b -> compareVersions(a.first, b.first) }
            ?: return Check.None
        if (!isNewer(apk.first, current)) return Check.None
        val body = release.str("body").replace("\r\n", "\n")
        return Check.Newer(
            Release(
                version = apk.first,
                tag = tag,
                page = release.str("html_url").takeIf { it.startsWith("https://github.com/") } ?: RELEASES,
                notes = body.take(NOTES_MAX).trimEnd(),
                notesCut = body.length > NOTES_MAX,
                publishedAt = release.str("published_at"),
                apk = apk.second,
                sums = assets.firstOrNull { it.name in SUMS },
            ),
        )
    }

    /** What is kept of GitHub's answer (for the ETag's 304): the fields [newest] reads, notes cut. */
    fun trim(release: JsonObject): JsonObject = buildJsonObject {
        put("tag_name", release.str("tag_name"))
        put("html_url", release.str("html_url"))
        put("body", release.str("body").take(NOTES_MAX + 1))
        put("published_at", release.str("published_at"))
        put("draft", release.bool("draft"))
        put("prerelease", release.bool("prerelease"))
        put(
            "assets",
            buildJsonArray {
                for (a in release.arr("assets").objectsOf()) {
                    add(
                        buildJsonObject {
                            put("name", a.str("name"))
                            put("browser_download_url", a.str("browser_download_url"))
                            put("size", a.long("size"))
                            put("digest", a.str("digest"))
                        },
                    )
                }
            },
        )
    }

    /** `SHA256SUMS` (sha256sum format: `<hex>  <name>`, or `<hex> *<name>`) → name → lowercase hex. */
    fun parseSums(text: String): Map<String, String> {
        val out = HashMap<String, String>()
        for (raw in text.lines()) {
            val l = raw.trim()
            val cut = l.indexOfFirst { it.isWhitespace() }
            if (cut <= 0) continue
            val hash = l.substring(0, cut)
            val name = l.substring(cut).trimStart().removePrefix("*").trim().removePrefix("./")
            if (HEX64.matches(hash) && name.isNotEmpty()) out[name] = hash.lowercase()
        }
        return out
    }

    /** Why the file `name` with SHA-256 `hex` may not be installed by the release's SHA256SUMS; null when it may. */
    fun checkSums(sums: String, name: String, hex: String): String? {
        val want = parseSums(sums)[name]
            ?: return "$name is not listed in the release's SHA256SUMS, so it was not installed."
        if (!want.equals(hex, ignoreCase = true)) {
            return "The download does not match the release's SHA256SUMS, so it was not installed (got $hex, expected $want)."
        }
        return null
    }

    /** Why the file may not be installed by GitHub's own digest of the upload; null when it matches or there is none. */
    fun checkDigest(asset: Asset, hex: String): String? {
        if (asset.sha256.isEmpty() || asset.sha256.equals(hex, ignoreCase = true)) return null
        return "The download does not match the checksum GitHub lists for ${asset.name}, so it was not installed."
    }

    /**
     * Whether an APK may replace the installed app: the same signing certificates (SHA-256 of each, as hex), or
     * one new certificate whose proven history (APK Signature Scheme v3 key rotation) holds the installed one.
     * Nothing read on either side never matches.
     */
    fun signersMatch(installed: Set<String>, apk: Set<String>, apkHistory: List<String>): Boolean {
        if (installed.isEmpty() || apk.isEmpty()) return false
        if (installed == apk) return true
        return apk.size == 1 && installed.size == 1 && apkHistory.contains(installed.first())
    }

    /** Bytes as lowercase hex. */
    fun hex(bytes: ByteArray): String {
        val digits = "0123456789abcdef"
        val sb = StringBuilder(bytes.size * 2)
        for (b in bytes) {
            val v = b.toInt() and 0xff
            sb.append(digits[v ushr 4]).append(digits[v and 0x0f])
        }
        return sb.toString()
    }

    // ---- release notes: Markdown as plain text (web-rs pages/diagnostics.rs `notes_text`) ------------------------

    /** Markdown release notes as plain text: headings as lines, bullets as "• ", links as their text. */
    fun notesText(md: String): String {
        var text = md.replace("\r\n", "\n")
        while (true) {
            val a = text.indexOf("<!--")
            if (a < 0) break
            val e = text.indexOf("-->", a)
            text = text.removeRange(a, if (e < 0) text.length else e + 3)
        }
        val out = ArrayList<String>()
        for (raw in text.split('\n')) {
            val body = raw.trimStart()
            val indent = minOf(raw.length - body.length, 8)
            var t = body.trimEnd()
            if (t.startsWith("```") || t.startsWith("~~~")) continue
            if (t.length >= 3 && t.all { it == '-' || it == '*' || it == '_' || it == ' ' } && t.count { it != ' ' } >= 3) t = ""
            if (t.startsWith('#')) t = t.trimStart('#').trimStart()
            while (t.startsWith('>')) t = t.substring(1).trimStart()
            var bullet = false
            for (m in listOf("* ", "- ", "+ ")) {
                if (t.startsWith(m)) {
                    t = t.substring(m.length).trimStart()
                    bullet = true
                    break
                }
            }
            val line = inlineText(t)
            if (line.isBlank()) {
                if (out.isNotEmpty() && out.last().isNotEmpty()) out.add("")
                continue
            }
            out.add(" ".repeat(indent) + (if (bullet) "• " else "") + line.trimEnd())
        }
        while (out.isNotEmpty() && out.last().isEmpty()) out.removeAt(out.size - 1)
        return out.joinToString("\n")
    }

    /** `[text](url)` at `i` (the `[`): its text and the index after the `)`. */
    private fun mdLink(s: String, i: Int): Pair<String, Int>? {
        val close = s.indexOf(']', i + 1)
        if (close < 0 || s.getOrNull(close + 1) != '(') return null
        val end = s.indexOf(')', close + 2)
        if (end < 0) return null
        return s.substring(i + 1, close) to end + 1
    }

    private fun inlineText(s: String): String {
        val out = StringBuilder()
        var i = 0
        while (i < s.length) {
            val at = if (s[i] == '!' && s.getOrNull(i + 1) == '[') i + 1 else i
            val link = if (s[at] == '[') mdLink(s, at) else null
            if (link != null) {
                out.append(inlineText(link.first))
                i = link.second
                continue
            }
            if (s[i] == '<') {
                val end = s.indexOf('>', i + 1)
                if (end > 0) {
                    val inner = s.substring(i + 1, end)
                    if (inner.startsWith("http://") || inner.startsWith("https://")) {
                        out.append(inner)
                        i = end + 1
                        continue
                    }
                }
            }
            if (s[i] != '*' && s[i] != '`') out.append(s[i])
            i++
        }
        return out.toString()
    }
}
