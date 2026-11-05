package dev.krifmaz.kit.compose.signup

import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.dbl
import dev.krifmaz.kit.compose.core.obj
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.stringOnly
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlin.math.ceil

/**
 * The words of sign-up, the same defaults as kit-rules' `signup::SignupWords`. An app passes its own copy to change
 * any of them.
 */
data class SignupWords(
    val title: String = "Create your account",
    val createAccount: String = "Create an account",
    val inviteHint: String = "New here? Open the invite link an admin sent you.",
    val managedHint: String = "Accounts are managed by {app}. Sign up there: the same account works here.",
    val closed: String = "Sign-up is closed. Ask an admin to make you an account.",
    val needInvite: String = "Sign-up is by invite only. Open the invite link an admin sent you, or paste it below.",
    val inviteField: String = "Invite link",
    val inviteGives: String = "This invite makes a {role} account.",
    val email: String = "Email",
    val emailOptional: String = "Optional",
    val confirm: String = "Password again",
    val mismatch: String = "The two passwords aren't the same",
    val needName: String = "Choose a username",
    val needEmail: String = "Enter your email address: we send a link to confirm it.",
    val shortPassword: String = "Use a password of at least {n} characters",
    val signUp: String = "Create account",
    val signingUp: String = "Creating…",
    val back: String = "Back to sign in",
    val adminTitle: String = "Sign-up",
    val whoTitle: String = "Who can sign up",
    val openTitle: String = "Open sign-up",
    val openRole: String = "New accounts get the role",
    val openPresets: String = "and these presets",
    val verifyEmail: String = "Confirm their email first",
    val noMail: String = "Mail isn't set up on this server: confirmation links only go to the server's log.",
    val approval: String = "An admin approves each one",
    val perHour: String = "Sign-ups per address per hour",
    val perDay: String = "Sign-ups per day (0: no limit)",
    val save: String = "Save",
    val saved: String = "Saved",
    val readOnly: String = "Accounts are managed by {app}: this setting is changed there and applies to every linked app.",
    val unreachable: String = "{app} can't be reached right now, so its setting isn't shown.",
    val invitesTitle: String = "Invites",
    val newInvite: String = "New invite",
    val note: String = "Note",
    val notePlaceholder: String = "Who it's for (only admins see this)",
    val role: String = "Role",
    val presets: String = "Presets",
    val uses: String = "Can be used by",
    val oneTime: String = "One person",
    val noLimit: String = "Anyone with the link",
    val several: String = "Up to",
    val expires: String = "Works for",
    val create: String = "Make invite",
    val linkTitle: String = "Invite link",
    val linkOnce: String = "Copy it now: for safety, the link is shown only this once.",
    val copyLink: String = "Copy link",
    val copied: String = "Link copied",
    val revoke: String = "Revoke",
    val revokeQuestion: String = "Revoke “{note}”?",
    val revokeText: String = "The link stops working at once. Accounts already made with it stay.",
    val noInvites: String = "No invites yet. New invite makes a link to send.",
    val usedBy: String = "Used by",
    val unused: String = "Not used yet",
    val untitled: String = "Invite",
    val pendingTitle: String = "Waiting sign-ups",
    val noPending: String = "Nobody is waiting.",
    val approve: String = "Approve",
    val reject: String = "Reject",
    val rejectQuestion: String = "Reject {username}?",
    val rejectText: String = "Their account is removed. They can sign up again.",
    val waitingEmail: String = "Waiting for their email",
    val waitingApproval: String = "Waiting for approval",
)

/** The app that keeps the accounts when apps share them. */
data class ManagedBy(val name: String, val url: String, val reachable: Boolean)

/** What `GET /api/signup` says. */
data class Policy(val who: String = "", val verifyEmail: Boolean = false, val approval: Boolean = false, val managedBy: ManagedBy? = null) {
    /** Whether the email field must be filled: open sign-up with confirmation, and no invite. */
    fun emailRequired(hasInvite: Boolean): Boolean = who == "open" && verifyEmail && !hasInvite
}

/** What the sign-in page's line opens. */
sealed interface SignInLink {
    /** Only words. */
    data object None : SignInLink

    /** This app's sign-up page. */
    data object SignUp : SignInLink

    /** The account source's address. */
    data class Source(val url: String) : SignInLink
}

/** A role or preset the app offers. */
data class Choice(val id: String, val label: String)

/** The admin's setting. */
data class SignupSettings(
    val who: String = "invite",
    val openRole: String = "",
    val openPresets: List<String> = emptyList(),
    val verifyEmail: Boolean = false,
    val approval: Boolean = false,
    val perHour: Int = 5,
    val perDay: Int = 50,
)

