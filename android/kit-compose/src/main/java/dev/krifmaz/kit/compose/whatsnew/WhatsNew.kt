package dev.krifmaz.kit.compose.whatsnew

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
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
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.widgets.Badge
import dev.krifmaz.kit.compose.widgets.CardBox
import dev.krifmaz.kit.compose.widgets.ErrorBox
import dev.krifmaz.kit.compose.widgets.Gutter
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * After signing in: asks the server whether something is new since this account last saw it and, when it is, shows
 * [WhatsNewPopup] once and marks this version seen (web-rs/src/whats_new.rs `start`). Put it once in the signed-in
 * root; it shows nothing when the server can't be reached.
 *
 * @param backend the app's changelog calls.
 * @param onFullChangelog opens Settings → What's new ([ChangelogScreen]).
 */
@Composable
fun WhatsNewOnStart(backend: ChangelogBackend, onFullChangelog: () -> Unit) {
    var releases by remember { mutableStateOf<List<WhatsNewRelease>>(emptyList()) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(backend) {
        val new = withContext(Dispatchers.IO) { runCatching { backend.new() }.getOrNull() } ?: return@LaunchedEffect
        if (!WhatsNewRules.shouldShow(new)) return@LaunchedEffect
        releases = new.releases
        withContext(Dispatchers.IO) { runCatching { backend.markSeen() } }
    }
    if (releases.isNotEmpty()) {
        WhatsNewPopup(
            releases = releases,
            onPopupChange = { on -> savePopup(scope, backend, on) },
            onFullChangelog = {
                releases = emptyList()
                onFullChangelog()
            },
            onDismiss = { releases = emptyList() },
        )
    }
}

/** Saves the popup switch and says so in a toast (the server's words on failure). */
private fun savePopup(scope: CoroutineScope, backend: ChangelogBackend, on: Boolean) {
    scope.launch {
        try {
            withContext(Dispatchers.IO) { backend.setPopup(on) }
            Toasts.show(WhatsNewRules.popupSaved(on), "ok")
        } catch (e: Exception) {
            Toasts.show(e.message ?: e.toString(), "err")
        }
    }
}

/**
 * The What's new popup: each new release's title, summary and highlights, the switch "Show this after updates",
 * Full changelog and Close.
 *
 * @param releases the new releases, newest first.
 * @param onPopupChange the switch was turned on or off.
 * @param onFullChangelog Full changelog was tapped (the popup closes).
 * @param onDismiss the popup closed.
 */
@Composable
fun WhatsNewPopup(releases: List<WhatsNewRelease>, onPopupChange: (Boolean) -> Unit, onFullChangelog: () -> Unit, onDismiss: () -> Unit) {
    val p = MaterialTheme.palette
    var on by remember { mutableStateOf(true) }
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = p.panel,
        titleContentColor = p.text,
        textContentColor = p.text,
        title = { Text(WhatsNewRules.TITLE) },
        text = {
            Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(18.dp)) {
                for (r in releases) ReleaseSummary(r)
                SwitchLine(WhatsNewRules.POPUP_SWITCH, on) {
                    on = it
                    onPopupChange(it)
                }
            }
        },
        confirmButton = {
            TextButton(onClick = onDismiss) { Text(WhatsNewRules.CLOSE, color = p.text, fontWeight = FontWeight.SemiBold) }
        },
        dismissButton = {
            TextButton(onClick = onFullChangelog) { Text(WhatsNewRules.FULL_CHANGELOG, color = p.muted) }
        },
    )
}

/** A release in the popup: its title (h3), a muted summary and the highlights as bullets. */
@Composable
private fun ReleaseSummary(r: WhatsNewRelease) {
    val p = MaterialTheme.palette
    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(r.title, style = MaterialTheme.typography.titleMedium)
        if (r.summary.isNotEmpty()) Text(r.summary, style = MaterialTheme.typography.bodyMedium, color = p.muted)
        for (h in r.highlights) Bullet(h)
    }
}

/** A bullet: "•" and the words, after an optional bold [lead] (the web's `<li><strong>lead</strong> text</li>`). */
@Composable
private fun Bullet(text: String, lead: String = "") {
    val words =
        buildAnnotatedString {
            if (lead.isNotEmpty()) {
                withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(lead) }
                append(" ")
            }
            append(text)
        }
    Row(Modifier.padding(start = 4.dp)) {
        Text("•", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.width(16.dp))
        Text(words, style = MaterialTheme.typography.bodyMedium)
    }
}

