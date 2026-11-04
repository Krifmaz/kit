package dev.krifmaz.kit.demo

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.settings.AppCrashesSection
import dev.krifmaz.kit.compose.settings.CardSlots
import dev.krifmaz.kit.compose.settings.DiagnosticsRules
import dev.krifmaz.kit.compose.settings.DiagnosticsScreen
import dev.krifmaz.kit.compose.settings.FieldBlock
import dev.krifmaz.kit.compose.settings.Muted
import dev.krifmaz.kit.compose.settings.SchemaApi
import dev.krifmaz.kit.compose.settings.SchemaFormState
import dev.krifmaz.kit.compose.settings.SchemaPage
import dev.krifmaz.kit.compose.settings.SettingsPage
import dev.krifmaz.kit.compose.settings.loadState
import dev.krifmaz.kit.compose.widgets.ChoiceRow
import dev.krifmaz.kit.compose.widgets.ErrorBox
import dev.krifmaz.kit.compose.widgets.Gutter
import dev.krifmaz.kit.compose.widgets.Spinner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject

/**
 * Server settings: the demo server's schema (`assets/settings-schema.json`) drawn as cards, one page at a time,
 * with every kind of field, a `when` rule, a saved secret to Show and Copy, Show advanced, a framed block of the
 * tunnel's fields, and each card's Test and Save.
 */
@Composable
fun DemoSettings(app: DemoApp, onBack: () -> Unit) {
    val api = remember(app) { SchemaApi(app.server.kit, app.server.http) }
    var state by remember { mutableStateOf<SchemaFormState?>(null) }
    var error by remember { mutableStateOf("") }
    var round by remember { mutableIntStateOf(0) }
    var page by remember { mutableIntStateOf(0) }
    LaunchedEffect(round) {
        error = ""
        try {
            state = withContext(Dispatchers.IO) { api.loadState(canReveal = true) }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: "Something went wrong"
        }
    }
    SettingsPage("Server settings", onBack, gutter = false) {
        val s = state
        when {
            error.isNotEmpty() -> ErrorBox(error) { round += 1 }

            s == null -> Box(Modifier.fillMaxWidth().padding(32.dp), contentAlignment = Alignment.Center) { Spinner() }

            s.schema.sections.isEmpty() -> Muted("This server has no settings to change here.", Modifier.padding(horizontal = Gutter))

            else -> {
                val sections = s.schema.sections
                val i = page.coerceAtMost(sections.lastIndex)
                if (sections.size > 1) ChoiceRow(sections.map { it.title }, i, { page = it })
                SchemaPage(
                    s,
                    api,
                    sections[i],
                    Modifier.padding(horizontal = Gutter, vertical = 12.dp),
                    slots = { g -> if (g.id == "tunnel") CardSlots(block = FieldBlock("tunnel_", "Connection")) else CardSlots.None },
                )
            }
        }
    }
}

/** Diagnostics: the demo server's checks with Copy report and Run checks, then the app's crash reports. */
@Composable
fun DemoDiagnostics(app: DemoApp, onBack: () -> Unit) {
    val machine = { m: JsonObject -> listOf("System" to m.str("os"), "Processors" to m.str("cpus"), "Memory" to m.str("memory")) }
    DiagnosticsScreen(
        title = "Diagnostics",
        subtitle = "What the server checks, with the raw answers. Copy report puts it all on the clipboard.",
        onBack = onBack,
        load = { app.server.getObject(KitApp.routes.diagnostics) },
        report = { DiagnosticsRules.report(it, serverRows = machine) },
        serverRows = machine,
        bottom = {
            AppCrashesSection(
                load = { app.server.getObject(KitApp.routes.clientCrash) },
                clear = { app.server.delete(KitApp.routes.clientCrash) },
            )
        },
    )
}
