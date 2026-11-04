package dev.krifmaz.kit.demo

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.ApiException
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.core.KitJson
import dev.krifmaz.kit.compose.core.parseJson
import dev.krifmaz.kit.compose.core.parseJsonObject
import dev.krifmaz.kit.compose.core.str
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import okhttp3.Interceptor
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Protocol
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.ResponseBody.Companion.toResponseBody
import okio.Buffer

/**
 * The demo's server on the phone: an OkHttp client whose requests to [BASE] never leave the app – an interceptor
 * hands them to [backend] and turns its answer into a response. Everything the kit sends through OkHttp (bug report
 * uploads, the settings' PUT, the health check, pictures) and through [kit] reaches it the same way.
 *
 * @param backend the server's state and routes.
 */
class DemoServer(val backend: DemoBackend) {
    private val jsonType = "application/json; charset=utf-8".toMediaType()

    /** The client every kit part is given. */
    val http: OkHttpClient = OkHttpClient.Builder()
        .addInterceptor { chain -> answer(chain.request()) ?: chain.proceed(chain.request()) }
        .build()

    /** The [KitApi] the kit's parts take. */
    val kit: KitApi = object : KitApi {
        override fun url(path: String): String = BASE + path

        override fun get(path: String): JsonElement? = call(Request.Builder().url(url(path)).get())

        override fun post(path: String, body: JsonElement): JsonElement? = call(Request.Builder().url(url(path)).post(body.toBody()))

        override fun put(path: String, body: JsonElement): JsonElement? = call(Request.Builder().url(url(path)).put(body.toBody()))

        override fun rawGet(path: String): Response = http.newCall(Request.Builder().url(url(path)).build()).execute()
    }

    /** GET [path], which must answer a JSON object. */
    fun getObject(path: String): JsonObject = kit.get(path) as? JsonObject ?: throw ApiException(500, "Unexpected reply from the server", null)

    /** DELETE [path]. */
    fun delete(path: String): JsonElement? = call(Request.Builder().url(kit.url(path)).delete())

    private fun JsonElement.toBody(): RequestBody = KitJson.encodeToString(JsonElement.serializer(), this).toRequestBody(jsonType)

    private fun call(req: Request.Builder): JsonElement? {
        http.newCall(req.header("User-Agent", KitApp.userAgent).build()).execute().use { res ->
            val text = res.body.string()
            if (!res.isSuccessful) {
                val o = parseJsonObject(text)
                throw ApiException(res.code, o?.str("detail")?.ifEmpty { null } ?: "HTTP ${res.code}", o)
            }
            return parseJson(text)
        }
    }

    /** The answer to [req] when it is for [BASE]; null sends it on to the network. */
    fun answer(req: Request): Response? {
        if (req.url.host != HOST) return null
        val query = (0 until req.url.querySize).associate { req.url.queryParameterName(it) to req.url.queryParameterValue(it).orEmpty() }
        val body = req.body
        val form = HashMap<String, String>()
        val files = ArrayList<DemoUpload>()
        var json: JsonObject? = null
        if (body is MultipartBody) {
            for (part in body.parts) {
                val disposition = part.headers?.get("Content-Disposition").orEmpty()
                val name = Regex("""name="([^"]*)"""").find(disposition)?.groupValues?.get(1) ?: continue
                val file = Regex("""filename="([^"]*)"""").find(disposition)?.groupValues?.get(1)
                val bytes = Buffer().also { part.body.writeTo(it) }.readByteArray()
                if (file != null) {
                    files += DemoUpload(file, part.body.contentType()?.toString() ?: "application/octet-stream", bytes)
                } else {
                    form[name] = String(bytes, Charsets.UTF_8)
                }
            }
        } else if (body != null) {
            json = parseJsonObject(Buffer().also { body.writeTo(it) }.readUtf8())
        }
        val a = backend.handle(DemoRequest(req.method, req.url.encodedPath, query, json, form, files))
        return Response.Builder()
            .request(req)
            .protocol(Protocol.HTTP_1_1)
            .code(a.code)
            .message(if (a.code < 400) "OK" else "Error")
            .body(a.body.toResponseBody(a.mime.toMediaType()))
            .build()
    }

    /** Where the demo server lives. */
    companion object {
        /** The demo server's host name (`.invalid` never resolves, so nothing can reach a real machine). */
        const val HOST = "demo.invalid"

        /** The demo server's address. */
        const val BASE = "https://$HOST"
    }
}
