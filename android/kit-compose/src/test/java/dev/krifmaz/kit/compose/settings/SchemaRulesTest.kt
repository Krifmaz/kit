package dev.krifmaz.kit.compose.settings

import dev.krifmaz.kit.compose.core.KitJson
import dev.krifmaz.kit.compose.settings.SchemaRules.Note
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

class SchemaRulesTest {
    // one card of the answer as Config::schema writes it (docs/contracts/settings-schema.md): every attribute present
    private val mail = """
        {"sections":[{"id":"connections","title":"Connections","aliases":["storage","sharing"],"groups":[
          {"id":"mail","title":"Email","part":"Notifications","desc":"Sends the app's emails",
           "test":"mail","test_label":"Test connection","fields":[
            {"key":"mail_mode","label":"How to send","help":null,"type":"select",
             "options":[{"value":"smtp","label":"SMTP"},{"value":"api","label":"Mail API"}],
             "when":null,"secret":false,"env":"MAIL_MODE","env_locked":false,"default":"smtp","unit":null,
             "min":null,"max":null,"step":null,"full":false,"advanced":false},
            {"key":"smtp_password","label":"SMTP password","help":null,"type":"password","options":[],
             "when":"mail_mode=smtp","secret":true,"env":"SMTP_PASSWORD","env_locked":false,"default":"",
             "unit":null,"min":null,"max":null,"step":null,"full":false,"advanced":false},
            {"key":"api_url","label":"Mail API address","help":"Where the mail API answers","type":"url",
             "options":[],"when":"mail_mode=api","secret":false,"env":null,"env_locked":false,"default":"",
             "unit":null,"min":null,"max":null,"step":null,"full":false,"advanced":false},
            {"key":"smtp_port","label":"Port","help":null,"type":"number","options":[],"when":null,
             "secret":false,"env":"SMTP_PORT","env_locked":false,"default":2234,"unit":null,"min":1,"max":65535,
             "step":null,"full":false,"advanced":false},
            {"key":"send_rate","label":"Emails per second","help":null,"type":"number","options":[],
             "when":null,"secret":false,"env":"SEND_RATE","env_locked":true,"default":1.0,"unit":"emails per second",
             "min":0,"max":null,"step":0.1,"full":false,"advanced":false},
            {"key":"notify_on","label":"Send an email for","help":null,"type":"checks",
             "options":[{"value":"Signup","label":"Signup"},{"value":"Report","label":"Report"},{"value":"Weeklydigest","label":"Weekly digest"}],
             "when":null,"secret":false,"env":null,"env_locked":false,"default":"Signup,Report","unit":null,"min":null,"max":null,
             "step":null,"full":true,"advanced":false},
            {"key":"auto_send","label":"Send without asking","help":null,"type":"bool","options":[],"when":null,
             "secret":false,"env":null,"env_locked":false,"default":false,"unit":null,"min":null,"max":null,"step":null,
             "full":false,"advanced":false},
            {"key":"mail_command","label":"Command (advanced)","help":null,"type":"text","options":[],
             "when":null,"secret":false,"env":null,"env_locked":false,"default":"sendmail","unit":null,"min":null,"max":null,
             "step":null,"full":true,"advanced":true,"some_later_attribute":{"x":1}}
          ]}]},
          {"id":"about","title":"About","aliases":[],"groups":[]}]}
    """.trimIndent()

    private val schema: SettingsSchema = KitJson.decodeFromString(SettingsSchema.serializer(), mail)
    private val card: SchemaGroup = schema.group("mail")!!
    private fun f(key: String): SchemaField = card.field(key)!!

