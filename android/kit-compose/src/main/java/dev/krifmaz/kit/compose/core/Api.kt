package dev.krifmaz.kit.compose.core

import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import okhttp3.Response
import java.io.IOException

/** An HTTP error from the server, with its `detail` message and the JSON body (e.g. `{two_factor: true}`). */
class ApiException(
    /** The HTTP status code. */
    val code: Int,
    message: String,
    /** The server's JSON answer, when it sent one. */
    val body: JsonObject?,
) : IOException(message)

/**
 * The app's server API as the kit's parts need it (bug reports, crash logs): the app implements it with its own
 * client, which carries the session. Every call is blocking (run it on Dispatchers.IO); a failed call throws
 * [ApiException] with the server's `detail`, or an [IOException] when the server can't be reached.
 */
interface KitApi {
    /** The full URL of a server [path] ("/api/…"). */
    fun url(path: String): String

    /** GET [path]; the JSON answer (null for an empty one). */
    fun get(path: String): JsonElement?

    /** POST [body] as JSON to [path]; the JSON answer (null for an empty one). */
    fun post(path: String, body: JsonElement): JsonElement?

    /**
     * PUT [body] as JSON to [path]; the JSON answer (null for an empty one). Apps whose kit parts never PUT may leave
     * it out: the default refuses with an [IOException] the caller shows as it is.
     */
    fun put(path: String, body: JsonElement): JsonElement? = throw IOException("This app can't send PUT $path")

    /** GET [path] as it comes (the caller reads and closes the response). */
    fun rawGet(path: String): Response
}
