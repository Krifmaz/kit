package dev.krifmaz.kit.compose.settings

import android.app.Activity
import android.content.Context
import android.content.ContextWrapper
import android.view.Window
import android.view.WindowManager
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

// The pieces the Settings pages share (Encore's features/settings/SettingsWidgets.kt): the page frame with its back
// arrow, the web's cards and section heads, small muted and monospaced text, status dots, text links, the secure
// window for secrets shown as text, and a server call run from a button.

/** One Settings page: a back arrow and the title, then [content] in a scrolling column with the page gutter. */
@Composable
fun SettingsPage(
    title: String,
    onBack: () -> Unit,
    subtitle: String = "",
    /** false: the content brings its own side gutter (sections shared with other screens) */
    gutter: Boolean = true,
    actions: @Composable () -> Unit = {},
    content: @Composable ColumnScope.() -> Unit,
) {
    BackHandler(onBack = onBack)
    val p = MaterialTheme.palette
    Column(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(top = 4.dp, end = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack) { Icon(painterResource(R.drawable.kit_ic_arrow_back), contentDescription = "Back") }
            Text(title, style = MaterialTheme.typography.titleLarge, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
            actions()
        }
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = if (gutter) 16.dp else 0.dp)) {
            if (subtitle.isNotEmpty()) Text(subtitle, style = MaterialTheme.typography.bodySmall, color = p.muted, modifier = Modifier.padding(bottom = 12.dp))
            content()
            Spacer(Modifier.height(32.dp))
        }
    }
}

/** The web's `.card` (or `.card.flat` for the blocks inside a card): a bordered panel with its content in a column. */
@Composable
fun SetCard(modifier: Modifier = Modifier, flat: Boolean = false, content: @Composable ColumnScope.() -> Unit) {
    val p = MaterialTheme.palette
    val shape = RoundedCornerShape(if (flat) 10.dp else 14.dp)
    Column(
        modifier
            .fillMaxWidth()
            .clip(shape)
            .background(if (flat) p.panel2 else p.panel)
            .border(1.dp, p.border, shape)
            .padding(horizontal = 14.dp, vertical = if (flat) 12.dp else 14.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = content,
    )
}

/** A card's h2 with an optional small button on the right. */
@Composable
fun CardHead(title: String, modifier: Modifier = Modifier, trailing: @Composable () -> Unit = {}) {
    Row(modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
        trailing()
    }
}

/** The web's `.section-head` inside a page: an h2 and an optional line under it. */
@Composable
fun PageSection(title: String, subtitle: String = "", trailing: @Composable () -> Unit = {}) {
    Row(Modifier.fillMaxWidth().padding(top = 22.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleLarge)
            if (subtitle.isNotEmpty()) Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.palette.muted)
        }
        trailing()
    }
}

/** Small muted text (`.small.muted`). */
@Composable
fun Muted(text: String, modifier: Modifier = Modifier, color: Color = MaterialTheme.palette.muted) {
    if (text.isEmpty()) return
    Text(text, style = MaterialTheme.typography.bodySmall, color = color, modifier = modifier)
}

/** Monospaced small text (`.mono`, `.code`). */
@Composable
fun Mono(text: String, modifier: Modifier = Modifier, color: Color = MaterialTheme.palette.text) {
    Text(text, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, color = color, modifier = modifier)
}

/** The status dot of a list row (`.dot ok|err`). */
@Composable
fun Dot(ok: Boolean, modifier: Modifier = Modifier) {
    Box(modifier.size(9.dp).clip(CircleShape).background(if (ok) KitStatusColors.Ok else KitStatusColors.Err))
}

/** A link-like text button (`a.small`). */
@Composable
fun TextLink(text: String, modifier: Modifier = Modifier, color: Color = MaterialTheme.palette.accent, enabled: Boolean = true, onClick: () -> Unit) {
    Text(
        text,
        style = MaterialTheme.typography.bodySmall,
        color = color,
        modifier = modifier.clip(
            RoundedCornerShape(4.dp),
        ).clickable(enabled = enabled, onClick = onClick).padding(vertical = 4.dp, horizontal = 2.dp).alpha(if (enabled) 1f else 0.5f),
    )
}

/** Windows kept secure, with how many places asked for it (the flag goes when the last one lets go). */
private val secureHolds = java.util.WeakHashMap<Window, Int>()

/**
 * While [on] (a secret is shown as text), the window is kept out of screenshots and of the Recents thumbnail
 * (FLAG_SECURE), as a password field would be.
 */
@Composable
fun SecureWindow(on: Boolean) {
    if (!on) return
    val context = LocalContext.current
    DisposableEffect(context) {
        val window = context.findActivity()?.window
        if (window != null) {
            val n = secureHolds[window] ?: 0
            secureHolds[window] = n + 1
            if (n == 0) window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
        onDispose {
            if (window != null) {
                val left = (secureHolds[window] ?: 1) - 1
                if (left <= 0) {
                    secureHolds.remove(window)
                    window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                } else {
                    secureHolds[window] = left
                }
            }
        }
    }
}

/**
 * Runs a blocking server call on the IO dispatcher from a button: [ok] is toasted when it worked and [after] gets
 * the answer; the server's words are toasted (or given to [onError]) when it did not.
 */
fun <T> CoroutineScope.serverCall(ok: String = "", onError: ((String) -> Unit)? = null, after: (T) -> Unit = {}, call: () -> T) {
    launch {
        try {
            val r = withContext(Dispatchers.IO) { call() }
            if (ok.isNotEmpty()) Toasts.show(ok, "ok")
            after(r)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            val msg = e.message ?: "Something went wrong"
            if (onError != null) onError(msg) else Toasts.show(msg, "err")
        }
    }
}

private fun Context.findActivity(): Activity? {
    var c: Context? = this
    while (c is ContextWrapper) {
        if (c is Activity) return c
        c = c.baseContext
    }
    return null
}
