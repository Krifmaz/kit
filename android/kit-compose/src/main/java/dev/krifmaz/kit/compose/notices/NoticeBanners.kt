package dev.krifmaz.kit.compose.notices

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.KitApp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.core.KitApi
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.widgets.SmallButton
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.JsonObject

/**
 * The server's notices as the screens see them: the last answer, and which notice's Check again runs. [load] asks
 * for them (GET [KitRoutes][dev.krifmaz.kit.compose.KitRoutes]`.notices`) and [check] runs Check again (POST
 * `<notices>/check`); both block, so they run on Dispatchers.IO. Make it with [rememberNotices].
 *
 * @param load asks the server for its notices.
 * @param check asks the server to check again; answers the notices after the check.
 */
@Stable
class NoticesState(private val load: () -> List<Notice>, private val check: () -> List<Notice>) {
    /** The notices of the last answer (empty until one came, or when the server has none). */
    var notices by mutableStateOf<List<Notice>>(emptyList())
        private set

    /** The notices (by id) whose Check again runs. */
    val checking = mutableStateMapOf<String, Boolean>()

    /** Asks the server now; a failed ask keeps the last answer. */
    suspend fun refresh() {
        try {
            notices = withContext(Dispatchers.IO) { load() }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            // the server can't be reached or has no notices: offline mode says so, the banners stay as they were
        }
    }

    /** Check again for [n]: the server checks, the banners follow its answer, and a toast says how it went. */
    suspend fun checkAgain(n: Notice, words: CheckWords) {
        checking[n.id] = true
        try {
            val after = withContext(Dispatchers.IO) { check() }
            notices = after
            val now = after.firstOrNull { it.id == n.id } ?: Notice(ok = true, id = n.id)
            val (msg, kind) = NoticeRules.checkToast(now, words)
            Toasts.show(msg, kind)
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            Toasts.show(e.message ?: "Something went wrong", "err")
        } finally {
            checking.remove(n.id)
        }
    }
}

/**
 * A [NoticesState] for the app's server API: GET [KitApp.routes]`.notices` and POST `<notices>/check`, polled while
 * the screen shows (at start, then every [NoticeRules.TICK_MS] while one fails, else every minute).
 *
 * @param api the app's server API.
 * @param admin only admins see banners; nothing is asked for anyone else.
 */
@Composable
fun rememberNotices(api: KitApi, admin: Boolean): NoticesState {
    val path = KitApp.routes.notices
    val state = remember(api, path) {
        NoticesState(
            load = { NoticeRules.noticesOf(api.get(path) as? JsonObject) },
            check = { NoticeRules.noticesOf(api.post("$path/check", JsonObject(emptyMap())) as? JsonObject) },
        )
    }
    LaunchedEffect(state, admin) {
        if (!admin) return@LaunchedEffect
        val poll = NoticePoll()
        state.refresh()
        while (true) {
            delay(NoticeRules.TICK_MS)
            if (poll.tick(state.notices.any { it.failing })) state.refresh()
        }
    }
    return state
}

/**
 * The banners of the failing notices, one under the other: an alert icon, the sentence, and Check again (Checking…
 * while it runs). Nothing shows for someone who is not an admin or when all is well.
 *
 * @param state the notices ([rememberNotices]).
 * @param admin whether the account is an admin.
 * @param words Check again's toast sentences per notice id; [CheckWords]'s defaults for the others.
 * @param modifier the banners' column.
 */
@Composable
fun NoticeBanners(state: NoticesState, admin: Boolean, words: Map<String, CheckWords> = emptyMap(), modifier: Modifier = Modifier) {
    val shown = NoticeRules.banners(state.notices, admin)
    if (shown.isEmpty()) return
    val scope = rememberCoroutineScope()
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        for (n in shown) {
            NoticeBanner(n, busy = state.checking[n.id] == true) {
                scope.launch { state.checkAgain(n, words[n.id] ?: CheckWords()) }
            }
        }
    }
}

/**
 * One banner: [n]'s sentence on a tinted panel with its severity's colour, and Check again.
 *
 * @param n the notice.
 * @param busy its check runs (the button says Checking… and waits).
 * @param onCheck Check again.
 */
@Composable
fun NoticeBanner(n: Notice, busy: Boolean, onCheck: () -> Unit) {
    val p = MaterialTheme.palette
    val tone: Color = when (n.severity) {
        Severity.Info -> p.accent
        Severity.Warn -> KitStatusColors.Warn
        Severity.Err -> KitStatusColors.Err
    }
    val shape = RoundedCornerShape(12.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .semantics { liveRegion = LiveRegionMode.Polite }
            .background(tone.copy(alpha = 0.10f), shape)
            .border(1.dp, tone.copy(alpha = 0.35f), shape)
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(painterResource(R.drawable.kit_ic_warning), contentDescription = null, tint = tone, modifier = Modifier.size(20.dp))
        Spacer(Modifier.width(10.dp))
        Text(n.banner ?: "", style = MaterialTheme.typography.bodyMedium, color = p.text, modifier = Modifier.weight(1f))
        Spacer(Modifier.width(8.dp))
        SmallButton(if (busy) NoticeRules.CHECKING else NoticeRules.CHECK_AGAIN, iconRes = R.drawable.kit_ic_refresh, enabled = !busy, onClick = onCheck)
    }
}
