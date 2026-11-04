package dev.krifmaz.kit.compose.settings

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.KitRoutes
import dev.krifmaz.kit.compose.core.ApiException
import dev.krifmaz.kit.compose.core.Checked
import dev.krifmaz.kit.compose.core.Items
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.core.KitJson
import dev.krifmaz.kit.compose.core.LBool
import dev.krifmaz.kit.compose.core.LString
import dev.krifmaz.kit.compose.core.LTexts
import dev.krifmaz.kit.compose.core.decodeModel
import dev.krifmaz.kit.compose.core.parseJsonObject
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.core.textOf
import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.KSerializer
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.descriptors.PrimitiveKind
import kotlinx.serialization.descriptors.PrimitiveSerialDescriptor
import kotlinx.serialization.descriptors.SerialDescriptor
import kotlinx.serialization.descriptors.nullable
import kotlinx.serialization.encoding.Decoder
import kotlinx.serialization.encoding.Encoder
import kotlinx.serialization.json.JsonDecoder
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import okhttp3.CacheControl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException
import java.net.URLEncoder
import java.util.concurrent.TimeUnit

// The answer of GET <settings schema> ([KitRoutes.settingsSchema], contracts/settings-schema.md): the settings pages
// (sections), the cards on each page (groups) and the fields on each card. Every attribute is always sent; one that
// does not apply is null (read here as "" for words and null for numbers), [] or false. The values are not in it:
// GET <settings> has them, with `_meta.<key>.source` ("env", "ui" or "default") and `_meta.<key>.env`.

/** A number that may be null (a field's `min`, `max`, `step`): null for null, a missing key or anything not a number. */
@OptIn(ExperimentalSerializationApi::class)
internal object NumberOrNull : KSerializer<Double?> {
    override val descriptor: SerialDescriptor = PrimitiveSerialDescriptor("kit.settings.NumberOrNull", PrimitiveKind.DOUBLE).nullable
    override fun deserialize(decoder: Decoder): Double? {
        val e = (decoder as? JsonDecoder)?.decodeJsonElement() ?: return null
        val p = e as? JsonPrimitive ?: return null
        if (p is JsonNull) return null
        return p.content.toDoubleOrNull()?.takeIf { it.isFinite() }
    }
    override fun serialize(encoder: Encoder, value: Double?) {
        if (value == null) encoder.encodeNull() else encoder.encodeDouble(value)
    }
}

/** One choice of a `select` or one box of a `checks` field. Values are always text, also for a number setting. */
@Serializable
data class SchemaOption(val value: LString = "", val label: LString = "")

/** One field of a card. [kind] is the schema's `type`; [rule] its `when`. */
@Serializable
data class SchemaField(
    /** the setting's name in GET/PUT <settings> */
    val key: LString = "",
    /** The field's label. */
    val label: LString = "",
    /** the line under the field ("" for none) */
    val help: LString = "",
    /** text | path | url | password | textarea | number | bool | select | checks (anything else is drawn as text) */
    @SerialName("type") val kind: LString = "text",
    @Serializable(with = Items::class) val options: List<SchemaOption> = emptyList(),
    /** visibility rule, `key=value` or `key!=value` ("" for always shown) */
    @SerialName("when") val rule: LString = "",
    /** the server never sends the value back (passwords, the WireGuard config, pasted cookies) */
    val secret: LBool = false,
    /** the environment variable that sets it ("" for none) */
    val env: LString = "",
    /** that variable is set on this server: the field is shown disabled and saving it does nothing */
    @SerialName("env_locked") val envLocked: LBool = false,
    /** the value when nothing is saved and no environment variable is set; its JSON type is the setting's */
    val default: JsonElement = JsonNull,
    /** what a number counts ("minutes", "MB" …); the label already says it in words */
    val unit: LString = "",
    @Serializable(with = NumberOrNull::class) val min: Double? = null,
    @Serializable(with = NumberOrNull::class) val max: Double? = null,
    /** a number's step (0.1, 0.05); null: whole numbers */
    @Serializable(with = NumberOrNull::class) val step: Double? = null,
    /** takes the card's whole width */
    val full: LBool = false,
    /** for people who know what they are doing (its label says so too): behind "Show advanced" */
    val advanced: LBool = false,
) : Checked<SchemaField> {
    /** A password field or a secret text area: it starts empty, and empty keeps the saved value. */
    val isSecret: Boolean get() = kind == "password" || secret

    override fun checked(): SchemaField? = if (key.isEmpty()) null else this
}

/** A card: a run of fields with a Save button, and a Test button when [test] is set. */
@Serializable
data class SchemaGroup(
    /** The card's id (the key of its results and open state). */
    val id: LString = "",
    /** The card's title. */
    val title: LString = "",
    /** the heading over a run of cards ("Notifications" over the email and push cards); "" for none */
    val part: LString = "",
    /** the line under the title */
    val desc: LString = "",
    /** POST <settings test>/<test>; "" for no Test button */
    val test: LString = "",
    /** the Test button's words */
    @SerialName("test_label") val testLabel: LString = "",
    @Serializable(with = Items::class) val fields: List<SchemaField> = emptyList(),
) : Checked<SchemaGroup> {
    /** The field with this [key], or null. */
    fun field(key: String): SchemaField? = fields.firstOrNull { it.key == key }

    override fun checked(): SchemaGroup? = if (id.isEmpty()) null else this
}

