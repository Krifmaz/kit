package dev.krifmaz.kit.compose.settings

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
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
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.core.arr
import dev.krifmaz.kit.compose.core.bool
import dev.krifmaz.kit.compose.core.dbl
import dev.krifmaz.kit.compose.core.obj
import dev.krifmaz.kit.compose.core.objectsOf
import dev.krifmaz.kit.compose.core.str
import dev.krifmaz.kit.compose.details.CopyIconButton
import dev.krifmaz.kit.compose.details.DetailsDialog
import dev.krifmaz.kit.compose.details.copyToClipboard
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.widgets.Badge
import dev.krifmaz.kit.compose.widgets.ConfirmDialog
import dev.krifmaz.kit.compose.widgets.ErrorBox
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.RowDivider
import dev.krifmaz.kit.compose.widgets.SmallButton
import dev.krifmaz.kit.compose.widgets.Spinner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject
import java.text.DateFormat
import java.util.Date

/**
 * Settings → Diagnostics (Encore's features/settings/DiagnosticsScreen.kt; web-rs/src/settings/diagnostics.rs):
 * Copy report and Run checks, then the app's [top] part (Encore: the server's update card), every connection,
 * folder and tool check with the raw answers, the Server block and where settings come from (folded away until
 * tapped), then the app's [bottom] parts (Encore: backups and the phone app's crash reports, [AppCrashesSection]).
 *
 * @param title the page's title ("Diagnostics").
 * @param subtitle the line under the title.
 * @param onBack the back arrow (and the system Back).
 * @param load asks the server for the diagnostics answer (blocking; run on Dispatchers.IO). Errors show with Try again.
 * @param report the text of Copy report for an answer ([DiagnosticsRules.report]).
 * @param serverRows the Server block's (label, value) rows for the answer's `machine`.
 * @param appName the app's name in the summary line ("Encore 0.29.0 · running as uid 1000 · …").
 * @param top drawn under the buttons; gets "run the checks again".
 * @param afterChecks drawn after the checks, with the answer (Encore: its download folders).
 * @param bottom drawn at the end of the page.
 */
@Composable
fun DiagnosticsScreen(
    title: String,
    subtitle: String,
    onBack: () -> Unit,
    load: () -> JsonObject,
    report: (JsonObject) -> String,
    serverRows: (JsonObject) -> List<Pair<String, String>> = { emptyList() },
    appName: String = KitApp.name,
    top: @Composable ColumnScope.(rerun: () -> Unit) -> Unit = {},
    afterChecks: @Composable ColumnScope.(JsonObject) -> Unit = {},
    bottom: @Composable ColumnScope.() -> Unit = {},
) {
    val context = LocalContext.current
    var run by remember { mutableIntStateOf(0) }
    var data by remember { mutableStateOf<JsonObject?>(null) }
    var checking by remember { mutableStateOf(true) }
    var error by remember { mutableStateOf("") }
    LaunchedEffect(run) {
        checking = true
        try {
            data = withContext(Dispatchers.IO) { load() }
            error = ""
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: "Something went wrong"
        }
        checking = false
    }
    SettingsPage(title, onBack, subtitle = subtitle) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.padding(bottom = 12.dp)) {
            KitButton("Copy report", iconRes = R.drawable.kit_ic_content_copy, enabled = data != null) {
                data?.let { copyToClipboard(context, report(it)) }
            }
            KitButton("Run checks", iconRes = R.drawable.kit_ic_refresh, primary = true, enabled = !checking) { run += 1 }
        }
        top { run += 1 }
        val d = data
        when {
            checking && d == null -> Row(verticalAlignment = Alignment.CenterVertically) {
                Spinner(18.dp)
                Spacer(Modifier.width(8.dp))
                Muted("Checking…")
            }

            d == null -> ErrorBox(error) { run += 1 }

            else -> {
                DiagnosticsChecks(d, serverRows, appName)
                afterChecks(d)
                SettingsSources(d, appName)
            }
        }
        bottom()
    }
}

