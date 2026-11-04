package dev.krifmaz.kit.compose.uploads

/**
 * Which files an upload takes, checked the same way for the file picker and drag and drop, and the words for those it
 * refuses (the same rules as kit-rules' `uploads`). An upload's rule is an `accept` list like an HTML input's:
 * `.zip,application/zip`, `image/png`, or every image type with `image/` and a star.
 */
object UploadRules {
    /** The toast for a drop on an upload that can't be used right now. */
    const val NOT_POSSIBLE = "Uploading isn't possible right now"

    /** The toast when an upload of one file got several. */
    const val ONLY_ONE = "Only one file can be uploaded here: the first was used"

    /**
     * Whether a file with this [name] and MIME type [mime] passes [accept]: a comma-separated list of `.ext` (the
     * name's ending, any case), `type/` and a star (any subtype) and `type/subtype` (exactly). An empty [accept] takes every
     * file; a file without a type ("") only passes on its name.
     */
    fun accepts(accept: String, name: String, mime: String): Boolean {
        val tokens = accept.split(',').map { it.trim().lowercase() }.filter { it.isNotEmpty() }
        if (tokens.isEmpty()) return true
        val n = name.lowercase()
        val m = mime.trim().lowercase()
        return tokens.any { t ->
            when {
                t.startsWith(".") -> n.endsWith(t)
                t.endsWith("/*") -> m.substringBefore('/', "") == t.removeSuffix("/*") && m.contains('/')
                else -> m.isNotEmpty() && m == t
            }
        }
    }

    /** The extensions [accept] names, for the words about a refused file (".zip, .db"; "" when it names only types). */
    fun acceptHint(accept: String): String = accept.split(',').map { it.trim() }.filter { it.startsWith(".") }.joinToString(", ")

    /**
     * Splits picked or dropped files by [accept]: (the files to hand over, the names of those refused). Without
     * [multiple] only the first acceptable file is kept and every other one counts as refused.
     */
    fun <T> choose(files: List<T>, accept: String, multiple: Boolean, name: (T) -> String, mime: (T) -> String): Pair<List<T>, List<String>> {
        val keep = ArrayList<T>()
        val refused = ArrayList<String>()
        for (f in files) {
            if (accepts(accept, name(f), mime(f)) && (multiple || keep.isEmpty())) keep.add(f) else refused.add(name(f))
        }
        return keep to refused
    }

    /** The toast after files were refused ("" when none were): one by name, several by count, with what it takes. */
    fun refusedMessage(refused: List<String>, accept: String, multiple: Boolean, kept: Int): String {
        if (refused.isEmpty()) return ""
        val hint = acceptHint(accept)
        val takes = if (hint.isEmpty()) "" else " (it takes $hint)"
        return when {
            kept == 1 && !multiple -> ONLY_ONE
            refused.size == 1 -> "${refused[0]} can't be uploaded here$takes"
            else -> "${refused.size} files can't be uploaded here$takes"
        }
    }

    /** The kind of [refusedMessage]'s toast: an error when nothing was kept, else "info". */
    fun refusedKind(kept: Int): String = if (kept == 0) "err" else "info"
}

/**
 * How many files an upload holds and how big each may be.
 *
 * @property maxFiles files per upload.
 * @property maxBytes bytes per file.
 * @property per what the files belong to, in the words ("report": "At most 6 files per report").
 */
data class FileLimits(val maxFiles: Int, val maxBytes: Long, val per: String = "upload") {
    private val mb: Long get() = maxBytes / (1024 * 1024)

    /** The toast when the upload is full: "At most 6 files per report". */
    fun tooMany(): String = "At most $maxFiles files per $per"

    /** The toast for a file over the size: "photo.png is over 8 MB". */
    fun tooBig(name: String): String = "$name is over $mb MB"

    /** The line under the picker: "Up to 6 files, 8 MB each.". */
    fun summary(): String = "Up to $maxFiles files, $mb MB each."

    /**
     * Adds files ((name, size) each) to the [have] already there: the indexes of those taken, and the toasts for the
     * rest. Once full it says [tooMany] and stops; a file over the size is left out with [tooBig].
     */
    fun add(have: Int, files: List<Pair<String, Long>>): Pair<List<Int>, List<String>> {
        val taken = ArrayList<Int>()
        val said = ArrayList<String>()
        for ((i, f) in files.withIndex()) {
            if (have + taken.size >= maxFiles) {
                said.add(tooMany())
                break
            }
            if (f.second > maxBytes) {
                said.add(tooBig(f.first))
                continue
            }
            taken.add(i)
        }
        return taken to said
    }

    /** Limits the kit uses. */
    companion object {
        /** A bug report's pictures: 6 files of 8 MB each. */
        val BUG_REPORT = FileLimits(6, 8L * 1024 * 1024, "report")
    }
}
