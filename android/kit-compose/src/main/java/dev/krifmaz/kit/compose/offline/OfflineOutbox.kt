package dev.krifmaz.kit.compose.offline

import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.long
import dev.krifmaz.kit.compose.core.obj
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.parseJsonObject
import dev.krifmaz.kit.compose.core.str
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.io.File

/**
 * One change made while the server could not be reached, waiting to be sent.
 *
 * @property kind what sort of change it is, as the app names it ("comment", "vote", "rename"…); [Outbox.countsText]
 *   says it with the app's own words.
 * @property key what it is about (an item's id); with [kind] it tells which waiting changes are about the same thing.
 * @property data what the app needs to send it (any JSON object; the kit never reads it).
 * @property at when it was made (Unix seconds).
 */
data class OutAction(val kind: String, val key: String, val data: JsonObject = JsonObject(emptyMap()), val at: Long = 0L)

/**
 * How a new [OutAction] meets the ones already waiting for the same [OutAction.kind] and [OutAction.key].
 */
enum class OutMerge {
    /** Every change counts and is sent in order (the same key at the same second is the same change, kept once). */
    Each,

    /** Only the last change counts: it replaces the one still waiting for the same key (a toggle, a rename). */
    Latest,
}

/**
 * Changes made while the server could not be reached (the offline outbox). It belongs to the account that made
 * them: [owner] is `<server>|<username>` (see [account]), a change by another account drops it first, and it is only
 * ever sent under that account's session. The app decides what a change is ([OutAction]), how it merges
 * ([OutMerge]) and how it is sent; the outbox keeps them, caps them and says how many wait.
 *
 * @property owner the account the changes belong to; "" for none yet.
 * @property actions the changes in the order they were made.
 */
data class Outbox(val owner: String = "", val actions: List<OutAction> = emptyList()) {
    /** How many changes wait. */
    val size: Int get() = actions.size

    /** Nothing waits. */
    val isEmpty: Boolean get() = actions.isEmpty()

    /** A change by `owner` is coming: another account's outbox is dropped first. */
    fun own(owner: String): Outbox = if (this.owner == owner) this else Outbox(owner)

    /** Whether this outbox is [owner]'s (never for an empty owner). */
    fun belongsTo(owner: String): Boolean = owner.isNotEmpty() && this.owner == owner

    /** The waiting changes of one [kind], oldest first. */
    fun of(kind: String): List<OutAction> = actions.filter { it.kind == kind }

    /** How many changes of each kind wait, in the order the kinds first appear. */
    fun counts(): Map<String, Int> = actions.groupingBy { it.kind }.eachCount()

    /**
     * Adds a change. With [OutMerge.Each] the same kind, key and second is kept once; with [OutMerge.Latest] it
     * replaces the one waiting for the same kind and key (nothing changes when it is the same). An empty kind or key
     * is ignored. At most [max] changes of a kind wait: the oldest go first.
     */
    fun add(a: OutAction, merge: OutMerge = OutMerge.Each, max: Int = MAX_PER_KIND): Outbox {
        if (a.kind.isEmpty() || a.key.isEmpty()) return this
        val list =
            when (merge) {
                OutMerge.Each -> {
                    if (actions.any { it.kind == a.kind && it.key == a.key && it.at == a.at }) return this
                    actions + a
                }

                OutMerge.Latest -> {
                    val i = actions.indexOfFirst { it.kind == a.kind && it.key == a.key }
                    when {
                        i < 0 -> actions + a
                        actions[i].data == a.data -> return this
                        else -> actions.toMutableList().also { it[i] = a }
                    }
                }
            }
        val over = list.count { it.kind == a.kind } - max.coerceAtLeast(1)
        if (over <= 0) return copy(actions = list)
        var drop = over
        return copy(actions = list.filter { if (it.kind == a.kind && drop > 0) false.also { drop -= 1 } else true })
    }

