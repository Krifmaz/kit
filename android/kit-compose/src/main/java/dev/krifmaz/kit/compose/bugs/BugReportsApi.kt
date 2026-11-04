package dev.krifmaz.kit.compose.bugs

import android.content.ContentResolver
import android.net.Uri
import dev.krifmaz.kit.compose.core.ApiException
import dev.krifmaz.kit.compose.core.Checked
import dev.krifmaz.kit.compose.core.Items
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.core.LBool
import dev.krifmaz.kit.compose.core.LDouble
import dev.krifmaz.kit.compose.core.LInt
import dev.krifmaz.kit.compose.core.LLong
import dev.krifmaz.kit.compose.core.LString
import dev.krifmaz.kit.compose.core.Nested
import dev.krifmaz.kit.compose.core.decodeModel
import dev.krifmaz.kit.compose.core.int
import dev.krifmaz.kit.compose.core.long
import dev.krifmaz.kit.compose.core.parseJsonObject
import dev.krifmaz.kit.compose.core.str
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import okhttp3.MediaType
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okio.BufferedSink
import okio.buffer
import okio.sink
import okio.source
import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

// ---- the answers (server/src/routes/bugreports.rs) ---------------------------------------------------------------

/**
 * A report's or a reply's file: `url` is the server path of GET /api/bug-reports/{id}/files/{fid}.
 *
 * @property id the file's id.
 * @property name its name.
 * @property mime its type.
 * @property size its size in bytes.
 * @property url its server path.
 */
@Serializable
data class BugFile(val id: LLong = 0L, val name: LString = "", val mime: LString = "", val size: LLong = 0L, val url: LString = "") : Checked<BugFile> {
    /** A picture (shown as a square), not a text file. */
    val isImage: Boolean get() = mime.startsWith("image/")

    /** Kept only with a [url]. */
    override fun checked(): BugFile? = if (url.isEmpty()) null else this
}

/** One row of GET /api/bug-reports (the server's fields, read leniently). */
@Serializable
data class BugReportRow(
    val id: LLong = 0L,
    val kind: LString = "",
    val title: LString = "",
    val status: LString = "",
    val username: LString = "",
    @SerialName("user_id") val userId: LLong = 0L,
    @SerialName("created_at") val createdAt: LDouble = 0.0,
    @SerialName("updated_at") val updatedAt: LDouble = 0.0,
    @SerialName("message_count") val messageCount: LInt = 0,
    @SerialName("file_count") val fileCount: LInt = 0,
    val unread: LBool = false,
) : Checked<BugReportRow> {
    override fun checked(): BugReportRow? = if (id <= 0L) null else this
}

/** GET /api/bug-reports/{id} `report` (the server's fields, read leniently). */
@Serializable
data class BugReport(
    val id: LLong = 0L,
    val kind: LString = "",
    val title: LString = "",
    val body: LString = "",
    val page: LString = "",
    @SerialName("app_version") val appVersion: LString = "",
    val device: LString = "",
    val status: LString = "",
    val username: LString = "",
    @SerialName("user_id") val userId: LLong = 0L,
    @SerialName("created_at") val createdAt: LDouble = 0.0,
    @SerialName("updated_at") val updatedAt: LDouble = 0.0,
    @Serializable(with = Items::class) val files: List<BugFile> = emptyList(),
) : Checked<BugReport> {
    override fun checked(): BugReport? = if (id <= 0L) null else this

    internal object OrNull : Nested<BugReport>(serializer())
}

/** One reply of a report's thread (the server's fields, read leniently). */
@Serializable
data class BugMessage(
    val id: LLong = 0L,
    val username: LString = "",
    @SerialName("is_admin") val isAdmin: LBool = false,
    val body: LString = "",
    @SerialName("created_at") val createdAt: LDouble = 0.0,
    @Serializable(with = Items::class) val files: List<BugFile> = emptyList(),
)

