package dev.krifmaz.kit.demo

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.bugs.BugReportsApi
import dev.krifmaz.kit.compose.bugs.BugRules
import dev.krifmaz.kit.compose.core.ApiException
import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.long
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.parseJsonObject
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.notices.NoticeRules
import dev.krifmaz.kit.compose.offline.probeHealth
import dev.krifmaz.kit.compose.settings.DiagnosticsRules
import dev.krifmaz.kit.compose.settings.SchemaApi
import dev.krifmaz.kit.compose.settings.SchemaRules
import dev.krifmaz.kit.compose.whatsnew.ApiChangelogBackend
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

/**
 * The demo server answers every kit route the way the kit's parts read it: each test drives a kit part (its real
 * calls, through OkHttp) against [DemoServer], so the demo's screens have what they show.
 */
class DemoServerTest {
    private lateinit var server: DemoServer

    @Before
    fun setUp() {
        KitApp.init(name = "Kit demo", versionName = "0.1.0", versionCode = 1)
        val schema = File("src/main/assets/settings-schema.json").readText()
        server = DemoServer(DemoBackend(schema, "0.1.0", now = { 1_800_000_000.0 }))
    }

    @Test
    fun theHealthCheckAnswersUntilTheServerIsDown() {
        assertNull(probeHealth(server.http, DemoServer.BASE, phoneOnline = true, waitSeconds = 1))
        server.backend.up = false
        val why = probeHealth(server.http, DemoServer.BASE, phoneOnline = true, waitSeconds = 1)
        assertNotNull(why)
        assertTrue(why!!, why.contains("503"))
        // every other call fails with the server's words too
        val e = runCatching { server.kit.get(KitApp.routes.notices) }.exceptionOrNull()
        assertEquals("The server is restarting", (e as ApiException).message)
    }

    @Test
    fun requestsToOtherHostsAreNotAnswered() {
        assertNull(server.answer(Request.Builder().url("https://example.com/api/health").build()))
    }

    @Test
    fun whatsNewShowsOnceAndRemembersTheSwitch() {
        val c = ApiChangelogBackend(server.kit)
        val first = c.new()
        assertTrue(first.show)
        assertEquals("0.1.0", first.releases.single().version)
        c.markSeen()
        assertFalse(c.new().show)
        c.setPopup(false)
        assertFalse(c.all().popup)
        assertTrue(c.all().releases.single().blocks.isNotEmpty())
    }

    @Test
    fun theNoticeFailsUntilCheckedAgain() {
        val before = NoticeRules.noticesOf(server.kit.get(KitApp.routes.notices) as JsonObject)
        assertEquals(1, NoticeRules.banners(before, admin = true).size)
        val after = NoticeRules.noticesOf(server.kit.post(KitApp.routes.notices + "/check", JsonObject(emptyMap())) as JsonObject)
        assertTrue(NoticeRules.banners(after, admin = true).isEmpty())
        assertEquals("ok", NoticeRules.checkToast(after.single()).second)
    }