/** An invite of the admin's list. */
data class InviteRow(
    val id: Long,
    val note: String,
    val role: String,
    val presets: List<String>,
    val maxUses: Long,
    val uses: Long,
    val expiresAt: Double,
    val status: String,
    val createdBy: String,
    val createdAt: Double,
    val usedBy: List<Pair<String, Double>>,
)

/** A waiting sign-up. */
data class PendingRow(val id: Long, val username: String, val email: String, val waitingFor: String, val address: String, val createdAt: Double)

/**
 * The rules of sign-up, the same as kit-rules' `signup` module (contracts/signup.md): reading the server's answers,
 * the sign-in page's line, the form's checks, invite links, and the admin's rows. Plain Kotlin, tested on the JVM.
 */
object SignupRules {
    /** Who can sign up: (value, name, sentence). */
    val WHO: List<Triple<String, String, String>> = listOf(
        Triple("closed", "Closed", "Only admins make accounts."),
        Triple("invite", "Invite only", "People sign up with an invite link an admin made. This is the default."),
        Triple("open", "Open", "Anyone can sign up, within the limits below."),
    )

    /** How long a new invite works: (hours, words); 0 is never. */
    val EXPIRY: List<Pair<Int, String>> = listOf(24 to "1 day", 168 to "7 days", 720 to "30 days", 0 to "Never")

    /** A new invite's expiry when nothing is picked. */
    const val DEFAULT_EXPIRY_HOURS = 168

    /** The shortest password the form takes before asking the server. */
    const val MIN_PASSWORD = 8

    /** `template` with each `{key}` filled in. */
    fun fill(template: String, vararg vars: Pair<String, String>): String = vars.fold(template) { s, (k, v) -> s.replace("{$k}", v) }

    /** `managed_by` of an answer; null for null or missing. */
    fun managedBy(v: JsonObject): ManagedBy? {
        val m = v.obj("managed_by") ?: return null
        val reachable = (m["reachable"] as? JsonPrimitive)?.content?.let { it == "true" } ?: true
        return ManagedBy(m.stringOnly("name"), m.stringOnly("url"), reachable)
    }

    /** `GET /api/signup` read. */
    fun policy(v: JsonObject): Policy = Policy(v.stringOnly("who"), v.bool("verify_email"), v.bool("approval"), managedBy(v))

    /** The line under the sign-in form, or null (closed). */
    fun signInLine(p: Policy, w: SignupWords = SignupWords()): Pair<String, SignInLink>? {
        p.managedBy?.let { return fill(w.managedHint, "app" to it.name) to SignInLink.Source(it.url) }
        return when (p.who) {
            "open" -> w.createAccount to SignInLink.SignUp
            "invite" -> w.inviteHint to SignInLink.None
            else -> null
        }
    }

    /** Null when the form shows; else the sentence shown instead. */
    fun formBlocked(p: Policy, hasInvite: Boolean, w: SignupWords = SignupWords()): String? {
        p.managedBy?.let { return fill(w.managedHint, "app" to it.name) }
        return when {
            p.who == "closed" -> w.closed
            p.who == "invite" && !hasInvite -> w.needInvite
            else -> null
        }
    }

    /** The form's checks before it is sent: null when it may go, else the sentence. */
    fun checkForm(username: String, password: String, again: String, email: String, emailRequired: Boolean, w: SignupWords = SignupWords()): String? = when {
        username.isBlank() -> w.needName
        password.length < MIN_PASSWORD -> fill(w.shortPassword, "n" to MIN_PASSWORD.toString())
        password != again -> w.mismatch
        emailRequired && email.isBlank() -> w.needEmail
        else -> null
    }

    /** The `POST /api/signup` body. */
    fun signUpBody(username: String, password: String, email: String, invite: String): JsonObject = buildJsonObject {
        put("username", username.trim())
        put("password", password)
        put("email", email.trim())
        put("invite", invite.trim())
    }

    /** The invite token in a pasted link or the token alone; "" for neither. */
    fun tokenFrom(pasted: String): String {
        val p = pasted.trim()
        val at = p.indexOf("invite=").takeIf { it >= 0 } ?: p.indexOf("token=").takeIf { it >= 0 }
        val from = if (at != null) p.substring(p.indexOf('=', at) + 1) else p
        val t = from.takeWhile { it.isDigit() || it in 'a'..'f' || it in 'A'..'F' }
        return if (t.length >= 16) t else ""
    }

    /** A link from the server: as it is with an address, else after [server]. */
    fun fullLink(link: String, server: String): String = if (link.startsWith("http://") || link.startsWith("https://")) link else server.trimEnd('/') + link

    /** The `settings` object read. */
    fun settings(v: JsonObject?): SignupSettings {
        val d = SignupSettings()
        if (v == null) return d
        fun n(k: String, def: Int) = (v[k] as? JsonPrimitive)?.content?.toDoubleOrNull()?.toInt() ?: def
        return SignupSettings(
            who = v.stringOnly("who").ifEmpty { d.who },
            openRole = v.stringOnly("open_role"),
            openPresets = strings(v.arr("open_presets")),
            verifyEmail = v.bool("verify_email"),
            approval = v.bool("approval"),
            perHour = n("per_hour", d.perHour),
            perDay = n("per_day", d.perDay),
        )
    }