/** A time as "Oct 13, 2026, 9:41 PM" in the phone's language; "—" for none. */
fun diagnosticsTime(t: Double): String = if (t <= 0) "—" else DateFormat.getDateTimeInstance(DateFormat.MEDIUM, DateFormat.SHORT).format(Date((t * 1000).toLong()))

/**
 * The checks of a diagnostics answer: a summary card ("2 problems" or "All good", the app's version, uid and time),
 * the Server block, and every check under the heading of its kind ([DiagnosticsRules.CHECK_KINDS]).
 *
 * @param d the diagnostics answer.
 * @param serverRows the Server block's rows for `machine`.
 * @param appName the app's name.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun DiagnosticsChecks(d: JsonObject, serverRows: (JsonObject) -> List<Pair<String, String>>, appName: String = KitApp.name) {
    val checks = d.arr("checks").objectsOf()
    val bad = checks.count { !it.bool("ok") }
    SetCard(Modifier.padding(bottom = 4.dp)) {
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Badge(if (bad > 0) "$bad problem${if (bad > 1) "s" else ""}" else "All good", kind = if (bad > 0) "err" else "ok")
            Muted("$appName ${d.str("version")} · running as uid ${DiagnosticsRules.jsStr(d, "uid")} · ${diagnosticsTime(d.dbl("time"))}")
        }
    }
    d.obj("machine")?.let { m ->
        PageSection("Server", "What $appName sees of the machine it runs on")
        SetCard(flat = true) {
            for ((k, v) in serverRows(m)) {
                Row(Modifier.fillMaxWidth()) {
                    Muted(k, Modifier.width(150.dp))
                    Text(v, style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
                }
            }
        }
    }
    for ((kind, title) in DiagnosticsRules.CHECK_KINDS) {
        val group = checks.filter { it.str("kind") == kind }
        if (group.isEmpty()) continue
        PageSection(title)
        SetCard(flat = true) {
            group.forEachIndexed { i, c ->
                if (i > 0) RowDivider(startIndent = 0.dp)
                CheckItem(c)
            }
        }
    }
}

/**
 * "Where settings come from": every setting key with its source badge (env VAR or saved in the app) and its value.
 * The list is long, so it starts folded: the heading is a toggle with a chevron, and the rows show once it is tapped.
 * The open/closed choice lasts while the page is on screen (a Run checks keeps it).
 *
 * @param d the diagnostics answer.
 * @param appName the app's name ("saved in Encore").
 */
@Composable
fun SettingsSources(d: JsonObject, appName: String = KitApp.name) {
    val p = MaterialTheme.palette
    var open by remember { mutableStateOf(false) }
    val rows = d.arr("settings").objectsOf()
    Column(Modifier.fillMaxWidth().clickable { open = !open }) {
        PageSection("Where settings come from", "Environment variables always win over values saved in $appName") {
            Icon(
                painterResource(R.drawable.kit_ic_expand_more),
                contentDescription = if (open) "Hide settings" else "Show ${rows.size} settings",
                tint = p.muted,
                // the chevron points down while folded and flips up once the list is open
                modifier = Modifier.rotate(if (open) 180f else 0f),
            )
        }
    }
    if (!open) return
    SetCard(flat = true) {
        rows.forEachIndexed { i, s ->
            if (i > 0) RowDivider(startIndent = 0.dp)
            Column(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Mono(s.str("key"), Modifier.weight(1f))
                    if (s.str("source") == "env") Badge("env ${s.str("env")}", kind = "info") else Badge("saved in $appName")
                }
                Mono(DiagnosticsRules.jsStr(s, "value"), color = p.muted)
            }
        }
    }
}

