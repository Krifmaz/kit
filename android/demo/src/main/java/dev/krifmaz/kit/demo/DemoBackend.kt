package dev.krifmaz.kit.demo

import dev.krifmaz.kit.compose.KitRoutes
import dev.krifmaz.kit.compose.core.KitJson
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.settings.SchemaRules
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.addJsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject

/** A file sent with a request (a bug report's picture), or kept by the demo server. */
class DemoUpload(
    /** The file's name. */
    val name: String,
    /** Its media type ("image/png"). */
    val mime: String,
    /** What it holds. */
    val bytes: ByteArray,
)

/**
 * One request to the demo server.
 *
 * @property method GET, POST, PUT or DELETE.
 * @property path the path without the query ("/api/bug-reports/3").
 * @property query the query's parameters.
 * @property json the JSON body, when the request sent one.
 * @property form a multipart form's text fields.
 * @property files a multipart form's files.
 */
class DemoRequest(
    val method: String,
    val path: String,
    val query: Map<String, String> = emptyMap(),
    val json: JsonObject? = null,
    val form: Map<String, String> = emptyMap(),
    val files: List<DemoUpload> = emptyList(),
)

/**
 * The demo server's answer.
 *
 * @property code the HTTP status.
 * @property body what it sends.
 * @property mime its media type.
 */
class DemoAnswer(val code: Int, val body: ByteArray, val mime: String = JSON) {
    /** The body as text. */
    val text: String get() = String(body, Charsets.UTF_8)

    /** Ways to make one. */
    companion object {
        /** The media type of a JSON answer. */
        const val JSON = "application/json"

        /** A JSON answer. */
        fun json(o: JsonElement, code: Int = 200): DemoAnswer = DemoAnswer(code, KitJson.encodeToString(JsonElement.serializer(), o).toByteArray(), JSON)

        /** An error with the server's `detail`, which the kit shows as it is. */
        fun error(code: Int, detail: String): DemoAnswer = json(buildJsonObject { put("detail", detail) }, code)
    }
}

/**
 * The server the demo talks to, kept in memory: it answers every route of [KitRoutes] the way a real server would
 * (the shapes of the kit's contracts), so every kit part works with no server and no network. [DemoServer] puts it
 * behind OkHttp. Nothing here needs Android, so the unit tests drive it directly.
 *
 * @param schema the settings schema (`assets/settings-schema.json`).
 * @param version the server's version (the demo's own).
 * @param routes where each call goes.
 * @param now the time in seconds.
 */
