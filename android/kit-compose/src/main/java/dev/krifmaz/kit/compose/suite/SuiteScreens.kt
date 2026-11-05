package dev.krifmaz.kit.compose.suite

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.addPathNodes
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.widgets.CardBox
import dev.krifmaz.kit.compose.widgets.KitField
import dev.krifmaz.kit.compose.widgets.SmallButton
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject

/**
 * An icon of the suite (SVG path data in a 24×24 box) as an [ImageVector] drawn as a 2 dp round stroke, like the
 * kit's own icons; null when the path data can't be read.
 */
fun pathIcon(d: String): ImageVector? = try {
    if (d.isBlank()) {
        null
    } else {
        ImageVector.Builder(defaultWidth = 24.dp, defaultHeight = 24.dp, viewportWidth = 24f, viewportHeight = 24f)
            .addPath(
                pathData = addPathNodes(d),
                fill = null,
                stroke = SolidColor(Color.Black),
                strokeLineWidth = 2f,
                strokeLineCap = StrokeCap.Round,
                strokeLineJoin = StrokeJoin.Round,
            )
            .build()
    }
} catch (_: Exception) {
    null
}

/** A suite icon in [tint] (nothing for path data that can't be drawn). */
@Composable
fun PathIcon(d: String, tint: Color, modifier: Modifier = Modifier.size(18.dp)) {
    val v = remember(d) { pathIcon(d) } ?: return
    Icon(v, contentDescription = null, tint = tint, modifier = modifier)
}

/**
 * The shared sidebar as the screens see it: the entries, and the draft while the person edits them. [load] asks
 * the server (GET [SuiteRules.SIDEBAR_PATH]) and [save] keeps the order (PUT); both block, so they run on
 * Dispatchers.IO. Make it with [rememberSidebar].
 */
@Stable
class SidebarState(private val api: KitApi) {
    /** Every entry in the person's order, hidden ones marked. */
    var items by mutableStateOf<List<SidebarItem>>(emptyList())
        private set

    /** The entries being edited (null while not editing). */
    var draft by mutableStateOf<List<SidebarItem>?>(null)

    /** The last failure, in the server's words ("" for none). */
    var error by mutableStateOf("")
        private set

    /** Asks the server now; a failed ask keeps the last answer. */
    suspend fun refresh() {
        try {
            items = SuiteRules.itemsOf(withContext(Dispatchers.IO) { api.get(SuiteRules.SIDEBAR_PATH) } as? JsonObject)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // offline mode says why; the sidebar keeps what it had
        }
    }

    /** Keeps [body] (the draft's order, or `{}` to reset) and stops editing. */
    suspend fun save(body: JsonObject) {
        try {
            items = SuiteRules.itemsOf(withContext(Dispatchers.IO) { api.put(SuiteRules.SIDEBAR_PATH, body) } as? JsonObject)
            draft = null
            error = ""
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            error = e.message ?: e.toString()
        }
    }
}

/** A [SidebarState] on [api], asked once when first shown. */
@Composable
fun rememberSidebar(api: KitApi): SidebarState {
    val s = remember(api) { SidebarState(api) }
    LaunchedEffect(s) { s.refresh() }
    return s
}

/**
 * The shared sidebar's entries: each visible one (a local one calls [onLocal] with its path, another app's calls
 * [onBrowse] with its address), then Edit sidebar: move up and down, hide and show, Reset order, Done.
 */
