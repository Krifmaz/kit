package dev.krifmaz.kit.compose.signup

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.details.copyToClipboard
import dev.krifmaz.kit.compose.dialogs.Confirm
import dev.krifmaz.kit.compose.dialogs.ConfirmDialog
import dev.krifmaz.kit.compose.settings.CardHead
import dev.krifmaz.kit.compose.settings.Muted
import dev.krifmaz.kit.compose.settings.SetCard
import dev.krifmaz.kit.compose.settings.SettingsPage
import dev.krifmaz.kit.compose.settings.TextLink
import dev.krifmaz.kit.compose.settings.serverCall
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.widgets.Badge
import dev.krifmaz.kit.compose.widgets.ChoiceRow
import dev.krifmaz.kit.compose.widgets.ErrorBox
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.KitField
import dev.krifmaz.kit.compose.widgets.SmallButton
import dev.krifmaz.kit.compose.widgets.Spinner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import java.text.DateFormat
import java.util.Date

/** The sign-up routes' base (`KitApp.routes.signup`). */
private fun path(p: String = ""): String = KitApp.routes.signup + p

/** A unix time as the phone's short date, "—" for none. */
private fun date(ts: Double): String = if (ts <= 0) "—" else DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date((ts * 1000).toLong()))

/**
 * The line under the sign-in form (contracts/signup.md): a link to [onSignUp] (open sign-up), how to get in (invite
 * only), where accounts are made (another app keeps them: [openUrl]), or nothing (closed, or the server can't say).
 */
@Composable
fun SignInLine(api: KitApi, onSignUp: () -> Unit, openUrl: (String) -> Unit, words: SignupWords = SignupWords()) {
    var policy by remember { mutableStateOf<Policy?>(null) }
    LaunchedEffect(api) {
        policy = try {
            withContext(Dispatchers.IO) { (api.get(path()) as? JsonObject)?.let(SignupRules::policy) }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            null
        }
    }
    val (text, link) = policy?.let { SignupRules.signInLine(it, words) } ?: return
    val p = MaterialTheme.palette
    when (link) {
        SignInLink.SignUp -> TextLink(text, Modifier.padding(top = 12.dp)) { onSignUp() }
        is SignInLink.Source -> TextLink(text, Modifier.padding(top = 12.dp)) { openUrl(link.url) }
        SignInLink.None -> Text(text, style = MaterialTheme.typography.bodySmall, color = p.muted, modifier = Modifier.padding(top = 12.dp))
    }
}

/**
 * The sign-up page: an invite link ([invite], or pasted), or open sign-up; then the server's answer and [onBack].
 */
@Composable
fun SignUpScreen(api: KitApi, invite: String, onBack: () -> Unit, words: SignupWords = SignupWords()) {
    val scope = rememberCoroutineScope()
    var policy by remember { mutableStateOf<Result<Policy>?>(null) }
    var inviteText by remember { mutableStateOf(invite) }
    var inviteCheck by remember { mutableStateOf<Result<String>?>(null) }
    var username by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var again by remember { mutableStateOf("") }
    var error by remember { mutableStateOf("") }
    var done by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    val token = SignupRules.tokenFrom(inviteText)
    LaunchedEffect(api) {
        policy = runCatching { withContext(Dispatchers.IO) { SignupRules.policy(api.get(path()) as? JsonObject ?: JsonObject(emptyMap())) } }
    }
    LaunchedEffect(token) {
        inviteCheck = if (token.isEmpty()) {
            null
        } else {
            runCatching {
                val v = withContext(Dispatchers.IO) { api.post(path("/invite"), buildJsonObject { put("token", JsonPrimitive(token)) }) as? JsonObject }
                SignupRules.fill(words.inviteGives, "role" to ((v?.get("role") as? JsonPrimitive)?.content ?: ""))
            }
        }
    }
    SettingsPage(title = words.title, onBack = onBack) {
        Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            done?.let { msg ->
                Text(msg, style = MaterialTheme.typography.bodyLarge)
                KitButton(words.back, primary = true, onClick = onBack)
                return@Column
            }
            val p = when (val r = policy) {
                null -> {
                    Spinner()
                    return@Column
                }

                else -> r.getOrElse {
                    ErrorBox(it.message ?: "Something went wrong") { policy = null }
                    return@Column
                }
            }
            val blocked = SignupRules.formBlocked(p, token.isNotEmpty(), words)
            val inviteField = @Composable {
                KitField(inviteText, { inviteText = it }, Modifier.fillMaxWidth(), label = words.inviteField, placeholder = "https://…")
            }
            if (blocked != null) {
                Muted(blocked)
                if (p.who == "invite" && p.managedBy == null) inviteField()
                return@Column
            }
            if (p.who == "invite" || token.isNotEmpty()) inviteField()
            inviteCheck?.let { r -> r.fold({ Muted(it) }, { Text(it.message ?: "", color = KitStatusColors.Err) }) }
            if (error.isNotEmpty()) Text(error, color = KitStatusColors.Err)
            val required = p.emailRequired(token.isNotEmpty())
            KitField(username, { username = it }, Modifier.fillMaxWidth(), label = "Username")
            KitField(email, {
                email = it
            }, Modifier.fillMaxWidth(), label = if (required) words.email else "${words.email} (${words.emailOptional})", keyboardType = KeyboardType.Email)
            KitField(password, { password = it }, Modifier.fillMaxWidth(), label = "Password", password = true)
            KitField(again, { again = it }, Modifier.fillMaxWidth(), label = words.confirm, password = true)
            KitButton(if (busy) words.signingUp else words.signUp, Modifier.fillMaxWidth(), primary = true, enabled = !busy) {
                val bad = SignupRules.checkForm(username, password, again, email, required, words)
                if (bad != null) {
                    error = bad
                    return@KitButton
                }
                error = ""
                busy = true
                scope.serverCall<JsonObject?>(onError = {
                    error = it
                    busy = false
                }, after = { v ->
                    busy = false
                    done = (v?.get("message") as? JsonPrimitive)?.content ?: ""
                }) { api.post(path(), SignupRules.signUpBody(username, password, email, token)) as? JsonObject }
            }
            TextLink(words.back) { onBack() }
        }
    }
}

