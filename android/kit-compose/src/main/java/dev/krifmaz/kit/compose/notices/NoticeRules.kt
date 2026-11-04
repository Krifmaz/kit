package dev.krifmaz.kit.compose.notices

import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.str
import kotlinx.serialization.json.JsonObject

/** How serious a notice is (contracts/notices.md). */
enum class Severity(
    /** The id the server sends. */
    val id: String,
) {
    /** For information (accent). */
    Info("info"),

    /** Something to check (amber); a notice without a severity is this. */
    Warn("warn"),

    /** Something is broken (red). */
    Err("err"),
    ;

    /** Lookup by [id]. */
    companion object {
        /** The severity with this [id]; [Warn] for an unknown or missing one. */
        fun of(id: String?): Severity = entries.firstOrNull { it.id == id } ?: Warn
    }
}

/**
 * One notice, as the server sends it (contracts/notices.md): a plain sentence saying what is wrong and what to check.
 *
 * @property ok false: the banner shows; true (or missing): everything is fine.
 * @property text the sentence the banner shows, as it is ("" for none: no banner).
 * @property checking a check still runs.
 * @property id the notice's id.
 * @property severity how serious it is.
 */
data class Notice(val ok: Boolean = true, val text: String = "", val checking: Boolean = false, val id: String = "", val severity: Severity = Severity.Warn) {
    /** The sentence the banner shows, or null when no banner shows: it shows while the notice fails and has one. */
    val banner: String? get() = if (ok) null else text.ifEmpty { null }

    /** The notice fails (it is asked for at every look then). */
    val failing: Boolean get() = !ok
}

/**
 * The three sentences Check again's toast says for one notice: still failing (warn), still checking (warn) and
 * fixed (ok). The app gives them per notice id ([NoticeRules.checkToast]); the defaults say it generally.
 */
data class CheckWords(
    val failing: String = "Still not fixed",
    val checking: String = "Still checking",
    val fixed: String = "Fixed – all good again",
)

/** The rules and words of notice banners, without Android (the unit tests run them on the JVM). */
object NoticeRules {
    /** The banner's button. */
    const val CHECK_AGAIN = "Check again"

    /** The button while the check runs. */
    const val CHECKING = "Checking…"

    /** How often the notices are looked at, in ms; a failing notice is asked for at every look. */
    const val TICK_MS = 15_000L

    /** While every notice is fine, they are asked for every this many looks (every minute). */
    const val QUIET_TICKS = 4

    /** One notice from its JSON (missing fields read as "fine"). */
    fun noticeOf(o: JsonObject): Notice = Notice(
        ok = if (o.containsKey("ok")) o.bool("ok") else true,
        text = o.str("text"),
        checking = o.bool("checking"),
        id = o.str("id"),
        severity = Severity.of(o.str("severity")),
    )

    /** The notices of `GET <notices>` (`{"notices": [...]}`); a single notice object reads as one. */
    fun noticesOf(o: JsonObject?): List<Notice> = when {
        o == null -> emptyList()
        o.containsKey("notices") -> o.arr("notices").objectsOf().map(::noticeOf)
        o.containsKey("ok") -> listOf(noticeOf(o))
        else -> emptyList()
    }

    /** The notices whose banners show, in the server's order; nothing for someone who is not an admin. */
    fun banners(notices: List<Notice>, admin: Boolean): List<Notice> = if (admin) notices.filter { it.banner != null } else emptyList()

    /** The toast after Check again, and its kind ("warn" or "ok"): still failing, a check that still runs, or fixed. */
    fun checkToast(n: Notice, words: CheckWords = CheckWords()): Pair<String, String> = when {
        !n.ok -> words.failing to "warn"
        n.checking -> words.checking to "warn"
        else -> words.fixed to "ok"
    }

    /** The toast kind of a banner's severity ("info", "warn", "err"). */
    fun kindOf(s: Severity): String = s.id
}

/**
 * When to ask for the notices again: at start, then at every look ([NoticeRules.TICK_MS]) while one fails, else
 * every [NoticeRules.QUIET_TICKS] looks.
 */
class NoticePoll {
    private var ticks = 0

    /** The look every [NoticeRules.TICK_MS]: whether to ask now. [failing]: a notice failed at the last answer. */
    fun tick(failing: Boolean): Boolean {
        ticks += 1
        return failing || ticks % NoticeRules.QUIET_TICKS == 0
    }
}