    // GET <settings>: values with `_meta`
    private val values: JsonObject = KitJson.parseToJsonElement(
        """
        {"mail_mode":"smtp","smtp_password":"•••• (saved)","api_url":"","smtp_port":2234,
         "send_rate":1.0,"notify_on":"Signup,Report","auto_send":false,"mail_command":"sendmail",
         "_meta":{"mail_mode":{"source":"ui","env":"MAIL_MODE"},
                  "smtp_password":{"source":"ui","env":"SMTP_PASSWORD"},
                  "api_url":{"source":"default","env":null},
                  "smtp_port":{"source":"default","env":"SMTP_PORT"},
                  "send_rate":{"source":"env","env":"SEND_RATE"},
                  "notify_on":{"source":"default","env":null},
                  "auto_send":{"source":"env","env":"APP_AUTO_SEND"},
                  "mail_command":{"source":"default","env":null}}}
        """,
    ).jsonObject

    @Test
    fun theAnswerDecodesWithEveryAttribute() {
        assertEquals(listOf("connections", "about"), schema.sections.map { it.id })
        assertEquals(listOf("storage", "sharing"), schema.section("connections")!!.aliases)
        assertTrue(schema.section("about")!!.groups.isEmpty())
        assertEquals("Notifications", card.part)
        assertEquals("mail", card.test)
        assertEquals("Test connection", SchemaRules.testLabel(card))
        val pw = f("smtp_password")
        assertEquals("password", pw.kind)
        assertEquals("mail_mode=smtp", pw.rule)
        assertTrue(pw.secret && pw.isSecret)
        assertEquals("", pw.help) // null reads as ""
        assertEquals("", f("api_url").env)
        val port = f("smtp_port")
        assertEquals(1.0, port.min!!, 0.0)
        assertEquals(65535.0, port.max!!, 0.0)
        assertNull(port.step)
        assertEquals(JsonPrimitive(2234), port.default)
        assertEquals(0.1, f("send_rate").step!!, 0.0)
        assertTrue(f("send_rate").envLocked)
        assertEquals(listOf("Signup", "Report", "Weeklydigest"), f("notify_on").options.map { it.value })
        assertTrue(f("mail_command").advanced) // and an attribute this app doesn't know is skipped
        assertEquals(JsonNull, SchemaField(key = "x").default)
    }

    @Test
    fun linksOpenThePageOfAnAliasOrACard() {
        // storage is an alias of the Connections page
        assertEquals("connections", schema.pageOf("connections"))
        assertEquals("connections", schema.pageOf("storage"))
        assertEquals("connections", schema.pageOf("mail"))
        assertEquals("users", schema.pageOf("users"))
    }

    @Test
    fun aPageIdWinsOverAnAliasOrACardOfTheSameName() {
        // Files holds a card called files too; a page's own id is looked up first, then aliases, then cards
        val split = KitJson.decodeFromString(
            SettingsSchema.serializer(),
            """
            {"sections":[
              {"id":"general","title":"General","aliases":["files"],"groups":[{"id":"storage","title":"x"}]},
              {"id":"files","title":"Files","aliases":["sharing"],"groups":[{"id":"quota","title":"y"}]},
              {"id":"storage","title":"Storage","aliases":["connections"],"groups":[{"id":"sharing","title":"z"}]}]}
            """.trimIndent(),
        )
        assertEquals("files", split.pageOf("files"))
        assertEquals("storage", split.pageOf("storage"))
        assertEquals("files", split.pageOf("sharing"))
        assertEquals("storage", split.pageOf("connections"))
        assertEquals("files", split.pageOf("quota"))
    }

    @Test
    fun whenRulesParseAndFollowTheUnsavedSelect() {
        assertEquals(SchemaRules.When("mail_mode", "smtp", true), SchemaRules.parseWhen("mail_mode=smtp"))
        assertEquals(SchemaRules.When("backup_mode", "none", false), SchemaRules.parseWhen("backup_mode!=none"))
        assertEquals(SchemaRules.When("a", "b", true), SchemaRules.parseWhen(" a = b "))
        for (bad in listOf(null, "", "   ", "novalue", "=x", "k=", "k!=")) assertNull(bad, SchemaRules.parseWhen(bad))
        assertTrue(SchemaRules.When("k", "v", true).shows("v"))
        assertFalse(SchemaRules.When("k", "v", true).shows("w"))
        assertTrue(SchemaRules.When("k", "v", false).shows("w"))
        assertFalse(SchemaRules.When("k", "v", false).shows("v"))

        val drafts = HashMap<String, String>()
        val shown = { key: String -> SchemaRules.isShown(f(key)) { k -> SchemaRules.current(k, values, drafts) } }
        assertTrue(shown("smtp_password"))
        assertFalse(shown("api_url"))
        assertTrue(shown("smtp_port")) // no rule
        drafts["mail_mode"] = "api" // chosen, not saved yet
        assertFalse(shown("smtp_password"))
        assertTrue(shown("api_url"))
    }