/** Everything the admin page shows. */
private data class AdminData(
    val settings: SignupSettings,
    val roles: List<Choice>,
    val presets: List<Choice>,
    val mailDelivers: Boolean,
    val managed: ManagedBy?,
    val invites: List<InviteRow>,
    val pending: List<PendingRow>,
)

/** Loads the admin page (blocking; on Dispatchers.IO). */
private fun loadAdmin(api: KitApi): AdminData {
    val s = api.get(path("/settings")) as? JsonObject ?: JsonObject(emptyMap())
    val managed = SignupRules.managedBy(s)
    val invites = if (managed == null) SignupRules.invites(api.get(path("/invites")) as? JsonObject ?: JsonObject(emptyMap())) else emptyList()
    val pending = if (managed == null) SignupRules.pending(api.get(path("/pending")) as? JsonObject ?: JsonObject(emptyMap())) else emptyList()
    return AdminData(
        SignupRules.settings(s["settings"] as? JsonObject),
        SignupRules.choices(s, "roles"),
        SignupRules.choices(s, "presets"),
        (s["mail_delivers"] as? JsonPrimitive)?.content == "true",
        managed,
        invites,
        pending,
    )
}

/**
 * The admin's sign-up page: who can sign up (and how open sign-up is held back), waiting sign-ups with Approve and
 * Reject, and invites (New invite, the link shown once with Copy link, Revoke, who used each). Read-only when another
 * app keeps the accounts. [server] is put before links the server gives without an address.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SignupAdminScreen(api: KitApi, server: String, onBack: () -> Unit, words: SignupWords = SignupWords()) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    var data by remember { mutableStateOf<Result<AdminData>?>(null) }
    var reloads by remember { mutableIntStateOf(0) }
    var edit by remember { mutableStateOf(SignupSettings()) }
    var perHour by remember { mutableStateOf("5") }
    var perDay by remember { mutableStateOf("50") }
    var formOpen by remember { mutableStateOf(false) }
    var note by remember { mutableStateOf("") }
    var role by remember { mutableStateOf("") }
    var presets by remember { mutableStateOf(listOf<String>()) }
    var uses by remember { mutableIntStateOf(0) }
    var usesN by remember { mutableStateOf("5") }
    var expiry by remember { mutableIntStateOf(SignupRules.EXPIRY.indexOfFirst { it.first == SignupRules.DEFAULT_EXPIRY_HOURS }) }
    var link by remember { mutableStateOf<String?>(null) }
    var asking by remember { mutableStateOf<Pair<Confirm, () -> Unit>?>(null) }
    val reload = { reloads++ }
    LaunchedEffect(reloads) {
        val r = runCatching { withContext(Dispatchers.IO) { loadAdmin(api) } }
        r.onSuccess {
            edit = it.settings
            perHour = it.settings.perHour.toString()
            perDay = it.settings.perDay.toString()
            if (role.isEmpty()) role = it.roles.firstOrNull()?.id ?: ""
        }
        data = r
    }
    SettingsPage(title = words.adminTitle, onBack = onBack) {
        val d = when (val r = data) {
            null -> {
                Spinner()
                return@SettingsPage
            }

            else -> r.getOrElse {
                ErrorBox(it.message ?: "Something went wrong") { reload() }
                return@SettingsPage
            }
        }
        val readOnly = d.managed != null
        Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
            d.managed?.let { m ->
                val line = SignupRules.fill(words.readOnly, "app" to m.name) + if (m.reachable) "" else " " + SignupRules.fill(words.unreachable, "app" to m.name)
                SetCard(flat = true) { Text(line, style = MaterialTheme.typography.bodySmall) }
            }
            SetCard {
                CardHead(words.whoTitle)
                for ((id, name, line) in SignupRules.WHO) {
                    Row(Modifier.fillMaxWidth().clickable(enabled = !readOnly) { edit = edit.copy(who = id) }, verticalAlignment = Alignment.CenterVertically) {
                        RadioButton(selected = edit.who == id, onClick = { edit = edit.copy(who = id) }, enabled = !readOnly)
                        Column {
                            Text(name, fontWeight = FontWeight.SemiBold)
                            Muted(line)
                        }
                    }
                }
                if (edit.who == "open") {
                    Text(words.openTitle, style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(top = 8.dp))
                    Muted(words.openRole)
                    val openRole = edit.openRole.ifEmpty { d.roles.firstOrNull()?.id ?: "" }
                    ChoiceRow(d.roles.map { it.label }, d.roles.indexOfFirst { it.id == openRole }, { if (!readOnly) edit = edit.copy(openRole = d.roles[it].id) })
                    for (pr in d.presets) {
                        SwitchLine(pr.label, pr.id in edit.openPresets, !readOnly) { on ->
                            edit =
                                edit.copy(openPresets = edit.openPresets.filter { it != pr.id } + if (on) listOf(pr.id) else emptyList())
                        }
                    }
                    SwitchLine(words.verifyEmail, edit.verifyEmail, !readOnly) { edit = edit.copy(verifyEmail = it) }
                    if (!d.mailDelivers) Muted(words.noMail)
                    SwitchLine(words.approval, edit.approval, !readOnly) { edit = edit.copy(approval = it) }
                    KitField(perHour, { perHour = it }, Modifier.fillMaxWidth(), label = words.perHour, enabled = !readOnly, keyboardType = KeyboardType.Number)
                    KitField(perDay, { perDay = it }, Modifier.fillMaxWidth(), label = words.perDay, enabled = !readOnly, keyboardType = KeyboardType.Number)
                }
                if (!readOnly) {
                    KitButton(words.save, primary = true) {
                        val s = edit.copy(perHour = perHour.trim().toIntOrNull() ?: edit.perHour, perDay = perDay.trim().toIntOrNull() ?: edit.perDay)
                        scope.serverCall(ok = words.saved, after = { _: Any? -> reload() }) { api.put(path("/settings"), SignupRules.settingsBody(s)) }
                    }
                }
            }
            if (readOnly) return@Column
            SetCard {
                CardHead(words.pendingTitle)
                if (d.pending.isEmpty()) Muted(words.noPending)
                for (p in d.pending) {
                    Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                        Text(if (p.email.isEmpty()) p.username else "${p.username} · ${p.email}", fontWeight = FontWeight.SemiBold)
                        Muted("${SignupRules.waitingLine(p, words)} · ${date(p.createdAt)} · ${p.address}")
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(top = 6.dp)) {
                            SmallButton(words.approve) { scope.serverCall(after = { _: Any? -> reload() }) { api.post(path("/pending/${p.id}/approve"), JsonNull) } }
                            SmallButton(words.reject, danger = true) {
                                val q = Confirm(SignupRules.fill(words.rejectQuestion, "username" to p.username), words.rejectText, words.reject, danger = true)
                                asking = q to { scope.serverCall(after = { _: Any? -> reload() }) { api.post(path("/pending/${p.id}/reject"), JsonNull) } }
                            }
                        }
                    }
                }
            }
            SetCard {
                CardHead(words.invitesTitle) { SmallButton(words.newInvite) { formOpen = !formOpen } }
                if (formOpen) {
                    SetCard(flat = true) {
                        KitField(note, { note = it }, Modifier.fillMaxWidth(), label = words.note, placeholder = words.notePlaceholder)
                        Muted(words.role)
                        ChoiceRow(d.roles.map { it.label }, d.roles.indexOfFirst { it.id == role }, { role = d.roles[it].id })
                        for (pr in d.presets) {
                            SwitchLine(pr.label, pr.id in presets, true) { on -> presets = presets.filter { it != pr.id } + if (on) listOf(pr.id) else emptyList() }
                        }
                        Muted(words.uses)
                        ChoiceRow(listOf(words.oneTime, words.several, words.noLimit), uses, { uses = it })
                        if (uses == 1) KitField(usesN, { usesN = it }, Modifier.width(120.dp), keyboardType = KeyboardType.Number)
                        Muted(words.expires)
                        ChoiceRow(SignupRules.EXPIRY.map { it.second }, expiry, { expiry = it })
                        KitButton(words.create, primary = true) {
                            val max = when (uses) {
                                0 -> 1L

                                2 -> 0L

                                else -> SignupRules.parseUses(usesN) ?: run {
                                    Toasts.show("Enter a number of people from 2 to 100000", "err")
                                    return@KitButton
                                }
                            }
                            val body = SignupRules.inviteBody(note, role, presets, max, SignupRules.EXPIRY[expiry].first)
                            scope.serverCall<JsonObject?>(after = { v ->
                                formOpen = false
                                note = ""
                                link = SignupRules.fullLink((v?.get("link") as? JsonPrimitive)?.content ?: "", server)
                                reload()
                            }) { api.post(path("/invites"), body) as? JsonObject }
                        }
                    }
                }
                if (d.invites.isEmpty()) Muted(words.noInvites)
                val now = System.currentTimeMillis() / 1000.0
                for (i in d.invites) {
                    val (label, kind) = SignupRules.status(i.status)
                    val title = SignupRules.title(i, words)
                    val gives = (listOf(SignupRules.labelOf(d.roles, i.role)) + i.presets.map { SignupRules.labelOf(d.presets, it) }).joinToString(", ")
                    Column(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.Center) {
                            Text(title, fontWeight = FontWeight.SemiBold)
                            Badge(label, kind)
                        }
                        Muted("$gives · ${SignupRules.usesLine(i)} · ${SignupRules.expiryLine(i, now)}")
                        Muted("${i.createdBy} · ${date(i.createdAt)}")
                        Text(
                            if (i.usedBy.isEmpty()) words.unused else "${words.usedBy}: " + i.usedBy.joinToString(", ") { (n, at) -> "$n (${date(at)})" },
                            style = MaterialTheme.typography.bodySmall,
                        )
                        if (SignupRules.canRevoke(i)) {
                            SmallButton(words.revoke, Modifier.padding(top = 4.dp), danger = true) {
                                val q = Confirm(SignupRules.fill(words.revokeQuestion, "note" to title), words.revokeText, words.revoke, danger = true)
                                asking = q to { scope.serverCall(after = { _: Any? -> reload() }) { api.post(path("/invites/${i.id}/revoke"), JsonNull) } }
                            }
                        }
                    }
                }
            }
        }
    }
    asking?.let { (q, act) ->
        ConfirmDialog(q, onDismiss = { asking = null }, onConfirm = {
            asking = null
            act()
        })
    }
    link?.let { l ->
        AlertDialog(
            onDismissRequest = { link = null },
            title = { Text(words.linkTitle) },
            text = {
                Column {
                    Muted(words.linkOnce)
                    Spacer(Modifier.height(8.dp))
                    SelectionContainer { Text(l, style = MaterialTheme.typography.bodySmall) }
                }
            },
            confirmButton = {
                TextButton(onClick = { if (copyToClipboard(context, l)) Toasts.show(words.copied, "ok") }) { Text(words.copyLink) }
            },
            dismissButton = { TextButton(onClick = { link = null }) { Text("Close") } },
        )
    }
}

/** A switch with its words on the left. */
@Composable
private fun SwitchLine(label: String, on: Boolean, enabled: Boolean, onChange: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text(label, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
        Spacer(Modifier.width(8.dp))
        Switch(checked = on, onCheckedChange = onChange, enabled = enabled)
    }
}