    /** What went: each change sent is removed (one that changed meanwhile, with other data, stays). */
    fun forget(sent: List<OutAction>): Outbox = copy(actions = actions.filter { it !in sent })

    /** The document saved in the app's storage ([parse] reads it back). */
    fun toJson(): JsonObject = buildJsonObject {
        put("owner", owner)
        put(
            "actions",
            buildJsonArray {
                for (a in actions) {
                    add(
                        buildJsonObject {
                            put("kind", a.kind)
                            put("key", a.key)
                            put("data", a.data)
                            put("at", a.at)
                        },
                    )
                }
            },
        )
    }

    /** The limits, the saved document and the words. */
    companion object {
        /** At most this many changes of one kind wait. */
        const val MAX_PER_KIND = 5000

        /** The owner of an account's outbox: `<server>|<username>`. */
        fun account(server: String, username: String): String = "${server.trimEnd('/')}|$username"

        /** The saved document; an unreadable one is empty. */
        fun parse(text: String?): Outbox {
            val o = parseJsonObject(text) ?: return Outbox()
            return Outbox(
                owner = o.str("owner"),
                actions =
                o.arr("actions").objectsOf().map {
                    OutAction(it.str("kind"), it.str("key"), it.obj("data") ?: JsonObject(emptyMap()), it.long("at"))
                }.filter { it.kind.isNotEmpty() && it.key.isNotEmpty() },
            )
        }

        /**
         * What waits in words: "3 comments and 1 vote", "1 comment" ("" for none). [nouns] names each kind as
         * (one, many); a kind without a noun is "change"/"changes".
         */
        fun countsText(counts: Map<String, Int>, nouns: Map<String, Pair<String, String>> = emptyMap()): String {
            val parts = counts.filter { it.value > 0 }.map { (kind, n) ->
                val (one, many) = nouns[kind] ?: ("change" to "changes")
                "$n ${if (n == 1) one else many}"
            }
            return when (parts.size) {
                0 -> ""
                1 -> parts[0]
                else -> parts.dropLast(1).joinToString(", ") + " and " + parts.last()
            }
        }

        /** The toast once the server answers again: "Back online – 3 comments sent" ("Back online" when nothing went). */
        fun backOnlineText(sent: Map<String, Int>, nouns: Map<String, Pair<String, String>> = emptyMap()): String {
            val words = countsText(sent, nouns)
            return if (words.isEmpty()) "Back online" else "Back online – $words sent"
        }
    }
}

/** The outbox kept in a file of the app's own storage; every change is written at once. Thread-safe. */
class OutboxFile(private val file: File) {
    private val lock = Any()
    private var doc: Outbox? = null
    private val _changes = MutableStateFlow(0)

    /** Moves on every change, so a screen showing what waits (a list's "waiting to send" note) reads it again. */
    val changes: StateFlow<Int> = _changes.asStateFlow()

    private fun loaded(): Outbox {
        doc?.let { return it }
        val d = try {
            if (file.isFile) Outbox.parse(file.readText()) else Outbox()
        } catch (_: Exception) {
            Outbox()
        }
        doc = d
        return d
    }

    /** The outbox as it is now (read from the file the first time). */
    fun read(): Outbox = synchronized(lock) { loaded() }

    /** Applies `change` and writes the result (kept in memory when the write fails, e.g. a full phone). */
    fun update(change: (Outbox) -> Outbox): Outbox = synchronized(lock) {
        val before = loaded()
        val after = change(before)
        if (after != before) {
            doc = after
            try {
                file.parentFile?.mkdirs()
                val tmp = File(file.parentFile, file.name + ".tmp")
                tmp.writeText(after.toJson().toString())
                if (!tmp.renameTo(file)) {
                    file.delete()
                    tmp.renameTo(file)
                }
            } catch (_: Exception) { /* kept in memory until the next write */ }
            _changes.value = _changes.value + 1
        }
        after
    }
}