/** A switch with its label (the web's `.switch`). */
@Composable
private fun SwitchLine(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    val p = MaterialTheme.palette
    Row(Modifier.fillMaxWidth().clickable { onChange(!checked) }.padding(vertical = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        Switch(
            checked = checked,
            onCheckedChange = onChange,
            colors = SwitchDefaults.colors(checkedTrackColor = p.accent, checkedThumbColor = p.onAccent),
        )
        Spacer(Modifier.width(12.dp))
        Text(label, style = MaterialTheme.typography.bodyMedium)
    }
}

/**
 * Settings → What's new: a card with the server's version and the popup switch, then every release with notes as a
 * card that opens to its notes (the newest open).
 *
 * @param backend the app's changelog calls.
 * @param modifier the screen's modifier.
 */
@Composable
fun ChangelogScreen(backend: ChangelogBackend, modifier: Modifier = Modifier) {
    val p = MaterialTheme.palette
    val scope = rememberCoroutineScope()
    var retry by remember { mutableIntStateOf(0) }
    var log by remember { mutableStateOf<Changelog?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(backend, retry) {
        error = null
        try {
            log = withContext(Dispatchers.IO) { backend.all() }
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
    val failed = error
    val c = log
    when {
        failed != null -> ErrorBox(failed) { retry++ }

        c == null ->
            Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
                CircularProgressIndicator(color = p.accent, trackColor = p.border)
            }

        else -> {
            var on by remember(c) { mutableStateOf(c.popup) }
            LazyColumn(modifier.fillMaxSize().padding(horizontal = Gutter), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                item(key = "head", contentType = "head") {
                    CardBox {
                        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(WhatsNewRules.TITLE, style = MaterialTheme.typography.titleLarge)
                            Text(WhatsNewRules.settingsIntro(KitApp.name, c.version), style = MaterialTheme.typography.bodySmall, color = p.muted)
                            SwitchLine(WhatsNewRules.SETTINGS_SWITCH, on) {
                                on = it
                                savePopup(scope, backend, it)
                            }
                        }
                    }
                }
                if (c.releases.isEmpty()) {
                    item(key = "none", contentType = "none") {
                        CardBox { Text(WhatsNewRules.NO_NOTES, color = p.muted) }
                    }
                }
                itemsIndexed(c.releases, key = { _, r -> r.version }, contentType = { _, _ -> "release" }) { i, r -> ReleaseCard(r, startOpen = i == 0) }
            }
        }
    }
}

/** One release in Settings → What's new: title, version badge and chevron; opens to the notes. */
@Composable
private fun ReleaseCard(r: ChangelogRelease, startOpen: Boolean) {
    val p = MaterialTheme.palette
    var open by remember { mutableStateOf(startOpen) }
    CardBox(Modifier.animateContentSize()) {
        Column {
            Row(Modifier.fillMaxWidth().clickable { open = !open }, verticalAlignment = Alignment.CenterVertically) {
                Text(r.title, style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
                Badge(r.version)
                Spacer(Modifier.width(8.dp))
                Icon(
                    painterResource(R.drawable.kit_ic_expand_more),
                    contentDescription = null,
                    tint = p.muted,
                    modifier = Modifier.size(20.dp).rotate(if (open) 180f else 0f),
                )
            }
            if (open) {
                Column(Modifier.padding(top = 12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    for (g in WhatsNewRules.group(r.blocks)) NoteGroupView(g)
                }
            }
        }
    }
}

/** One group of a release's notes. */
@Composable
private fun NoteGroupView(g: NoteGroup) {
    when (g) {
        is NoteGroup.Heading -> Text(g.text, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 10.dp))

        is NoteGroup.Paragraph -> Text(g.text, style = MaterialTheme.typography.bodyMedium)

        is NoteGroup.Bullets -> Column(verticalArrangement = Arrangement.spacedBy(6.dp)) { for (b in g.items) Bullet(b.text, b.lead) }

        is NoteGroup.Table ->
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                for (row in g.rows) {
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        for (cell in row.cells) {
                            Text(
                                cell,
                                style = MaterialTheme.typography.bodySmall,
                                fontWeight = if (row.head) FontWeight.SemiBold else null,
                                modifier = Modifier.weight(1f),
                            )
                        }
                    }
                }
            }
    }
}