/** One check: its dot, name, HTTP status and time, the request, the detail, and the raw response behind a link. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun CheckItem(c: JsonObject) {
    val p = MaterialTheme.palette
    val ok = c.bool("ok")
    var raw by remember { mutableStateOf(false) }
    Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
        Dot(ok, Modifier.padding(top = 6.dp))
        Spacer(Modifier.width(10.dp))
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(c.str("name"), style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Bold)
                if (SchemaRules.truthy(c["status"])) Badge("HTTP ${c.str("status")}", kind = if (ok) "ok" else "err")
                if (!DiagnosticsRules.nullish(c, "ms")) Muted("${c.str("ms")} ms", color = p.faint)
            }
            if (c.str("request").isNotEmpty()) Mono(c.str("request"), color = p.faint)
            Muted(c.str("detail"), color = if (ok) p.muted else KitStatusColors.Err)
            if (c.str("body").isNotEmpty()) {
                TextLink("Raw response", color = p.muted) { raw = !raw }
                if (raw) Mono(c.str("body").take(4000), color = p.muted)
            }
        }
    }
}

/**
 * "App crashes": the crash reports the phone app sent (`GET /api/diagnostics/client-crash`, newest 50), each with
 * its first line (opens the whole report with Copy), kind, device, version, OS, user and time; Clear removes them all.
 *
 * @param load asks for the reports (`{crashes: [...]}`); blocking, run on Dispatchers.IO.
 * @param clear removes them all (`DELETE /api/diagnostics/client-crash`); blocking.
 */
@Composable
fun AppCrashesSection(load: () -> JsonObject, clear: () -> Unit) {
    val p = MaterialTheme.palette
    val scope = rememberCoroutineScope()
    var version by remember { mutableIntStateOf(0) }
    var list by remember { mutableStateOf<List<JsonObject>?>(null) }
    var error by remember { mutableStateOf("") }
    var asking by remember { mutableStateOf(false) }
    // the crash opened whole: (kind, text)
    var whole by remember { mutableStateOf<Pair<String, String>?>(null) }
    LaunchedEffect(version) {
        try {
            list = withContext(Dispatchers.IO) { load() }.arr("crashes").objectsOf()
            error = ""
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: "Something went wrong"
        }
    }
    PageSection("App crashes") { SmallButton("Clear", enabled = !list.isNullOrEmpty()) { asking = true } }
    SetCard(flat = true) {
        Muted("The phone app sends a report the next time it opens after it crashed or the system closed it. The newest 50 are kept.")
        val l = list
        when {
            l == null && error.isNotEmpty() -> Muted(error)

            l == null -> Spinner(16.dp)

            l.isEmpty() -> Muted("No app crashes reported.")

            else -> l.forEachIndexed { i, c ->
                if (i > 0) RowDivider(startIndent = 0.dp)
                var open by remember(c) { mutableStateOf(false) }
                val first = c.str("first_line").ifEmpty { "Crash" }
                val kind = DiagnosticsRules.crashKind(c.str("kind"))
                val meta = listOf("device", "app_version", "os", "user").map { c.str(it) }.filter { it.isNotEmpty() }
                // the whole report to read and copy: the cut-off first line opens it, Copy takes all of it
                val full = DiagnosticsRules.crashText(first, kind, meta.joinToString(" · "), diagnosticsTime(c.dbl("time")), c.str("text"))
                Row(Modifier.fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.Top) {
                    Dot(false, Modifier.padding(top = 6.dp))
                    Spacer(Modifier.width(10.dp))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            Text(
                                first,
                                style = MaterialTheme.typography.bodyMedium,
                                fontWeight = FontWeight.Bold,
                                maxLines = 2,
                                modifier = Modifier.weight(1f, fill = false).clickable(onClickLabel = "Details") { whole = kind to full },
                            )
                            Badge(kind)
                            CopyIconButton(full, size = 28.dp)
                        }
                        Muted("${meta.joinToString(" · ")} · ${diagnosticsTime(c.dbl("time"))}")
                        TextLink("Full report", color = p.muted) { open = !open }
                        if (open) Mono(c.str("text").take(20000), color = p.muted)
                    }
                }
            }
        }
    }
    whole?.let { (kind, text) -> DetailsDialog(kind, text, onDismiss = { whole = null }) }
    if (asking) {
        ConfirmDialog("Clear app crashes?", "All crash reports from the phone app are removed.", "Clear", danger = true, onDismiss = { asking = false }) {
            scope.serverCall(after = { _: Unit -> version += 1 }, onError = { msg ->
                Toasts.show(msg, "err")
                version += 1
            }) { clear() }
        }
    }
}
