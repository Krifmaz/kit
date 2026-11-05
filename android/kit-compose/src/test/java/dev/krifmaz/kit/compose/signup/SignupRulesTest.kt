package dev.krifmaz.kit.compose.signup

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The sign-up rules, the same cases as kit-rules' `signup` tests. */
class SignupRulesTest {
    private fun obj(s: String): JsonObject = Json.parseToJsonElement(s).jsonObject

    @Test
    fun theSignInLineFollowsThePolicy() {
        val w = SignupWords()
        assertEquals("Create an account" to SignInLink.SignUp, SignupRules.signInLine(SignupRules.policy(obj("""{"who":"open"}"""))))
        assertEquals(SignInLink.None, SignupRules.signInLine(SignupRules.policy(obj("""{"who":"invite"}""")))!!.second)
        assertNull(SignupRules.signInLine(SignupRules.policy(obj("""{"who":"closed"}"""))))
        val m = SignupRules.policy(obj("""{"who":"open","managed_by":{"name":"Hub","url":"https://hub.example"}}"""))
        assertTrue(m.managedBy!!.reachable)
        val (text, link) = SignupRules.signInLine(m, w)!!
        assertTrue(text.startsWith("Accounts are managed by Hub."))
        assertEquals(SignInLink.Source("https://hub.example"), link)
        assertNull(SignupRules.policy(obj("""{"who":"open","managed_by":null}""")).managedBy)
    }

    @Test
    fun theFormShowsWhenItCanBeUsed() {
        val w = SignupWords()
        assertEquals(w.closed, SignupRules.formBlocked(SignupRules.policy(obj("""{"who":"closed"}""")), true))
        assertEquals(w.needInvite, SignupRules.formBlocked(SignupRules.policy(obj("""{"who":"invite"}""")), false))
        assertNull(SignupRules.formBlocked(SignupRules.policy(obj("""{"who":"invite"}""")), true))
        assertNull(SignupRules.formBlocked(SignupRules.policy(obj("""{"who":"open"}""")), false))
        val v = SignupRules.policy(obj("""{"who":"open","verify_email":true}"""))
        assertTrue(v.emailRequired(false))
        assertFalse(v.emailRequired(true))
    }

    @Test
    fun formChecks() {
        val w = SignupWords()
        assertEquals(w.needName, SignupRules.checkForm(" ", "password1", "password1", "", false))
        assertEquals("Use a password of at least 8 characters", SignupRules.checkForm("ana", "short", "short", "", false))
        assertEquals(w.mismatch, SignupRules.checkForm("ana", "password1", "password2", "", false))
        assertEquals(w.needEmail, SignupRules.checkForm("ana", "password1", "password1", "", true))
        assertNull(SignupRules.checkForm("ana", "password1", "password1", "a@b.co", true))
        assertEquals(obj("""{"username":"ana","password":"pw","email":"a@b.co","invite":"t"}"""), SignupRules.signUpBody(" ana ", "pw", " a@b.co", "t"))
    }

    @Test
    fun tokensAndLinks() {
        val t = "a".repeat(64)
        assertEquals(t, SignupRules.tokenFrom("https://x.example/#/sign-up?invite=$t"))
        assertEquals(t, SignupRules.tokenFrom("  $t\n"))
        assertEquals(t, SignupRules.tokenFrom("/#/sign-up/verify?token=$t&x=1"))
        assertEquals("", SignupRules.tokenFrom("hello"))
        assertEquals("https://h.example/#/sign-up?invite=1", SignupRules.fullLink("/#/sign-up?invite=1", "https://h.example/"))
        assertEquals("https://a.example/x", SignupRules.fullLink("https://a.example/x", "https://h"))
    }

    @Test
    fun settingsRoundTrip() {
        val s = SignupRules.settings(obj("""{"who":"open","approval":true,"per_hour":3,"open_presets":["a"]}"""))
        assertEquals(SignupSettings(who = "open", approval = true, perHour = 3, openPresets = listOf("a")), s)
        assertEquals(s, SignupRules.settings(SignupRules.settingsBody(s)))
        assertEquals(SignupSettings(), SignupRules.settings(obj("{}")))
        val roles = SignupRules.choices(obj("""{"roles":[{"id":"m","label":"Member"}]}"""), "roles")
        assertEquals("Member", SignupRules.labelOf(roles, "m"))
        assertEquals("x", SignupRules.labelOf(roles, "x"))
    }

    @Test
    fun inviteRows() {
        val list = SignupRules.invites(
            obj(
                """{"invites":[{"id":3,"note":"","role":"member","presets":["a"],"max_uses":5,"uses":2,"expires_at":260200.0,
                "status":"active","created_by_name":"admin","created_at":10.0,"used_by":[{"username":"ana","used_at":20.0}]}]}""",
            ),
        )
        val r = list[0]
        assertEquals("Invite 3", SignupRules.title(r))
        assertEquals("2 of 5 uses", SignupRules.usesLine(r))
        assertEquals("Expires in 3 days", SignupRules.expiryLine(r, 1000.0))
        assertEquals(listOf("ana" to 20.0), r.usedBy)
        assertTrue(SignupRules.canRevoke(r))
        val one = r.copy(maxUses = 1, uses = 1, status = "used_up")
        assertEquals("1 of 1 use", SignupRules.usesLine(one))
        assertFalse(SignupRules.canRevoke(one))
        val open = r.copy(maxUses = 0, uses = 1, expiresAt = 0.0)
        assertEquals("1 use, no limit", SignupRules.usesLine(open))
        assertEquals("Never expires", SignupRules.expiryLine(open, 5.0))
        val soon = r.copy(expiresAt = 3600.0)
        assertEquals("Expires in 1 hour", SignupRules.expiryLine(soon, 10.0))
        assertEquals("Expired", SignupRules.expiryLine(soon, 4000.0))
        assertEquals("Used up" to "warn", SignupRules.status("used_up"))
        assertEquals(5L, SignupRules.parseUses("5"))
        assertNull(SignupRules.parseUses("1"))
        assertEquals(168, SignupRules.DEFAULT_EXPIRY_HOURS)
    }

    @Test
    fun pendingRows() {
        val w = SignupWords()
        val l = SignupRules.pending(obj("""{"pending":[{"id":1,"username":"vic","waiting_for":"verify_email"},{"id":2,"waiting_for":"approval"}]}"""))
        assertEquals(w.waitingEmail, SignupRules.waitingLine(l[0]))
        assertEquals(w.waitingApproval, SignupRules.waitingLine(l[1]))
    }
}