    @Test
    fun valuesReadAsJavaScriptReadsThem() {
        assertEquals("2", SchemaRules.textOf(JsonPrimitive(2)))
        assertEquals("1", SchemaRules.textOf(KitJson.parseToJsonElement("1.0")))
        assertEquals("0.85", SchemaRules.textOf(KitJson.parseToJsonElement("0.85")))
        assertEquals("true", SchemaRules.textOf(JsonPrimitive(true)))
        assertEquals("", SchemaRules.textOf(JsonNull))
        assertEquals("", SchemaRules.textOf(null))
        assertEquals("007", SchemaRules.textOf(JsonPrimitive("007")))
        assertFalse(SchemaRules.truthy(JsonPrimitive("")))
        assertFalse(SchemaRules.truthy(JsonPrimitive(0)))
        assertFalse(SchemaRules.truthy(JsonPrimitive(false)))
        assertTrue(SchemaRules.truthy(JsonPrimitive("•••• (saved)")))
        assertTrue(SchemaRules.truthy(JsonPrimitive("0"))) // a non-empty string, as in JavaScript

        assertEquals("", SchemaRules.initialText(f("smtp_password"), values)) // a secret starts empty
        assertEquals("false", SchemaRules.initialText(f("auto_send"), values))
        assertEquals("2234", SchemaRules.initialText(f("smtp_port"), values))
        assertEquals("1", SchemaRules.initialText(f("send_rate"), values))
    }

    @Test
    fun numbersParseAsTyped() {
        assertEquals(5.0, SchemaRules.parseNumber(" 5 ")!!, 0.0)
        assertEquals(0.5, SchemaRules.parseNumber("0,5")!!, 0.0) // a decimal comma
        assertEquals(-2.5, SchemaRules.parseNumber("-2.5")!!, 0.0)
        assertEquals(1000.0, SchemaRules.parseNumber("1e3")!!, 0.0)
        for (bad in listOf("", "  ", "abc", "1.2.3", "1,2,3", "1,5.0", "NaN", "Infinity", "0x10", "5 MB")) {
            assertNull(bad, SchemaRules.parseNumber(bad))
        }
        assertNull(SchemaRules.numberError(""))
        assertNull(SchemaRules.numberError("12"))
        assertEquals("Enter a number", SchemaRules.numberError("twelve"))
        assertEquals(JsonPrimitive(65535L), SchemaRules.numberJson(65535.0))
        assertEquals(JsonPrimitive(0.25), SchemaRules.numberJson(0.25))
        assertNull(SchemaRules.rangeNote(f("smtp_port"), "2234"))
        assertEquals("Usually 1 to 65535", SchemaRules.rangeNote(f("smtp_port"), "70000"))
        assertEquals("Usually 0 or more", SchemaRules.rangeNote(f("send_rate"), "-1"))
        assertNull(SchemaRules.rangeNote(f("smtp_port"), "x"))
        assertTrue(SchemaRules.needsDecimalKeys(f("send_rate")))
        assertFalse(SchemaRules.needsDecimalKeys(f("smtp_port")))
    }