    /** The `PUT /api/signup/settings` body. */
    fun settingsBody(s: SignupSettings): JsonObject = buildJsonObject {
        put("who", s.who)
        put("open_role", s.openRole)
        putJsonArray("open_presets") { s.openPresets.forEach { add(JsonPrimitive(it)) } }
        put("verify_email", s.verifyEmail)
        put("approval", s.approval)
        put("per_hour", s.perHour)
        put("per_day", s.perDay)
    }

    /** `roles` or `presets` of the settings answer. */
    fun choices(v: JsonObject, key: String): List<Choice> = v.arr(key).objectsOf().map { Choice(it.stringOnly("id"), it.stringOnly("label")) }

    /** The label of [id], or [id] itself. */
    fun labelOf(list: List<Choice>, id: String): String = list.firstOrNull { it.id == id }?.label ?: id

    /** The `POST /api/signup/invites` body. */
    fun inviteBody(note: String, role: String, presets: List<String>, maxUses: Long, expiresInHours: Int): JsonObject = buildJsonObject {
        put("note", note.trim())
        put("role", role)
        putJsonArray("presets") { presets.forEach { add(JsonPrimitive(it)) } }
        put("max_uses", maxUses)
        put("expires_in_hours", expiresInHours)
    }

    /** The number of people typed in, from 2 to 100 000; null otherwise. */
    fun parseUses(text: String): Long? = text.trim().toLongOrNull()?.takeIf { it in 2..100_000 }

    /** `{"invites": […]}` read. */
    fun invites(v: JsonObject): List<InviteRow> = v.arr("invites").objectsOf().map { i ->
        InviteRow(
            id = i.dbl("id").toLong(),
            note = i.stringOnly("note"),
            role = i.stringOnly("role"),
            presets = strings(i.arr("presets")),
            maxUses = if (i["max_uses"] == null) 1 else i.dbl("max_uses").toLong(),
            uses = i.dbl("uses").toLong(),
            expiresAt = i.dbl("expires_at"),
            status = i.stringOnly("status"),
            createdBy = i.stringOnly("created_by_name"),
            createdAt = i.dbl("created_at"),
            usedBy = i.arr("used_by").objectsOf().map { it.stringOnly("username") to it.dbl("used_at") },
        )
    }

    /** `{"pending": […]}` read. */
    fun pending(v: JsonObject): List<PendingRow> = v.arr("pending").objectsOf().map { p ->
        PendingRow(p.dbl("id").toLong(), p.stringOnly("username"), p.stringOnly("email"), p.stringOnly("waiting_for"), p.stringOnly("address"), p.dbl("created_at"))
    }

    /** An invite's title: its note, else "Invite <id>". */
    fun title(i: InviteRow, w: SignupWords = SignupWords()): String = i.note.ifBlank { "${w.untitled} ${i.id}" }

    /** "1 of 1 use", "2 of 5 uses", "3 uses, no limit". */
    fun usesLine(i: InviteRow): String {
        fun s(n: Long) = if (n == 1L) "" else "s"
        return if (i.maxUses > 0) "${i.uses} of ${i.maxUses} use${s(i.maxUses)}" else "${i.uses} use${s(i.uses)}, no limit"
    }

    /** "Never expires", "Expires in 3 days", "Expires in 5 hours", "Expired". */
    fun expiryLine(i: InviteRow, now: Double): String {
        if (i.expiresAt <= 0) return "Never expires"
        val left = i.expiresAt - now
        if (left <= 0) return "Expired"
        val hours = ceil(left / 3600.0).toLong()
        return if (hours < 48) "Expires in $hours hour${if (hours == 1L) "" else "s"}" else "Expires in ${hours / 24} days"
    }

    /** Whether Revoke shows: only while it works. */
    fun canRevoke(i: InviteRow): Boolean = i.status == "active"

    /** A status's words and badge kind. */
    fun status(s: String): Pair<String, String> = when (s) {
        "active" -> "Active" to "ok"
        "revoked" -> "Revoked" to "err"
        "expired" -> "Expired" to ""
        "used_up" -> "Used up" to "warn"
        else -> "Unknown" to ""
    }

    /** What a waiting sign-up waits for, in words. */
    fun waitingLine(p: PendingRow, w: SignupWords = SignupWords()): String = if (p.waitingFor == "verify_email") w.waitingEmail else w.waitingApproval

    private fun strings(a: JsonArray?): List<String> = a?.mapNotNull { (it as? JsonPrimitive)?.takeIf { p -> p.isString }?.content } ?: emptyList()
}