@Composable
fun SuiteNav(state: SidebarState, active: String, onLocal: (String) -> Unit, onBrowse: (String) -> Unit, words: SuiteWords = SuiteWords()) {
    val p = MaterialTheme.palette
    val scope = rememberCoroutineScope()
    val draft = state.draft
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        if (draft != null) {
            draft.forEachIndexed { n, i ->
                val color = if (i.hidden) p.faint else p.text
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    PathIcon(i.icon, color)
                    Text(i.title, color = color, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                    SmallButton("↑", enabled = n > 0) { state.draft = SuiteRules.move(draft, i.key, -1) }
                    SmallButton("↓", enabled = n < draft.size - 1) { state.draft = SuiteRules.move(draft, i.key, 1) }
                    SmallButton(if (i.hidden) words.show else words.hide) { state.draft = SuiteRules.toggle(draft, i.key) }
                }
            }
            if (state.error.isNotEmpty()) Text(state.error, color = KitStatusColors.Err, style = MaterialTheme.typography.bodySmall)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp, Alignment.End)) {
                SmallButton(words.reset) { scope.launch { state.save(JsonObject(emptyMap())) } }
                SmallButton(words.done, primary = true) { scope.launch { state.save(SuiteRules.prefsOf(draft)) } }
            }
        } else {
            for (i in SuiteRules.visible(state.items)) {
                val on = i.local && i.path == active
                Row(
                    Modifier
                        .fillMaxWidth()
                        .clickable { if (i.local) onLocal(i.path) else onBrowse(i.url) }
                        .padding(horizontal = 12.dp, vertical = 10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    PathIcon(i.icon, if (on) p.text else p.muted)
                    Text(i.title, color = if (on) p.text else p.muted, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                    if (!i.local) Text(i.appName, color = p.faint, style = MaterialTheme.typography.labelSmall)
                }
            }
            if (state.items.isNotEmpty()) {
                SmallButton(words.edit, modifier = Modifier.padding(start = 12.dp, top = 6.dp)) { state.draft = state.items }
            }
        }
    }
}

/** Search groups: each app's icon and name ([title] filled with `{app}`), its hits ([onOpen] with the address), or why it failed. */
@Composable
fun SearchGroups(groups: List<SearchGroup>, title: String, onOpen: (SearchHit, SearchGroup) -> Unit) {
    val p = MaterialTheme.palette
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        for (g in groups) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                PathIcon(g.icon, p.muted, Modifier.size(16.dp))
                Text(SuiteRules.fill(title, "app" to g.appName), style = MaterialTheme.typography.titleSmall, color = p.text)
            }
            if (g.error.isNotEmpty()) Text(g.error, color = KitStatusColors.Err, style = MaterialTheme.typography.bodySmall)
            if (g.results.isNotEmpty()) {
                CardBox(flat = true) {
                    Column {
                        for (h in g.results) {
                            Column(Modifier.fillMaxWidth().clickable { onOpen(h, g) }.padding(horizontal = 14.dp, vertical = 10.dp)) {
                                Text(h.title, color = p.text, maxLines = 1, overflow = TextOverflow.Ellipsis)
                                if (h.subtitle.isNotEmpty()) Text(h.subtitle, color = p.muted, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            }
        }
    }
}

/** The search across apps: a field, then one group per app (asked on Enter). */
@Composable
fun SuiteSearch(api: KitApi, onOpen: (SearchHit, SearchGroup) -> Unit, words: SuiteWords = SuiteWords()) {
    val p = MaterialTheme.palette
    val scope = rememberCoroutineScope()
    var text by remember { mutableStateOf("") }
    var groups by remember { mutableStateOf<List<SearchGroup>?>(null) }
    var error by remember { mutableStateOf("") }
    Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
        KitField(text, { text = it }, Modifier.fillMaxWidth(), placeholder = words.searchPlaceholder, onAction = {
            val q = text
            if (q.isBlank()) {
                groups = null
            } else {
                scope.launch {
                    try {
                        groups = SuiteRules.groupsOf(withContext(Dispatchers.IO) { api.get(SuiteRules.searchPath(q)) } as? JsonObject)
                        error = ""
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        error = e.message ?: e.toString()
                    }
                }
            }
        })
        if (error.isNotEmpty()) Text(error, color = KitStatusColors.Err, style = MaterialTheme.typography.bodySmall)
        val g = groups
        when {
            g == null -> Spacer(Modifier.size(0.dp))
            g.isEmpty() -> Text(SuiteRules.fill(words.noResults, "query" to text.trim()), color = p.muted)
            else -> SearchGroups(g, "{app}", onOpen)
        }
    }
}

/** What other apps have for [query], under "Related in <app>"; nothing while they have nothing. */
@Composable
fun Related(api: KitApi, query: String, onOpen: (SearchHit, SearchGroup) -> Unit, words: SuiteWords = SuiteWords()) {
    var groups by remember(query) { mutableStateOf<List<SearchGroup>>(emptyList()) }
    LaunchedEffect(query) {
        if (query.isNotBlank()) {
            groups = try {
                SuiteRules.groupsOf(withContext(Dispatchers.IO) { api.get(SuiteRules.searchPath(query, "others")) } as? JsonObject).filter { it.results.isNotEmpty() }
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                emptyList()
            }
        }
    }
    if (groups.isNotEmpty()) SearchGroups(groups, words.relatedIn, onOpen)
}