    @Test
    fun saveSendsOnlyWhatChanged() {
        val fields = card.fields
        val drafts = HashMap<String, String>()
        assertEquals(JsonObject(emptyMap()), SchemaRules.changes(fields, values, drafts))

        // typed back to what is saved: nothing to send
        drafts["mail_mode"] = "smtp"
        drafts["smtp_port"] = "2234.0"
        drafts["notify_on"] = "Report,Signup"
        drafts["api_url"] = ""
        drafts["mail_command"] = "sendmail"
        assertEquals(JsonObject(emptyMap()), SchemaRules.changes(fields, values, drafts))

        drafts["mail_mode"] = "api"
        drafts["api_url"] = "http://mail:8025" // hidden until the select changed; saved with its card
        drafts["smtp_port"] = "50300"
        drafts["notify_on"] = "Signup,Report,Weeklydigest"
        drafts["mail_command"] = "sendmail -t" // an advanced field behind Show advanced is still saved
        val body = SchemaRules.changes(fields, values, drafts)
        assertEquals(
            listOf("mail_mode", "api_url", "smtp_port", "notify_on", "mail_command"),
            body.keys.toList(),
        ) // the card's order
        assertEquals(JsonPrimitive("api"), body["mail_mode"])
        assertEquals(JsonPrimitive(50300L), body["smtp_port"]) // a number as a number
        assertEquals(JsonPrimitive("Signup,Report,Weeklydigest"), body["notify_on"]) // checks as comma-separated text
    }

    @Test
    fun switchesChecksAndNumbers() {
        val open = values.toMutableMap().also { it["_meta"] = JsonObject(emptyMap()) }.let(::JsonObject)
        val sw = f("auto_send")
        assertNull(SchemaRules.changedValue(sw, open, "false"))
        assertEquals(JsonPrimitive(true), SchemaRules.changedValue(sw, open, SchemaRules.textFor(true)))

        // checks compare without case, spaces and punctuation: a saved "Weekly digest" ticks "Weeklydigest"
        val rt = f("notify_on")
        val saved = JsonObject(mapOf("notify_on" to JsonPrimitive("signup, Weekly digest")))
        assertEquals(setOf("Signup", "Weeklydigest"), SchemaRules.ticked(rt, "signup, Weekly digest"))
        assertNull(SchemaRules.changedValue(rt, saved, "Signup,Weeklydigest"))
        assertEquals("Signup,Report", SchemaRules.toggle(rt, "Report", "Signup", true)) // in the options' order
        assertEquals("Report", SchemaRules.toggle(rt, "Signup,Report", "Signup", false))
        assertEquals(JsonPrimitive(""), SchemaRules.changedValue(rt, saved, ""))

        // an emptied number sends "" (the server's default); one that doesn't parse sends nothing
        val port = f("smtp_port")
        assertEquals(JsonPrimitive(""), SchemaRules.changedValue(port, open, " "))
        assertNull(SchemaRules.changedValue(port, open, "lots"))
        assertEquals(listOf("smtp_port"), SchemaRules.invalid(card.fields, open, mapOf("smtp_port" to "lots")).map { it.key })
        assertTrue(SchemaRules.invalid(card.fields, open, mapOf("smtp_port" to "")).isEmpty())
        assertEquals(JsonPrimitive(0.5), SchemaRules.changedValue(port, open, "0,5"))
    }

    @Test
    fun secretsKeepTheSavedValue() {
        val pw = f("smtp_password")
        assertNull(SchemaRules.changedValue(pw, values, null))
        assertNull(SchemaRules.changedValue(pw, values, ""))
        assertNull(SchemaRules.changedValue(pw, values, "   ")) // empty keeps the saved one
        assertNull(SchemaRules.changedValue(pw, values, "•••• (saved)")) // never echo the mask
        assertEquals(JsonPrimitive("hunter2"), SchemaRules.changedValue(pw, values, "  hunter2 ")) // trimmed
        // what Show put in the field goes back only when it was changed
        assertNull(SchemaRules.changedValue(pw, values, "hunter2", revealed = "hunter2"))
        assertEquals(JsonPrimitive("hunter3"), SchemaRules.changedValue(pw, values, "hunter3", revealed = "hunter2"))
    }

