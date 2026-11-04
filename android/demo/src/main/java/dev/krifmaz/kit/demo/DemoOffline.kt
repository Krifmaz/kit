package dev.krifmaz.kit.demo

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import dev.krifmaz.kit.compose.offline.OutAction
import dev.krifmaz.kit.compose.offline.OutMerge
import dev.krifmaz.kit.compose.offline.Outbox
import dev.krifmaz.kit.compose.offline.probeHealth
import dev.krifmaz.kit.compose.toasts.Toasts
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject

/**
 * Offline mode in the demo: the health check ([probeHealth]) against the demo server, the reason it failed, and an
 * outbox of changes made while offline that go out once the server answers again ("Back online – 2 notes sent").
 *
 * @param server the demo server; "Server down" stops it answering.
 */
@Stable
class DemoOffline(private val server: DemoServer) {
    /** Why the last check failed; null while the server answers. */
    var reason by mutableStateOf<String?>(null)
        private set

    /** A check is on its way. */
    var trying by mutableStateOf(false)
        private set

    /** What waits to be sent. */
    var outbox by mutableStateOf(Outbox(OWNER))
        private set

    /** Offline mode is on. */
    val active: Boolean get() = reason != null

    /** The sheet's status line: what waits ("" for nothing). */
    val status: String get() = Outbox.countsText(outbox.counts(), NOUNS).let { if (it.isEmpty()) "" else "Waiting to be sent: $it." }

    /** Asks the server now; once it answers again, what waited is sent and a toast says so. */
    suspend fun check() {
        if (trying) return
        trying = true
        val r = try {
            withContext(Dispatchers.IO) { probeHealth(server.http, DemoServer.BASE, phoneOnline = true, waitSeconds = WAIT_SECONDS) }
        } finally {
            trying = false
        }
        val was = reason
        reason = r
        if (was != null && r == null) {
            val sent = outbox.counts()
            outbox = outbox.forget(outbox.actions)
            Toasts.show(Outbox.backOnlineText(sent, NOUNS), "ok")
        }
    }

    /** A change made now: saved for later while offline ([key] names what it changes; the same star merges). */
    fun change(kind: String, key: String) {
        if (!active) {
            Toasts.show("Saved", "ok")
            return
        }
        val merge = if (kind == STAR) OutMerge.Latest else OutMerge.Each
        outbox = outbox.add(OutAction(kind, key, buildJsonObject { put("at", JsonPrimitive(System.currentTimeMillis())) }, System.currentTimeMillis()), merge)
        Toasts.show("Saved on the phone – it goes out when the server answers")
    }

    /** The demo's kinds of change and their words. */
    companion object {
        /** A note written offline (each one waits). */
        const val NOTE = "note"

        /** A star set offline (only the last one per item waits). */
        const val STAR = "star"

        /** How each kind is named in "2 notes and 1 star". */
        val NOUNS: Map<String, Pair<String, String>> = mapOf(NOTE to ("note" to "notes"), STAR to ("star" to "stars"))

        private const val OWNER = "${DemoServer.BASE}|admin"
        private const val WAIT_SECONDS = 5L
    }
}
