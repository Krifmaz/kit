package dev.krifmaz.kit.compose.offline

import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.KitRoutes
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.sizeIn
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.widgets.Gutter
import dev.krifmaz.kit.compose.widgets.KitButton
import dev.krifmaz.kit.compose.widgets.SmallButton
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

// The offline banner and the sheet it opens, and the health check that
// decides offline mode (OfflineMode.kt `probe`). The app keeps the state (offline or not, a check on its way, the
// reason) and passes it in; the words that say what still works offline are the app's own.

/** The sheet's last line: how often the app asks the server again. */
const val RETRY_NOTE = "The app asks the server again every 30 seconds."

/**
 * The banner's first line and Retry, above the app while offline: "[title] · [line]" with Retry.
 *
 * It is meant as the Scaffold's top bar, which in an edge-to-edge window owns the status bar inset: the warn tint runs
 * under the status bar, and the text and Retry sit below it. Two short lines instead of one long wrapped sentence, a
 * 48 dp Retry, and a tap anywhere else on the banner calls [onDetails] (open [OfflineSheet] with the full message,
 * why the server can't be reached, and Retry). Draws nothing when not [active].
 *
 * @param active offline mode is on.
 * @param trying a check is on its way (Retry reads "Trying…" and can't be tapped).
 * @param title the first line, short enough for one line on the narrowest phone ("Offline").
 * @param line the second line, ellipsized: what still works, and that the banner opens the details.
 * @param onRetry Retry: ask the server again now.
 * @param onDetails a tap on the banner: open the details.
 */