/** A page under Settings → Server. A page with no cards (Auto-download, Contribute, Blocked) is drawn by its own code. */
@Serializable
data class SchemaSection(
    /** The page's id. */
    val id: LString = "",
    /** The page's title. */
    val title: LString = "",
    /** older `?s=` names that open this page */
    val aliases: LTexts = emptyList(),
    @Serializable(with = Items::class) val groups: List<SchemaGroup> = emptyList(),
) : Checked<SchemaSection> {
    override fun checked(): SchemaSection? = if (id.isEmpty()) null else this
}

/** The whole answer of GET <settings schema>, the pages in nav order. */
@Serializable
data class SettingsSchema(@Serializable(with = Items::class) val sections: List<SchemaSection> = emptyList()) {
    /** The page with this [id], or null. */
    fun section(id: String): SchemaSection? = sections.firstOrNull { it.id == id }

    /** The card with this [id] on any page, or null. */
    fun group(id: String): SchemaGroup? = sections.asSequence().flatMap { it.groups.asSequence() }.firstOrNull { it.id == id }

    /**
     * The page that shows [id]: a page's own id, one of its aliases (an old page id that moved), or the id of one
     * of its cards (links that name a card), in that order. Anything else comes back as it is.
     */
    fun pageOf(id: String): String = (
        sections.firstOrNull { it.id == id } ?: sections.firstOrNull { id in it.aliases }
            ?: sections.firstOrNull { s -> s.groups.any { it.id == id } }
        )?.id ?: id
}

/** One line under a card: a Test result, or a warning that came back with Save (a folder the server can't write to). */
data class ResultLine(val ok: Boolean, val message: String)

// ---- the calls ---------------------------------------------------------------------------------------------------

/**
 * The calls behind the schema-drawn cards ([KitRoutes]' settings paths, contracts/settings-schema.md). Blocking
 * like [KitApi]: callers run them on Dispatchers.IO. All are for admins (the secret one for the server's owner).
 *
 * @param api the app's server API (GET, and the server's URL; an empty URL means not signed in).
 * @param client the app's HTTP client, carrying the session, for the PUT, the secret and the tests.
 * @param routes where the settings calls go ([KitRoutes.settingsSchema], [KitRoutes.settings], …).
 */
class SchemaApi(private val api: KitApi, private val client: OkHttpClient, private val routes: KitRoutes = KitApp.routes) {
    private val jsonType = "application/json; charset=utf-8".toMediaType()

    /** A Test can take a while (Test all signs in to every service in turn; Send test email waits for the mail server). */
    private val slow: OkHttpClient by lazy { client.newBuilder().readTimeout(120, TimeUnit.SECONDS).build() }

    /** GET <settings schema>. */
    fun schema(): SettingsSchema = (api.get(routes.settingsSchema) as? JsonObject)?.decodeModel(SettingsSchema.serializer())
        ?: throw IOException("Unexpected reply from the server")

    /** GET <settings>: every value (secrets as "•••• (saved)" or ""), and `_meta` saying where each comes from. */
    fun values(): JsonObject = api.get(routes.settings) as? JsonObject ?: throw IOException("Unexpected reply from the server")

    /**
     * PUT <settings> with the keys to change: the answer is every value again, plus `_warnings`
     * (`[{key, message}]`) when a folder that was saved can't be written to.
     */
    fun save(body: JsonObject): JsonObject {
        val text = KitJson.encodeToString(JsonObject.serializer(), body)
        val req = Request.Builder().url(api.url(routes.settings)).put(text.toRequestBody(jsonType))
        return send(client, req)
    }

    /**
     * GET <settings secret>/{key}: one saved secret in plain text (the server's owner only; the server logs
     * it). Never kept in the app's HTTP cache, also when a proxy rewrites the server's no-store.
     */
    fun secret(key: String): String {
        val req = Request.Builder().url(api.url(routes.settingsSecret + "/" + enc(key))).cacheControl(CacheControl.Builder().noStore().build()).get()
        val o = send(client, req)
        return o["value"].textOf()
    }

    /** POST <settings test>/{service}: `{ok, message}`. */
    fun test(service: String): JsonObject {
        val req = Request.Builder().url(api.url(routes.settingsTest + "/" + enc(service))).post("".toRequestBody(jsonType))
        return send(slow, req)
    }

    /** Sends [req] and reads the JSON answer; the server's `detail` becomes the error's message. */
    private fun send(http: OkHttpClient, req: Request.Builder): JsonObject {
        if (api.url("").isEmpty()) throw IOException("Not signed in")
        http.newCall(req.header("Accept", "application/json").build()).execute().use { res ->
            val text = res.body.string()
            if (!res.isSuccessful) {
                val o = parseJsonObject(text)
                throw ApiException(res.code, o?.str("detail")?.ifEmpty { null } ?: "HTTP ${res.code}", o)
            }
            return parseJsonObject(text) ?: JsonObject(emptyMap())
        }
    }

    /** A path segment, URL-encoded (spaces as %20). */
    private fun enc(s: String): String = URLEncoder.encode(s, "UTF-8").replace("+", "%20")
}
