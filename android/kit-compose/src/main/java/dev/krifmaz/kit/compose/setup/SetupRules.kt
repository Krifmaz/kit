package dev.krifmaz.kit.compose.setup

import dev.krifmaz.kit.compose.signup.SignupRules
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/**
 * The words of first-run setup, the same defaults as kit-rules' `setup::SetupWords`. An app passes its own copy to
 * change any of them.
 */
data class SetupWords(
    val title: String = "Create your account",
    val intro: String = "This server has no account yet. Make yours: it is the owner, who can change everything.",
    val code: String = "Setup code",
    val codeHint: String = "The server prints it in its log when it starts (for Docker: docker logs, then the container's name).",
    val username: String = "Username",
    val password: String = "Password",
    val again: String = "Password again",
    val email: String = "Email",
    val emailOptional: String = "optional",
    val needCode: String = "Enter the setup code from the server's log.",
    val needName: String = "Choose a username",
    val shortPassword: String = "Use a password of at least {n} characters",
    val mismatch: String = "The two passwords aren't the same",
    val create: String = "Create account",
    val creating: String = "Creating…",
)

/** First-run setup's rules (contracts/setup.md), the same as kit-rules' `setup`. */
object SetupRules {
    /** Whether the server wants "Create your account" (`GET /api/setup` → `{needed}`); anything else is no. */
    fun needed(v: JsonObject?): Boolean = (v?.get("needed") as? JsonPrimitive)?.content == "true"

    /** The form's problem before it is sent, in [w]'s words; null when it can go. */
    fun checkForm(code: String, username: String, password: String, again: String, w: SetupWords = SetupWords()): String? = when {
        code.isBlank() -> w.needCode
        username.isBlank() -> w.needName
        password.length < SignupRules.MIN_PASSWORD -> SignupRules.fill(w.shortPassword, "n" to SignupRules.MIN_PASSWORD.toString())
        password != again -> w.mismatch
        else -> null
    }

    /** The `POST /api/setup` body. */
    fun body(code: String, username: String, password: String, email: String): JsonObject = buildJsonObject {
        put("code", code.trim())
        put("username", username.trim())
        put("password", password)
        put("email", email.trim())
    }
}