    @Test
    fun environmentLocksAField() {
        // _meta says env (auto_send), or the schema does (send_rate)
        assertTrue(SchemaRules.isLocked(f("auto_send"), values))
        assertTrue(SchemaRules.isLocked(f("send_rate"), values))
        assertFalse(SchemaRules.isLocked(f("smtp_port"), values))
        assertNull(SchemaRules.changedValue(f("auto_send"), values, "true"))
        assertNull(SchemaRules.changedValue(f("send_rate"), values, "3"))
        assertTrue(SchemaRules.invalid(card.fields, values, mapOf("send_rate" to "x")).isEmpty())
        assertEquals("APP_AUTO_SEND", SchemaRules.envOf(f("auto_send"), values)) // _meta first
        // the hand-written pages read the same `_meta` by key
        assertEquals("env", SchemaRules.source(values, "auto_send"))
        assertEquals("APP_AUTO_SEND", SchemaRules.envVar(values, "auto_send"))
        assertEquals("", SchemaRules.envVar(values, "notify_on"))
        assertEquals("", SchemaRules.envVar(values, "no_such_key"))
        assertEquals("set by environment", SchemaRules.passwordPlaceholder(f("smtp_password").copy(envLocked = true), values))
    }

    @Test
    fun theNoteUnderAField() {
        assertEquals(Note.LOCKED, SchemaRules.note(f("auto_send"), values))
        assertEquals(Note.SAVED_SECRET, SchemaRules.note(f("smtp_password"), values))
        val unset = JsonObject(values + ("smtp_password" to JsonPrimitive("")))
        assertEquals(Note.UNSET_SECRET, SchemaRules.note(f("smtp_password"), unset))
        assertEquals(Note.NONE, SchemaRules.note(f("mail_mode"), values)) // saved in Settings
        assertEquals(Note.DEFAULT_ENV, SchemaRules.note(f("smtp_port"), values))
        assertEquals(Note.NONE, SchemaRules.note(f("api_url"), values)) // no variable

        assertEquals("saved – paste a new value to replace", SchemaRules.passwordPlaceholder(f("smtp_password"), values))
        assertEquals("not set", SchemaRules.passwordPlaceholder(f("smtp_password"), unset))
        assertTrue(SchemaRules.canShow(f("smtp_password"), values, canReveal = true))
        assertFalse(SchemaRules.canShow(f("smtp_password"), values, canReveal = false))
        assertFalse(SchemaRules.canShow(f("smtp_password"), unset, canReveal = true))
        assertFalse(SchemaRules.canShow(f("api_url"), values, canReveal = true)) // not a secret
        val wg = SchemaField(key = "tunnel_config", kind = "textarea", secret = true)
        val wgSaved = JsonObject(mapOf(wg.key to JsonPrimitive("•••• (saved)")))
        assertTrue(SchemaRules.canShow(wg, wgSaved, canReveal = true))
        assertFalse(SchemaRules.canShow(wg, wgSaved, canReveal = true, neverShown = setOf("tunnel_config"))) // the app's never-shown secret
        assertEquals("Saved – paste to replace", SchemaRules.textareaPlaceholder(wg, JsonObject(mapOf(wg.key to JsonPrimitive("•••• (saved)")))))
    }

    @Test
    fun cardsAndAFieldBlock() {
        // a field the server's values lack is left out; a card with none left goes
        val few = JsonObject(mapOf("mail_mode" to JsonPrimitive("smtp"), "auto_send" to JsonPrimitive(false)))
        val cards = SchemaRules.visibleCards(schema.section("connections")!!, few)
        assertEquals(listOf("mail"), cards.map { it.first.id })
        assertEquals(listOf("mail_mode", "auto_send"), cards[0].second.map { it.key })
        assertTrue(SchemaRules.visibleCards(schema.section("connections")!!, JsonObject(emptyMap())).isEmpty())
        assertTrue(SchemaRules.hasAdvanced(card.fields))

        val k = { key: String -> SchemaField(key = key) }
        val keys = listOf(k("sync_enabled"), k("tunnel_enabled"), k("tunnel_config"), k("sync_interval"), k("tunnel_kill_switch"))
        val l = SchemaRules.layout(keys, "tunnel_")
        assertEquals(listOf("sync_enabled"), l.before.map { it.key })
        assertEquals(listOf("tunnel_enabled", "tunnel_config", "tunnel_kill_switch"), l.block.map { it.key })
        assertEquals(listOf("sync_interval"), l.after.map { it.key })
        // no prefix, or none of its keys: every field in order, no block
        for (none in listOf(SchemaRules.layout(keys), SchemaRules.layout(listOf(k("a"), k("b")), "tunnel_"))) {
            assertTrue(none.block.isEmpty() && none.after.isEmpty())
        }
        assertEquals(listOf("a", "b"), SchemaRules.layout(listOf(k("a"), k("b")), "tunnel_").before.map { it.key })
    }