    @Test
    fun bugReportsListFilterReadReplyAndDelete() {
        val api = BugReportsApi(server.kit, server.http)
        val all = api.list(BugRules.Filter(), admin = true)
        assertEquals(2, all.size)
        assertEquals(2, api.unread())
        assertEquals(1, api.list(BugRules.Filter(kind = "howto"), admin = true).size)
        assertEquals(1, api.list(BugRules.Filter(q = "save"), admin = true).size)

        // a new report with a picture, the way the sheet sends it (multipart)
        val form = MultipartBody.Builder().setType(MultipartBody.FORM)
            .addFormDataPart("kind", "bug")
            .addFormDataPart("title", "The list is empty")
            .addFormDataPart("body", "After a restart")
            .addFormDataPart("page", "Home")
            .addFormDataPart("app_version", "0.1.0")
            .addFormDataPart("device", "Test phone")
            .addFormDataPart("file", "shot.png", byteArrayOf(1, 2, 3).toRequestBody("image/png".toMediaType()))
            .build()
        val id = server.http.newCall(Request.Builder().url(server.kit.url(KitApp.routes.bugReports)).post(form).build()).execute().use {
            parseJsonObject(it.body.string())!!.long("id")
        }
        val d = api.detail(id)
        assertEquals("The list is empty", d.report!!.title)
        val pic = d.report!!.files.single()
        assertTrue(pic.isImage)
        assertEquals(3, server.kit.rawGet(pic.url).use { it.body.bytes().size })

        api.setStatus(id, "fixed")
        assertEquals("fixed", api.detail(id).report!!.status)
        val reply = MultipartBody.Builder().setType(MultipartBody.FORM).addFormDataPart("body", "Thanks!").build()
        server.http.newCall(Request.Builder().url(server.kit.url("${KitApp.routes.bugReports}/$id/messages")).post(reply).build()).execute().close()
        assertEquals("Thanks!", api.detail(id).messages.single().body)

        val export = File.createTempFile("export", ".txt")
        api.export(BugRules.exportQuery(BugRules.Filter()), export)
        assertTrue(export.readText().contains("The list is empty"))
        export.delete()

        api.delete(id)
        assertEquals(404, (runCatching { api.detail(id) }.exceptionOrNull() as ApiException).code)
    }

    @Test
    fun settingsLoadSaveRevealAndTest() {
        val api = SchemaApi(server.kit, server.http)
        val schema = api.schema()
        assertEquals(listOf("general", "network"), schema.sections.map { it.id })
        // the saved secret comes masked, and Show asks for it
        assertTrue(SchemaRules.isMasked(api.values().str("smtp_password")))
        assertEquals("correct horse battery staple", api.secret("smtp_password"))
        // the environment sets the public address: saving it changes nothing
        val saved = api.save(
            buildJsonObject {
                put("site_name", "Notes")
                put("api_url", "https://other.example.com")
            },
        )
        assertEquals("Notes", saved.str("site_name"))
        assertEquals("https://notes.example.com", saved.str("api_url"))
        assertTrue(SchemaRules.isLocked(schema.group("site")!!.field("api_url")!!, saved))
        // a folder the server can't write to comes back as a warning
        assertEquals("data_dir", api.save(buildJsonObject { put("data_dir", "/tmp/x") }).arr("_warnings").objectsOf().single().str("key"))
        // Test answers with the line under the card
        assertTrue(api.test("mail").bool("ok"))
        assertFalse(api.test("tunnel").bool("ok"))
        api.save(
            buildJsonObject {
                put("tunnel_mode", "on")
                put("tunnel_endpoint", "vpn.example.com:51820")
            },
        )
        assertTrue(api.test("tunnel").bool("ok"))
    }

    @Test
    fun diagnosticsAndAppCrashes() {
        val d = server.getObject(KitApp.routes.diagnostics)
        val kinds = d.arr("checks").objectsOf().map { it.str("kind") }.toSet()
        assertEquals(DiagnosticsRules.CHECK_KINDS.map { it.first }.toSet(), kinds)
        assertTrue(DiagnosticsRules.report(d).startsWith("Kit demo 0.1.0 diagnostics"))

        server.kit.post(
            KitApp.routes.clientCrash,
            buildJsonObject {
                put("kind", "crash")
                put("summary", "java.lang.IllegalStateException: on purpose\n\tat …")
                put("text", "the whole log")
                put("time", 1_800_000_000.0)
            },
        )
        val c = server.getObject(KitApp.routes.clientCrash).arr("crashes").objectsOf().single()
        assertEquals("java.lang.IllegalStateException: on purpose", c.str("first_line"))
        server.delete(KitApp.routes.clientCrash)
        assertTrue(server.getObject(KitApp.routes.clientCrash).arr("crashes").objectsOf().isEmpty())
    }

    @Test
    fun anUnknownRouteIsA404WithItsName() {
        val e = runCatching { server.kit.get("/api/nothing") }.exceptionOrNull() as ApiException
        assertEquals(404, e.code)
        assertEquals("Not found: GET /api/nothing", e.message)
    }
}