class DemoBackend(
    private val schema: String,
    private val version: String,
    private val routes: KitRoutes = KitRoutes(),
    private val now: () -> Double = { System.currentTimeMillis() / 1000.0 },
) {
    /** False: the server is "restarting" and answers every call with 503 (offline mode shows). */
    @Volatile
    var up: Boolean = true

    /** The one notice's problem: false while the mail server "can't be reached" (its banner shows). */
    @Volatile
    var mailOk: Boolean = false

    private val values = LinkedHashMap<String, JsonElement>()
    private val secrets = HashMap<String, String>()
    private val reports = ArrayList<Report>()
    private val crashes = ArrayList<JsonObject>()
    private var nextId = 1L
    private var popup = true
    private var seen = false
    private val signup = DemoSignup(routes.signup, now)

    private class Message(val id: Long, val username: String, val admin: Boolean, val body: String, val at: Double, val files: List<Pair<Long, DemoUpload>>)

    private class Report(val id: Long, val kind: String, val title: String, val body: String, val page: String, val appVersion: String, val device: String, val at: Double) {
        var status = "new"
        var updated = at
        var unread = true
        val files = ArrayList<Pair<Long, DemoUpload>>()
        val messages = ArrayList<Message>()
    }

    init {
        // the settings start at their defaults, with one saved secret to Show and Copy
        for (f in fieldsOfSchema()) {
            val d = f.second
            if (d !is JsonNull) values[f.first] = d
        }
        values["api_url"] = JsonPrimitive("https://notes.example.com")
        values["mail_mode"] = JsonPrimitive("smtp")
        values["smtp_host"] = JsonPrimitive("mail.example.com")
        secrets["smtp_password"] = "correct horse battery staple"
        // two reports, so the list has something in it
        val a = addReport(
            "bug",
            "The save button does nothing",
            "Tapping Save on the profile page shows no toast and nothing changes.",
            "Profile",
            "0.9.0",
            "Pixel 8 · Android 16",
            now() - 86_400 * 2,
        )
        a.status = "looking"
        a.messages += Message(nextId++, "admin", true, "Thanks, I can see it too. Looking into it.", now() - 86_400, emptyList())
        addReport("howto", "How do I change the theme?", "I'd like a lighter look.", "Settings", "0.9.0", "Pixel 8 · Android 16", now() - 3_600)
    }

    /** Answers [r]. */
    fun handle(r: DemoRequest): DemoAnswer = synchronized(this) {
        if (!up) return DemoAnswer.error(503, "The server is restarting")
        try {
            route(r) ?: DemoAnswer.error(404, "Not found: ${r.method} ${r.path}")
        } catch (e: IllegalArgumentException) {
            DemoAnswer.error(400, e.message ?: "Bad request")
        }
    }

    private fun route(r: DemoRequest): DemoAnswer? {
        val p = r.path
        val m = r.method
        return when {
            p == routes.health && m == "GET" -> DemoAnswer.json(
                buildJsonObject {
                    put("ok", true)
                    put("version", version)
                },
            )

            p == "/api/auth/me" && m == "GET" -> DemoAnswer.json(
                buildJsonObject {
                    put("username", "admin")
                    put("role", "owner")
                },
            )

            p.startsWith(routes.changelog) || p == routes.prefs -> changelog(r)

            p.startsWith(routes.notices) -> notices(r)

            p.startsWith(routes.bugReports) -> bugs(r, p.removePrefix(routes.bugReports).trim('/'))

            p == routes.clientCrash -> crashes(r)

            p == routes.signup || p.startsWith(routes.signup + "/") -> signup.route(r)

            p == routes.diagnostics && m == "GET" -> DemoAnswer.json(diagnostics())

            p.startsWith(routes.settingsSchema) && m == "GET" -> DemoAnswer(200, schema.toByteArray())

            p == routes.settings && m == "GET" -> DemoAnswer.json(settings())

            p == routes.settings && m == "PUT" -> DemoAnswer.json(save(r.json ?: JsonObject(emptyMap())))

            p.startsWith(routes.settingsSecret + "/") && m == "GET" -> DemoAnswer.json(buildJsonObject { put("value", secrets[p.substringAfterLast('/')] ?: "") })

            m == "POST" && testOf(p) != null -> DemoAnswer.json(test(testOf(p)!!))

            else -> null
        }
    }

    // ---- What's new ------------------------------------------------------------------------------------------

    private val release: JsonObject = buildJsonObject {
        put("version", version)
        put("title", "Kit demo $version")
        put("summary", "Every part of the kit, on one screen.")
        putJsonArray("highlights") {
            add("Four themes and ten accents")
            add("Toasts that keep errors until they are copied")
            add("Settings drawn from the server's schema")
        }
        putJsonArray("blocks") {
            addJsonObject {
                put("k", "p")
                put("text", "A neutral base for every app: the same look and the same helpers on web, Android and desktop.")
            }
            addJsonObject {
                put("k", "h")
                put("text", "Added")
            }
            addJsonObject {
                put("k", "li")
                put("lead", "Confirm dialogs:")
                put("text", "with a word to type for the ones that can't be undone")
            }
            addJsonObject {
                put("k", "li")
                put("lead", "Notice banners:")
                put("text", "a problem the server found, with Check again")
            }
            addJsonObject {
                put("k", "row")
                putJsonArray("cells") {
                    add("Part")
                    add("Platform")
                }
                put("head", true)
            }
            addJsonObject {
                put("k", "row")
                putJsonArray("cells") {
                    add("kit-compose")
                    add("Android")
                }
            }
        }
    }

    private fun changelog(r: DemoRequest): DemoAnswer? = when {
        r.path == routes.changelogNew && r.method == "GET" -> DemoAnswer.json(
            buildJsonObject {
                put("version", version)
                put("popup", popup)
                put("show", popup && !seen)
                putJsonArray("releases") { add(release) }
            },
        )

        r.path == routes.changelogSeen && r.method == "POST" -> {
            seen = true
            DemoAnswer.json(JsonObject(emptyMap()))
        }

        r.path == routes.changelog && r.method == "GET" -> DemoAnswer.json(
            buildJsonObject {
                put("version", version)
                put("popup", popup)
                putJsonArray("releases") { add(release) }
            },
        )

        r.path == routes.prefs && r.method == "PUT" -> {
            r.json?.get("whats_new_popup")?.let { popup = (it as? JsonPrimitive)?.content == "true" }
            DemoAnswer.json(JsonObject(emptyMap()))
        }

        else -> null
    }

    // ---- notices ---------------------------------------------------------------------------------------------

    private fun noticeList(): JsonObject = buildJsonObject {
        putJsonArray("notices") {
            addJsonObject {
                put("id", "mail")
                put("ok", mailOk)
                put("text", "The mail server can't be reached, so no email goes out.")
                put("severity", "warn")
                put("checking", false)
            }
        }
    }

    private fun notices(r: DemoRequest): DemoAnswer? = when {
        r.path == routes.notices && r.method == "GET" -> DemoAnswer.json(noticeList())

        r.path == routes.notices + "/check" && r.method == "POST" -> {
            // checking again finds it fixed, so the banner goes and the toast says so
            mailOk = true
            DemoAnswer.json(noticeList())
        }

        else -> null
    }

    // ---- bug reports -----------------------------------------------------------------------------------------

    private fun addReport(kind: String, title: String, body: String, page: String, appVersion: String, device: String, at: Double): Report {
        val rep = Report(nextId++, kind, title, body, page, appVersion, device, at)
        reports += rep
        return rep
    }

    private fun fileJson(reportId: Long, f: Pair<Long, DemoUpload>): JsonObject = buildJsonObject {
        put("id", f.first)
        put("name", f.second.name)
        put("mime", f.second.mime)
        put("size", f.second.bytes.size)
        put("url", "${routes.bugReports}/$reportId/files/${f.first}")
    }

    private fun row(rep: Report): JsonObject = buildJsonObject {
        put("id", rep.id)
        put("kind", rep.kind)
        put("title", rep.title)
        put("status", rep.status)
        put("username", "admin")
        put("user_id", 1)
        put("created_at", rep.at)
        put("updated_at", rep.updated)
        put("message_count", rep.messages.size)
        put("file_count", rep.files.size + rep.messages.sumOf { it.files.size })
        put("unread", rep.unread)
    }

    private fun bugs(r: DemoRequest, rest: String): DemoAnswer? {
        val parts = if (rest.isEmpty()) emptyList() else rest.split('/')
        val m = r.method
        if (parts.isEmpty()) {
            return when (m) {
                "GET" -> {
                    val q = r.query["q"].orEmpty().lowercase()
                    val list = reports
                        .filter { rep -> r.query["status"].let { it == null || it == rep.status } }
                        .filter { rep -> r.query["kind"].let { it == null || it == rep.kind } }
                        .filter { rep -> q.isEmpty() || rep.title.lowercase().contains(q) || rep.body.lowercase().contains(q) }
                        .sortedByDescending { it.updated }
                    DemoAnswer.json(buildJsonObject { putJsonArray("reports") { list.forEach { add(row(it)) } } })
                }

                "POST" -> {
                    val title = r.form["title"].orEmpty().trim()
                    require(title.isNotEmpty()) { "A report needs a title" }
                    val f = r.form
                    val rep = addReport(f["kind"] ?: "bug", title, f["body"].orEmpty(), f["page"].orEmpty(), f["app_version"].orEmpty(), f["device"].orEmpty(), now())
                    r.files.forEach { rep.files += nextId++ to it }
                    DemoAnswer.json(buildJsonObject { put("id", rep.id) })
                }

                else -> null
            }
        }
        if (parts[0] == "unread" && m == "GET") return DemoAnswer.json(buildJsonObject { put("count", reports.count { it.unread }) })
        if (parts[0] == "export" && m == "GET") return DemoAnswer(200, export().toByteArray(), "text/plain")
        val id = parts[0].toLongOrNull() ?: return null
        val rep = reports.firstOrNull { it.id == id } ?: return DemoAnswer.error(404, "That report was deleted")
        return when {
            parts.size == 1 && m == "GET" -> {
                rep.unread = false
                DemoAnswer.json(detail(rep))
            }

            parts.size == 1 && m == "DELETE" -> {
                reports.remove(rep)
                DemoAnswer.json(JsonObject(emptyMap()))
            }

            parts.size == 2 && parts[1] == "status" && m == "POST" -> {
                rep.status = r.json?.str("status")?.ifEmpty { null } ?: throw IllegalArgumentException("No status")
                rep.updated = now()
                DemoAnswer.json(JsonObject(emptyMap()))
            }

            parts.size == 2 && parts[1] == "messages" && m == "POST" -> {
                val body = r.form["body"].orEmpty().trim()
                require(body.isNotEmpty() || r.files.isNotEmpty()) { "Write something first" }
                rep.messages += Message(nextId++, "admin", true, body, now(), r.files.map { nextId++ to it })
                rep.updated = now()
                DemoAnswer.json(JsonObject(emptyMap()))
            }

            parts.size == 3 && parts[1] == "files" && m == "GET" -> {
                val fid = parts[2].toLongOrNull()
                val all = rep.files + rep.messages.flatMap { it.files }
                val f = all.firstOrNull { it.first == fid } ?: return DemoAnswer.error(404, "No such file")
                DemoAnswer(200, f.second.bytes, f.second.mime)
            }

            else -> null
        }
    }

    private fun detail(rep: Report): JsonObject = buildJsonObject {
        putJsonObject("report") {
            put("id", rep.id)
            put("kind", rep.kind)
            put("title", rep.title)
            put("body", rep.body)
            put("page", rep.page)
            put("app_version", rep.appVersion)
            put("device", rep.device)
            put("status", rep.status)
            put("username", "admin")
            put("user_id", 1)
            put("created_at", rep.at)
            put("updated_at", rep.updated)
            putJsonArray("files") { rep.files.forEach { add(fileJson(rep.id, it)) } }
        }
        putJsonArray("messages") {
            for (msg in rep.messages) {
                addJsonObject {
                    put("id", msg.id)
                    put("username", msg.username)
                    put("is_admin", msg.admin)
                    put("body", msg.body)
                    put("created_at", msg.at)
                    putJsonArray("files") { msg.files.forEach { add(fileJson(rep.id, it)) } }
                }
            }
        }
    }

    private fun export(): String = reports.joinToString("\n\n") { rep ->
        "#${rep.id} [${rep.kind}, ${rep.status}] ${rep.title}\n${rep.body}" + rep.messages.joinToString("") { "\n  ${it.username}: ${it.body}" }
    }

    // ---- crash reports and diagnostics -----------------------------------------------------------------------

    private fun crashes(r: DemoRequest): DemoAnswer? = when (r.method) {
        "GET" -> DemoAnswer.json(buildJsonObject { putJsonArray("crashes") { crashes.forEach { add(it) } } })

        "POST" -> {
            val b = r.json ?: JsonObject(emptyMap())
            // the newest first, at most 50, as the contract keeps them
            crashes.add(
                0,
                buildJsonObject {
                    put("kind", b.str("kind").ifEmpty { "crash" })
                    put("first_line", b.str("summary").lineSequence().firstOrNull().orEmpty())
                    put("text", b.str("text"))
                    put("time", b["time"] ?: JsonPrimitive(now()))
                    put("device", b.str("device"))
                    put("app_version", b.str("app_version"))
                    put("os", b.str("os"))
                    put("user", "admin")
                },
            )
            while (crashes.size > 50) crashes.removeAt(crashes.lastIndex)
            DemoAnswer.json(JsonObject(emptyMap()))
        }

        "DELETE" -> {
            crashes.clear()
            DemoAnswer.json(JsonObject(emptyMap()))
        }

        else -> null
    }

    private fun diagnostics(): JsonObject = buildJsonObject {
        put("version", version)
        put("uid", 1000)
        put("time", now())
        putJsonObject("machine") {
            put("os", "Linux 6.12 (x86_64)")
            put("cpus", 4)
            put("memory", "7.6 GB")
        }
        putJsonArray("checks") {
            addJsonObject {
                put("kind", "service")
                put("name", "Mail server")
                put("ok", mailOk)
                put("request", "SMTP ${(values["smtp_host"] as? JsonPrimitive)?.content.orEmpty()}:587")
                put("detail", if (mailOk) "Signed in" else "Connection timed out after 10 s")
                put("ms", if (mailOk) 84 else 10_000)
            }
            addJsonObject {
                put("kind", "service")
                put("name", "Update feed")
                put("ok", true)
                put("request", "GET https://api.github.com/repos/…/releases")
                put("status", 200)
                put("detail", "Answered")
                put("ms", 212)
                put("body", "[{\"tag_name\": \"v$version\"}]")
            }
            addJsonObject {
                put("kind", "path")
                put("name", "Data folders:")
                put("ok", true)
                put("request", (values["data_dir"] as? JsonPrimitive)?.content.orEmpty())
                put("detail", "Readable and writable")
            }
            addJsonObject {
                put("kind", "tool")
                put("name", "Image converter")
                put("ok", true)
                put("request", "convert --version")
                put("detail", "7.1.1")
                put("ms", 31)
            }
        }
        putJsonArray("settings") {
            for ((k, v) in values) {
                addJsonObject {
                    put("key", k)
                    put("value", v)
                    if (k == "api_url") {
                        put("source", "env")
                        put("env", "APP_BASE_URL")
                    } else {
                        put("source", "ui")
                    }
                }
            }
        }
    }

    // ---- settings --------------------------------------------------------------------------------------------

    /** (key, default) of every field of the schema, and which keys are secrets. */
    private fun fieldsOfSchema(): List<Triple<String, JsonElement, Boolean>> {
        val root = KitJson.parseToJsonElement(schema).jsonObject
        val out = ArrayList<Triple<String, JsonElement, Boolean>>()
        for (sec in root["sections"]?.jsonArray.orEmpty()) {
            for (g in sec.jsonObject["groups"]?.jsonArray.orEmpty()) {
                for (f in g.jsonObject["fields"]?.jsonArray.orEmpty()) {
                    val o = f.jsonObject
                    out += Triple(o.str("key"), o["default"] ?: JsonNull, o.bool("secret") || o.str("type") == "password")
                }
            }
        }
        return out
    }

    private val secretKeys: Set<String> by lazy { fieldsOfSchema().filter { it.third }.map { it.first }.toSet() }

    private fun settings(): JsonObject = buildJsonObject {
        for ((k, v) in values) if (k !in secretKeys) put(k, v)
        for (k in secretKeys) put(k, if (secrets[k].isNullOrEmpty()) "" else SchemaRules.MASK + " (saved)")
        putJsonObject("_meta") {
            // the public address comes from the environment, as a real server's would
            putJsonObject("api_url") {
                put("source", "env")
                put("env", "APP_BASE_URL")
            }
        }
    }

    private fun save(body: JsonObject): JsonObject {
        val warnings = ArrayList<JsonObject>()
        for ((k, v) in body) {
            if (k == "api_url") continue // set by the environment: saving it does nothing
            if (k in secretKeys) {
                secrets[k] = (v as? JsonPrimitive)?.content.orEmpty()
            } else {
                values[k] = v
            }
            if (k == "data_dir" && !(v as? JsonPrimitive)?.content.orEmpty().startsWith("/data")) {
                warnings += buildJsonObject {
                    put("key", k)
                    put("message", "The server can't write to ${(v as? JsonPrimitive)?.content}")
                }
            }
        }
        val all = settings()
        return if (warnings.isEmpty()) all else JsonObject(all + ("_warnings" to JsonArray(warnings)))
    }

    private fun text(key: String): String = (values[key] as? JsonPrimitive)?.content.orEmpty()

    /** The check id in a settings card's Test path ([KitRoutes.settingsTest] with its `{test}`), or null. */
    private fun testOf(path: String): String? {
        val (before, after) = routes.settingsTest.split("{test}").let { it[0] to it.getOrElse(1) { "" } }
        if (!path.startsWith(before) || !path.endsWith(after) || path.length <= before.length + after.length) return null
        return path.substring(before.length, path.length - after.length)
    }

    private fun test(service: String): JsonObject = when (service) {
        "mail" -> when {
            text("mail_mode") == "off" -> result(false, "Email is off – choose how to send it first")

            text(
                "mail_mode",
            ) == "command" -> result(text("mail_command").isNotBlank(), if (text("mail_command").isBlank()) "No command set" else "Sent a test email with the command")

            text("smtp_host").isBlank() -> result(false, "No mail server set")

            secrets["smtp_password"].isNullOrEmpty() -> result(false, "${text("smtp_host")} refused the sign-in: no password")

            else -> result(true, "Sent a test email through ${text("smtp_host")}")
        }

        "tunnel" -> if (text("tunnel_mode") == "on" && text("tunnel_endpoint").contains(':')) {
            result(true, "Connected to ${text("tunnel_endpoint")}")
        } else {
            result(false, "The tunnel is off or has no endpoint (host:port)")
        }

        else -> throw IllegalArgumentException("No test called $service")
    }

    private fun result(ok: Boolean, message: String): JsonObject = buildJsonObject {
        put("ok", ok)
        put("message", message)
    }

    private fun JsonArray?.orEmpty(): List<JsonElement> = this ?: emptyList()
}