/** GET /api/bug-reports/{id}: the report with its files, and the thread. */
@Serializable
data class BugReportDetail(
    @Serializable(with = BugReport.OrNull::class) val report: BugReport? = null,
    @Serializable(with = Items::class) val messages: List<BugMessage> = emptyList(),
)

/** GET /api/bug-reports. */
@Serializable
data class BugReportList(@Serializable(with = Items::class) val reports: List<BugReportRow> = emptyList())

// ---- what is sent ------------------------------------------------------------------------------------------------

/**
 * A picked picture, ready to go: as the phone has it ([bytes] null: read again from [uri] when it is sent), or made
 * smaller as a JPEG ([bytes]). [uri] also draws the thumbnail.
 */
class Attachment(
    /** Where the picture is (also draws the thumbnail). */
    val uri: Uri,
    /** The name it is sent under. */
    val name: String,
    /** Its type. */
    val mime: String,
    /** Its size in bytes. */
    val size: Long,
    /** The smaller JPEG, or null to send [uri] as it is. */
    val bytes: ByteArray?,
) {
    /** The upload's body: [bytes], or the picture streamed from [uri]. */
    fun body(resolver: ContentResolver): RequestBody {
        val type = mime.toMediaTypeOrNull()
        return bytes?.toRequestBody(type) ?: UriBody(resolver, uri, type, size)
    }
}

/** A picture streamed from the photo picker's URI (not held in memory); read again if OkHttp sends the body twice. */
private class UriBody(private val resolver: ContentResolver, private val uri: Uri, private val type: MediaType?, private val length: Long) : RequestBody() {
    override fun contentType(): MediaType? = type
    override fun contentLength(): Long = length
    override fun writeTo(sink: BufferedSink) {
        val input = resolver.openInputStream(uri) ?: throw IOException("A picture can't be read any more")
        input.source().use { sink.writeAll(it) }
    }
}

// ---- the calls ---------------------------------------------------------------------------------------------------

/**
 * The bug report calls (the web's `/api/bug-reports…`). Blocking like [KitApi]: callers run them on
 * Dispatchers.IO. Everything goes through the app's OkHttpClient, so it carries the session.
 *
 * @param api the app's server API (paths, JSON calls).
 * @param client the app's OkHttpClient (carries the session); uploads and exports use a slower copy of it.
 */
class BugReportsApi(private val api: KitApi, private val client: OkHttpClient) {
    /** Uploads and the export take longer than an API call: up to 6 pictures of 8 MB, which the server also decodes. */
    private val slow: OkHttpClient by lazy {
        client.newBuilder().writeTimeout(120, TimeUnit.SECONDS).readTimeout(120, TimeUnit.SECONDS).build()
    }

    /** GET /api/bug-reports with the list's filter. */
    fun list(filter: BugRules.Filter, admin: Boolean): List<BugReportRow> =
        model(getObject("/api/bug-reports" + BugRules.listQuery(filter, admin)), BugReportList.serializer()).reports

    /** GET /api/bug-reports/unread: reports with something this account hasn't seen yet. */
    fun unread(): Int = getObject("/api/bug-reports/unread").int("count")

    /** GET /api/bug-reports/{id} (the server marks it seen). */
    fun detail(id: Long): BugReportDetail {
        val d = model(getObject("/api/bug-reports/$id"), BugReportDetail.serializer())
        if (d.report == null) throw IOException("Unexpected reply from the server")
        return d
    }

    /** POST /api/bug-reports (multipart: kind, title, body, page, app_version, device, file…) → the new report's id. */
    fun create(kind: String, title: String, body: String, page: String, appVersion: String, device: String, files: List<Attachment>, resolver: ContentResolver): Long {
        val form = MultipartBody.Builder().setType(MultipartBody.FORM)
            .addFormDataPart("kind", kind)
            .addFormDataPart("title", title)
            .addFormDataPart("body", body)
            .addFormDataPart("page", page)
            .addFormDataPart("app_version", appVersion)
            .addFormDataPart("device", device)
        for (f in files) form.addFormDataPart("file", f.name, f.body(resolver))
        return send(slow, Request.Builder().url(api.url("/api/bug-reports")).post(form.build())).long("id")
    }