    @Test
    fun testResultsAndSaveWarnings() {
        fun o(s: String) = KitJson.parseToJsonElement(s).jsonObject
        assertEquals(ResultLine(true, "Connected"), SchemaRules.testLine(o("""{"ok":true}""")))
        assertEquals(ResultLine(false, "Failed"), SchemaRules.testLine(o("""{"ok":false,"message":""}""")))
        assertEquals(ResultLine(true, "Mail server 1.2 · ready"), SchemaRules.testLine(o("""{"ok":true,"message":"Mail server 1.2 · ready"}""")))
        assertEquals(ResultLine(false, "Test failed"), SchemaRules.testFailed(" "))
        assertEquals(ResultLine(false, "Admin only"), SchemaRules.testFailed("Admin only"))

        val answer = o("""{"data_path":"/data","_warnings":[{"key":"data_path","message":"The app can't write to /data"},{"key":"x"}]}""")
        assertEquals(listOf(ResultLine(false, "The app can't write to /data")), SchemaRules.warnings(answer))
        assertEquals(setOf("data_path"), SchemaRules.withoutWarnings(answer).keys)
        assertTrue(SchemaRules.warnings(o("{}")).isEmpty())
        assertEquals("Mail API", SchemaRules.optionLabel(f("mail_mode"), "api"))
        assertEquals("pigeon", SchemaRules.optionLabel(f("mail_mode"), "pigeon")) // not an option: as it is
    }

    /** The demo app's settings schema (android/demo) decodes whole, and its rules hold together. */
    @Test
    fun theDemoSchemaDecodesAndHoldsTogether() {
        val file = File("../demo/src/main/assets/settings-schema.json")
        assertTrue("demo schema not found from ${File(".").absolutePath}", file.isFile)
        val raw = KitJson.parseToJsonElement(file.readText()).jsonObject
        val s = KitJson.decodeFromString(SettingsSchema.serializer(), file.readText())
        val rawSections = raw["sections"] as JsonArray
        assertEquals(rawSections.size, s.sections.size)
        val all = s.sections.flatMap { it.groups }.flatMap { it.fields }
        val rawCount = rawSections.sumOf { sec -> (sec.jsonObject["groups"] as JsonArray).sumOf { g -> (g.jsonObject["fields"] as JsonArray).size } }
        assertEquals(rawCount, all.size) // no field dropped
        val kinds = setOf("text", "path", "url", "password", "textarea", "number", "bool", "select", "checks")
        assertEquals(kinds, all.map { it.kind }.toSet()) // every kind of field shows in the demo
        // every rule parses and names a select whose options hold the value
        for (fld in all.filter { it.rule.isNotEmpty() }) {
            val w = SchemaRules.parseWhen(fld.rule)!!
            val select = all.first { it.key == w.key }
            assertEquals("select", select.kind)
            assertTrue(fld.key, select.options.any { it.value == w.value })
        }
        assertTrue(all.filter { it.kind == "password" }.all { it.secret })
        assertTrue(all.filter { it.kind == "select" || it.kind == "checks" }.all { it.options.isNotEmpty() })
        assertTrue(s.sections.flatMap { it.groups }.filter { it.test.isNotEmpty() }.all { it.testLabel.isNotEmpty() })
    }
}
