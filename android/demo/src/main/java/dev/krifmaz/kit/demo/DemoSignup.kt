package dev.krifmaz.kit.demo

import dev.krifmaz.kit.compose.signup.SignupRules
import dev.krifmaz.kit.compose.signup.SignupSettings
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.add
import kotlinx.serialization.json.addJsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray

/**
 * The demo server's sign-up routes (`/api/signup`, the kit's sign-up contract), kept in memory: the setting, the
 * invites (made, used once by "sam", revoked) and one account waiting for approval. Nothing is mailed; nothing here
 * makes a real account.
 *
 * @param base where the routes start (`KitRoutes.signup`).
 * @param now the time in seconds.
 */
class DemoSignup(private val base: String, private val now: () -> Double) {
    private class Invite(val id: Long, val note: String, val role: String, val presets: List<String>, val maxUses: Long, val expiresAt: Double, val createdAt: Double) {
        var revoked = false
        val usedBy = ArrayList<Pair<String, Double>>()

        fun status(now: Double): String = when {
            revoked -> "revoked"
            expiresAt > 0 && now >= expiresAt -> "expired"
            maxUses > 0 && usedBy.size >= maxUses -> "used_up"
            else -> "active"
        }
    }

    private class Pending(val id: Long, val username: String, val email: String, val waitingFor: String, val createdAt: Double)

    private var settings = SignupSettings()
    private val invites = ArrayList<Invite>()
    private val pending = ArrayList<Pending>()
    private var nextId = 1L

    init {
        val t = now()
        val used = Invite(nextId++, "For Sam", "user", listOf("shared"), 1, t + 86_400 * 6, t - 86_400)
        used.usedBy += "sam" to t - 3_600
        invites += used
        pending += Pending(nextId++, "robin", "robin@example.com", "approval", t - 1_800)
    }

    /** Answers a sign-up request, or null when [r] is not one. */
    fun route(r: DemoRequest): DemoAnswer? {
        if (r.path != base && !r.path.startsWith("$base/")) return null
        val rest = r.path.removePrefix(base).trim('/')
        val parts = if (rest.isEmpty()) emptyList() else rest.split('/')
        val m = r.method
        return when {
            parts.isEmpty() && m == "GET" -> DemoAnswer.json(
                buildJsonObject {
                    put("who", settings.who)
                    put("verify_email", settings.verifyEmail)
                    put("approval", settings.approval)
                },
            )

            parts.isEmpty() && m == "POST" -> DemoAnswer.error(403, "The demo makes no accounts; sign-up works on a real server")

            parts == listOf("settings") && m == "GET" -> DemoAnswer.json(admin())

            parts == listOf("settings") && m == "PUT" -> {
                settings = SignupRules.settings(r.json)
                DemoAnswer.json(admin())
            }

            parts == listOf("invites") && m == "GET" -> DemoAnswer.json(buildJsonObject { putJsonArray("invites") { invites.asReversed().forEach { add(row(it)) } } })

            parts == listOf("invites") && m == "POST" -> DemoAnswer.json(create(r.json ?: JsonObject(emptyMap())))

            parts.size == 3 && parts[0] == "invites" && parts[2] == "revoke" && m == "POST" -> {
                val i = invites.firstOrNull { it.id == parts[1].toLongOrNull() } ?: return DemoAnswer.error(404, "No such invite")
                i.revoked = true
                DemoAnswer.json(buildJsonObject { put("invite", row(i)) })
            }

            parts == listOf("pending") && m == "GET" -> DemoAnswer.json(
                buildJsonObject {
                    putJsonArray("pending") {
                        pending.forEach { p ->
                            addJsonObject {
                                put("id", p.id)
                                put("username", p.username)
                                put("email", p.email)
                                put("waiting_for", p.waitingFor)
                                put("address", "192.0.2.10")
                                put("created_at", p.createdAt)
                            }
                        }
                    }
                },
            )

            parts.size == 3 && parts[0] == "pending" && (parts[2] == "approve" || parts[2] == "reject") && m == "POST" -> {
                if (!pending.removeIf { it.id == parts[1].toLongOrNull() }) return DemoAnswer.error(404, "No such sign-up")
                DemoAnswer.json(buildJsonObject { put("ok", true) })
            }

            else -> null
        }
    }

    private fun admin(): JsonObject = buildJsonObject {
        put("settings", SignupRules.settingsBody(settings))
        putJsonArray("roles") {
            addJsonObject {
                put("id", "user")
                put("label", "Member")
            }
            addJsonObject {
                put("id", "admin")
                put("label", "Admin")
            }
        }
        putJsonArray("presets") {
            addJsonObject {
                put("id", "shared")
                put("label", "Shared space")
            }
            addJsonObject {
                put("id", "uploads")
                put("label", "May upload files")
            }
        }
        put("mail_delivers", false)
    }

    private fun create(body: JsonObject): JsonObject {
        fun text(k: String) = (body[k] as? JsonPrimitive)?.content.orEmpty()
        val hours = text("expires_in_hours").toDoubleOrNull() ?: SignupRules.DEFAULT_EXPIRY_HOURS.toDouble()
        val presets = (body["presets"] as? JsonArray)?.mapNotNull { (it as? JsonPrimitive)?.content } ?: emptyList()
        val t = now()
        val i = Invite(
            nextId++,
            text("note"),
            text("role").ifEmpty { "user" },
            presets,
            text("max_uses").toLongOrNull() ?: 1,
            if (hours > 0) t + hours * 3_600 else 0.0,
            t,
        )
        invites += i
        val token = "%064x".format(i.id)
        return buildJsonObject {
            put("invite", row(i))
            put("token", token)
            put("link", "/#/sign-up?invite=$token")
        }
    }

    private fun row(i: Invite): JsonObject = buildJsonObject {
        put("id", i.id)
        put("note", i.note)
        put("role", i.role)
        putJsonArray("presets") { i.presets.forEach { add(it) } }
        put("max_uses", i.maxUses)
        put("uses", i.usedBy.size)
        put("expires_at", i.expiresAt)
        put("status", i.status(now()))
        put("created_by_name", "admin")
        put("created_at", i.createdAt)
        putJsonArray("used_by") {
            i.usedBy.forEach { (who, at) ->
                addJsonObject {
                    put("username", who)
                    put("used_at", at)
                }
            }
        }
    }
}