    /** POST /api/bug-reports/{id}/messages (multipart: body, file…). */
    fun reply(id: Long, body: String, files: List<Attachment>, resolver: ContentResolver) {
        val form = MultipartBody.Builder().setType(MultipartBody.FORM).addFormDataPart("body", body)
        for (f in files) form.addFormDataPart("file", f.name, f.body(resolver))
        send(slow, Request.Builder().url(api.url("/api/bug-reports/$id/messages")).post(form.build()))
    }

    /** POST /api/bug-reports/{id}/status {status} (admins). */
    fun setStatus(id: Long, status: String) {
        api.post("/api/bug-reports/$id/status", buildJsonObject { put("status", status) })
    }

    /** DELETE /api/bug-reports/{id} (admins): the report, its replies and its files. */
    fun delete(id: Long) {
        send(client, Request.Builder().url(api.url("/api/bug-reports/$id")).delete())
    }

    /**
     * GET /api/bug-reports/export[query] (admins) into [into]: a plain text file with `format=txt` (the phone's
     * exports), else the zip with reports.md, reports.json and the files.
     * A file left from an earlier export in the same folder is replaced.
     */
    fun export(query: String, into: File): File {
        val req = Request.Builder().url(api.url("/api/bug-reports/export$query")).header("Accept", "text/plain, application/zip, application/json").build()
        slow.newCall(req).execute().use { res ->
            if (!res.isSuccessful) throw failure(res.code, res.body.string())
            into.parentFile?.mkdirs()
            val part = File(into.path + ".part")
            try {
                part.sink().buffer().use { out -> out.writeAll(res.body.source()) }
                if (into.exists()) into.delete()
                if (!part.renameTo(into)) throw IOException("Could not save the export")
            } finally {
                part.delete()
            }
            return into
        }
    }

    /** A text file of a report (.txt, .log), at most [max] characters. */
    fun text(path: String, max: Int = 200_000): String {
        api.rawGet(path).use { res ->
            if (!res.isSuccessful) throw failure(res.code, res.body.string())
            val s = res.body.string()
            return if (s.length > max) s.take(max) + "\n…" else s
        }
    }

    // ---- plumbing ------------------------------------------------------------------------------------------

    private fun getObject(path: String): JsonObject = api.get(path) as? JsonObject ?: throw IOException("Unexpected reply from the server")

    private fun <T> model(o: JsonObject, serializer: KSerializer<T>): T = o.decodeModel(serializer) ?: throw IOException("Unexpected reply from the server")

    /** Sends [req] and reads the JSON answer; the server's `detail` becomes the error's message. */
    private fun send(http: OkHttpClient, req: Request.Builder): JsonObject {
        http.newCall(req.header("Accept", "application/json").build()).execute().use { res ->
            val text = res.body.string()
            if (!res.isSuccessful) throw failure(res.code, text)
            return parseJsonObject(text) ?: JsonObject(emptyMap())
        }
    }

    private fun failure(code: Int, text: String): ApiException {
        val o = parseJsonObject(text)
        return ApiException(code, o?.str("detail")?.ifEmpty { null } ?: "HTTP $code", o)
    }
}

/**
 * The unread count behind the badge on Settings → This device → Bug reports (the web's side bar badge): reports with
 * an answer this person hasn't read, or for admins, reports and replies nobody has looked at yet.
 */
object BugUnread {
    private val _count = MutableStateFlow(0)

    /** The count (0 until the first answer). */
    val count: StateFlow<Int> = _count.asStateFlow()

    /** GET /api/bug-reports/unread; the last count stays when the server can't be reached (or is older). */
    suspend fun refresh(api: BugReportsApi) {
        try {
            _count.value = withContext(Dispatchers.IO) { api.unread() }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
        }
    }
}