@Composable
fun OfflineBanner(active: Boolean, trying: Boolean, title: String, line: String, onRetry: () -> Unit, onDetails: () -> Unit) {
    if (!active) return
    val p = MaterialTheme.palette
    Row(
        Modifier
            .fillMaxWidth()
            .background(KitStatusColors.Warn.copy(alpha = 0.16f))
            // after the background: the tint fills the status bar, the content stays below it
            .windowInsetsPadding(WindowInsets.statusBars)
            .clickable(onClickLabel = "Offline details") { onDetails() }
            .heightIn(min = 48.dp)
            .padding(start = Gutter, end = 8.dp, top = 4.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(painterResource(R.drawable.kit_ic_cloud_off), contentDescription = null, tint = p.text, modifier = Modifier.size(20.dp))
        Spacer(Modifier.width(12.dp))
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.labelLarge, fontWeight = FontWeight.SemiBold, color = p.text, maxLines = 1)
            Text(line, style = MaterialTheme.typography.bodySmall, color = p.muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        Spacer(Modifier.width(8.dp))
        RetryButton(trying, onRetry)
    }
}

/**
 * The banner and its sheet in one: [OfflineBanner], opening [OfflineSheet] on a tap.
 *
 * @param active offline mode is on.
 * @param trying a check is on its way.
 * @param title the banner's first line.
 * @param line the banner's second line.
 * @param message the full offline message, at the top of the sheet.
 * @param reason why the last check failed ([OfflineReason]).
 * @param status the sheet's status line (what is saved here, what waits to be sent); "" for none.
 * @param onRetry Retry: ask the server again now.
 */
@Composable
fun OfflineBanner(active: Boolean, trying: Boolean, title: String, line: String, message: String, reason: String, status: String, onRetry: () -> Unit) {
    // the sheet with the full message, opened by a tap anywhere on the banner but Retry
    var details by remember { mutableStateOf(false) }
    OfflineBanner(active, trying, title, line, onRetry, onDetails = { details = true })
    if (details) OfflineSheet(active, trying, message, reason, status, onRetry, onDismiss = { details = false })
}

/**
 * The banner's and the sheet's Retry: a 48 dp touch target around the small button (Material's minimum), reading
 * "Trying…" and not tappable while a check is on its way.
 */
@Composable
private fun RetryButton(trying: Boolean, onRetry: () -> Unit) {
    Box(Modifier.sizeIn(minWidth = 48.dp, minHeight = 48.dp), contentAlignment = Alignment.Center) {
        SmallButton(if (trying) "Trying…" else "Retry", iconRes = R.drawable.kit_ic_refresh, enabled = !trying, onClick = onRetry)
    }
}

/**
 * The sheet the offline banner opens: the full offline message, why the last check failed, the app's status line
 * (how much is saved here and what waits to be sent), how often the app asks again, Retry, and Close. Closes by
 * itself once the server answers again ([active] turns false).
 *
 * @param active offline mode is on.
 * @param trying a check is on its way.
 * @param message the full offline message.
 * @param reason why the last check failed ([OfflineReason]); shown as "Why: …".
 * @param status the status line; "" for none.
 * @param onRetry Retry: ask the server again now.
 * @param onDismiss Close, or a swipe down.
 * @param retryNote the last line, how often the app asks again ([RETRY_NOTE]).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun OfflineSheet(active: Boolean, trying: Boolean, message: String, reason: String, status: String, onRetry: () -> Unit, onDismiss: () -> Unit, retryNote: String = RETRY_NOTE) {
    // back online: nothing left to say
    LaunchedEffect(active) { if (!active) onDismiss() }
    val p = MaterialTheme.palette
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = sheetState, containerColor = p.panel, contentColor = p.text) {
        Column(Modifier.fillMaxWidth().padding(start = Gutter, end = Gutter, bottom = 24.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(painterResource(R.drawable.kit_ic_cloud_off), contentDescription = null, tint = KitStatusColors.Warn, modifier = Modifier.size(22.dp))
                Spacer(Modifier.width(10.dp))
                Text("You're offline", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
            }
            Text(message, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(top = 12.dp))
            Text("Why: $reason", style = MaterialTheme.typography.bodySmall, color = p.muted, modifier = Modifier.padding(top = 8.dp))
            if (status.isNotEmpty()) {
                Text(status, style = MaterialTheme.typography.bodySmall, color = p.muted, modifier = Modifier.padding(top = 4.dp))
            }
            Text(retryNote, style = MaterialTheme.typography.bodySmall, color = p.faint, modifier = Modifier.padding(top = 4.dp))
            Row(Modifier.padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                KitButton(if (trying) "Trying…" else "Retry", iconRes = R.drawable.kit_ic_refresh, primary = true, enabled = !trying, onClick = onRetry)
                KitButton("Close", onClick = onDismiss)
            }
        }
    }
}

/**
 * Asks the server's health check ([KitRoutes.health]) once, as offline mode's check does: null when it answered (any status but
 * 502–504: a proxy saying the server behind it is down counts as no answer); otherwise why not, in words
 * ([OfflineReason]). Blocking: run it on Dispatchers.IO.
 *
 * @param http the app's client (its interceptors and headers); the check uses a copy without a cache.
 * @param server the server's base URL ("https://app.example.com").
 * @param phoneOnline whether the phone has a network at all (the reason when nothing answers).
 * @param waitSeconds how long the server gets to answer.
 * @param path the health check's path ([KitRoutes.health] by default).
 */
fun probeHealth(http: OkHttpClient, server: String, phoneOnline: Boolean, waitSeconds: Long, path: String = KitApp.routes.health): String? {
    val client =
        http
            .newBuilder()
            .connectTimeout(waitSeconds, TimeUnit.SECONDS)
            .callTimeout(waitSeconds, TimeUnit.SECONDS)
            .cache(null)
            .build()
    return try {
        client.newCall(Request.Builder().url(server.trimEnd('/') + path).build()).execute().use {
            if (it.code in 502..504) OfflineReason.of(phoneOnline = true, code = it.code, error = null, seconds = waitSeconds) else null
        }
    } catch (e: Exception) {
        // still asked without a network (a VPN or a LAN can answer anyway); without one, that is the reason
        OfflineReason.of(phoneOnline = phoneOnline, code = null, error = e, seconds = waitSeconds)
    }
}
