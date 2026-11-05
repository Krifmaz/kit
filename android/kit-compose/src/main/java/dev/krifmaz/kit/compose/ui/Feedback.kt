package dev.krifmaz.kit.compose.ui

import android.provider.Settings
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette

/** The picture of an empty state (kit-leptos `Illustration`), with the kit icon at its centre. */
enum class KitIllustration(val icon: String) {
    /** Nothing here yet. */
    Empty("inbox"),

    /** A search found nothing. */
    Search("search"),

    /** No connection. */
    Offline("wifi-off"),

    /** Something went wrong. */
    Error("triangle-alert"),

    /** All done. */
    Done("circle-check"),
}

/** Whether the system asks for no animations ("Remove animations", animator duration scale 0). */
@Composable
fun rememberReducedMotion(): Boolean {
    val context = LocalContext.current
    return remember(context) { Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }
}

/**
 * A screen or list with nothing to show (kit-leptos `EmptyState`): the illustration, what is (not) there, what to
 * do and the [actions].
 */
@Composable
fun KitEmptyState(title: String, modifier: Modifier = Modifier, text: String? = null, art: KitIllustration = KitIllustration.Empty, actions: @Composable () -> Unit = {}) {
    val p = MaterialTheme.palette
    val tone =
        when (art) {
            KitIllustration.Error -> KitStatusColors.Err
            KitIllustration.Offline -> KitStatusColors.Warn
            KitIllustration.Done -> KitStatusColors.Ok
            else -> p.accent
        }
    Column(
        modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .background(p.panel.copy(alpha = 0.6f))
            .border(1.dp, p.border, RoundedCornerShape(14.dp))
            .padding(horizontal = 16.dp, vertical = 40.dp)
            .semantics { liveRegion = LiveRegionMode.Polite },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(Modifier.width(160.dp).height(120.dp), contentAlignment = Alignment.Center) {
            Canvas(Modifier.width(160.dp).height(120.dp)) {
                val s = size.width / 160f
                val c = Offset(80 * s, 60 * s)
                drawCircle(tone.copy(alpha = 0.08f), 56 * s, c)
                drawCircle(tone.copy(alpha = 0.10f), 40 * s, c)
                for ((x, deg) in listOf(36f to -10f, 68f to 8f)) {
                    rotate(deg, Offset((x + 28) * s, 64 * s)) {
                        drawRoundRect(p.panel2, Offset(x * s, 32 * s), Size(56 * s, 64 * s), CornerRadius(10 * s))
                        drawRoundRect(p.border, Offset(x * s, 32 * s), Size(56 * s, 64 * s), CornerRadius(10 * s), style = Stroke(1.5f * s))
                    }
                }
                drawCircle(tone.copy(alpha = 0.55f), 4 * s, Offset(132 * s, 24 * s))
                drawCircle(tone.copy(alpha = 0.55f), 3 * s, Offset(24 * s, 92 * s))
                drawCircle(tone.copy(alpha = 0.55f), 2.5f * s, Offset(140 * s, 96 * s))
            }
            Icon(kitIcon(art.icon), contentDescription = null, tint = tone, modifier = Modifier.size(48.dp))
        }
        Text(title, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold, color = p.text, textAlign = TextAlign.Center)
        if (text != null) Text(text, color = p.muted, textAlign = TextAlign.Center)
        Row(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) { actions() }
    }
}

/** A skeleton's shimmer; still when the system asks for reduced motion. */
fun Modifier.shimmer(shape: RoundedCornerShape = RoundedCornerShape(8.dp)): Modifier = composed {
    val p = MaterialTheme.palette
    val still = rememberReducedMotion()
    val base = p.panel2
    val light = Color(
        red = (base.red + p.border.red) / 2,
        green = (base.green + p.border.green) / 2,
        blue = (base.blue + p.border.blue) / 2,
    )
    val x =
        if (still) {
            0f
        } else {
            val t = rememberInfiniteTransition(label = "shimmer")
            val v by t.animateFloat(0f, 1f, infiniteRepeatable(tween(1400, easing = LinearEasing), RepeatMode.Restart), label = "x")
            v
        }
    this.clip(shape).background(
        if (still) {
            Brush.linearGradient(listOf(base, base))
        } else {
            Brush.linearGradient(listOf(base, light, base), start = Offset(-600f + 1200f * x, 0f), end = Offset(1200f * x, 0f))
        },
    )
}

/** Lines of text on their way (kit-leptos `SkeletonLines`): the last one shorter. */
@Composable
fun SkeletonLines(lines: Int = 3, modifier: Modifier = Modifier) {
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        repeat(lines) { i -> Box(Modifier.fillMaxWidth(if (i == lines - 1 && lines > 1) 0.6f else 1f).height(12.dp).shimmer()) }
    }
}

/** One card on its way (kit-leptos skeleton card). */
@Composable
fun SkeletonCard(modifier: Modifier = Modifier, shape: KitShape = KitShape.Poster) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Box(Modifier.fillMaxWidth().aspectRatio(shape.ratio).shimmer(RoundedCornerShape(12.dp)))
        Box(Modifier.fillMaxWidth().height(12.dp).shimmer())
        Box(Modifier.fillMaxWidth(0.6f).height(12.dp).shimmer())
    }
}

/** A [MediaGrid] on its way: [count] cards (kit-leptos `SkeletonGrid`). */
@Composable
fun SkeletonGrid(count: Int = 12, modifier: Modifier = Modifier, shape: KitShape = KitShape.Poster) {
    MediaGrid((0 until count).toList(), modifier, shape) { SkeletonCard(shape = shape) }
}

/** A [Carousel] on its way: a title bar and [count] cards (kit-leptos `SkeletonRail`). */
@Composable
fun SkeletonRail(count: Int = 8, modifier: Modifier = Modifier, gutter: Dp = 16.dp) {
    Column(modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Box(Modifier.padding(horizontal = gutter).width(160.dp).height(20.dp).shimmer())
        Row(Modifier.padding(start = gutter), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            repeat(count.coerceAtMost(4)) { SkeletonCard(Modifier.width(132.dp)) }
        }
    }
}
