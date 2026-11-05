package dev.krifmaz.kit.compose.setup

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.settings.Muted
import dev.krifmaz.kit.compose.settings.serverCall
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.KitField
import kotlinx.serialization.json.JsonObject

/**
 * Whether the server wants its owner made on "Create your account" (`GET KitApp.routes.setup`). Blocking: call it
 * on Dispatchers.IO. A server without setup, or one that can't answer, gives false: show the sign-in page as before.
 */
fun setupNeeded(api: KitApi): Boolean = try {
    SetupRules.needed(api.get(KitApp.routes.setup) as? JsonObject)
} catch (_: Exception) {
    false
}

/**
 * "Create your account" (contracts/setup.md): the setup code from the server's log, a username, an optional email
 * and the password twice. The app puts it where its sign-in form goes, under its name. Once the server made the
 * account, [onDone] gets the username and password, and the app signs in with them as its sign-in page does.
 */
@Composable
fun SetupForm(api: KitApi, onDone: (username: String, password: String) -> Unit, words: SetupWords = SetupWords()) {
    val scope = rememberCoroutineScope()
    var code by remember { mutableStateOf("") }
    var username by remember { mutableStateOf("") }
    var email by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var again by remember { mutableStateOf("") }
    var error by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }

    fun submit() {
        if (busy) return
        val bad = SetupRules.checkForm(code, username, password, again, words)
        if (bad != null) {
            error = bad
            return
        }
        error = ""
        busy = true
        val (name, pw) = username.trim() to password
        scope.serverCall<Unit>(onError = {
            error = it
            busy = false
        }, after = {
            busy = false
            onDone(name, pw)
        }) {
            api.post(KitApp.routes.setup, SetupRules.body(code, username, password, email))
            Unit
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(words.title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
        Muted(words.intro)
        KitField(code, { code = it }, Modifier.fillMaxWidth(), label = words.code, placeholder = "ABCD-EFGH-JKMN", imeAction = ImeAction.Next)
        Muted(words.codeHint)
        KitField(username, { username = it }, Modifier.fillMaxWidth(), label = words.username, imeAction = ImeAction.Next)
        KitField(
            email,
            { email = it },
            Modifier.fillMaxWidth(),
            label = "${words.email} (${words.emailOptional})",
            keyboardType = KeyboardType.Email,
            imeAction = ImeAction.Next,
        )
        KitField(
            password,
            { password = it },
            Modifier.fillMaxWidth(),
            label = words.password,
            password = true,
            keyboardType = KeyboardType.Password,
            imeAction = ImeAction.Next,
        )
        KitField(
            again,
            { again = it },
            Modifier.fillMaxWidth(),
            label = words.again,
            password = true,
            keyboardType = KeyboardType.Password,
            imeAction = ImeAction.Done,
            onAction = ::submit,
        )
        if (error.isNotEmpty()) Text(error, color = KitStatusColors.Err)
        KitButton(if (busy) words.creating else words.create, Modifier.fillMaxWidth(), primary = true, enabled = !busy, onClick = ::submit)
    }
}
