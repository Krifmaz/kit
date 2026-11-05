package dev.krifmaz.kit.compose.ui

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.krifmaz.kit.compose.icons.KitIcons
import dev.krifmaz.kit.compose.theme.KitMotion
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette

/** One number with its label, an icon and how it changed (kit-leptos `StatCard`); [delta] is a fraction (0.12 is +12%). */
@Composable
fun StatCard(label: String, value: String, modifier: Modifier = Modifier, icon: String? = null, delta: Double? = null, hint: String? = null) {
    val p = MaterialTheme.palette
    Column(
        modifier
            .clip(RoundedCornerShape(14.dp))
            .background(Brush.linearGradient(listOf(p.panel2, p.panel)))
            .border(1.dp, p.border, RoundedCornerShape(14.dp))
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(label, color = p.muted, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (icon != null) {
                Box(Modifier.size(32.dp).clip(RoundedCornerShape(10.dp)).background(p.accent.copy(alpha = 0.14f)), contentAlignment = Alignment.Center) {
                    Icon(kitIcon(icon), contentDescription = null, tint = p.accent, modifier = Modifier.size(17.dp))
                }
            }
        }
        Text(value, color = p.text, fontSize = 28.sp, fontWeight = FontWeight.ExtraBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            if (delta != null) {
                val (text, dir) = DesignRules.deltaText(delta)
                val color =
                    when (dir) {
                        "up" -> KitStatusColors.Ok
                        "down" -> KitStatusColors.Err
                        else -> p.muted
                    }
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                    if (dir !=
                        "flat"
                    ) {
                        Icon(if (dir == "up") KitIcons.TrendingUp else KitIcons.TrendingDown, contentDescription = null, tint = color, modifier = Modifier.size(14.dp))
                    }
                    Text(text, color = color, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                }
            }
            if (hint != null) Text(hint, color = p.faint, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** A ring that fills with [value] (0–1), the percentage inside and an optional [label] under it (kit-leptos `ProgressRing`). */
@Composable
fun ProgressRing(value: Double, modifier: Modifier = Modifier, size: Dp = 72.dp, stroke: Dp = 7.dp, label: String? = null, color: Color = Color.Unspecified) {
    val p = MaterialTheme.palette
    val v = DesignRules.unit(value).toFloat()
    val shown by animateFloatAsState(v, tween(KitMotion.POP), label = "ring")
    val tint = if (color == Color.Unspecified) p.accent else color
    val pct = Math.round(v * 100)
    Column(
        modifier.semantics(mergeDescendants = true) {
            progressBarRangeInfo = ProgressBarRangeInfo(v, 0f..1f)
            if (label != null) contentDescription = "$label $pct%"
        },
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(Modifier.size(size), contentAlignment = Alignment.Center) {
            Canvas(Modifier.size(size)) {
                val w = stroke.toPx()
                val inset = w / 2
                val arc = androidx.compose.ui.geometry.Size(this.size.width - w, this.size.height - w)
                val tl = androidx.compose.ui.geometry.Offset(inset, inset)
                drawArc(p.panel2, 0f, 360f, false, topLeft = tl, size = arc, style = Stroke(w))
                drawArc(tint, -90f, 360f * shown, false, topLeft = tl, size = arc, style = Stroke(w, cap = StrokeCap.Round))
            }
            Text("$pct%", color = p.text, fontSize = 13.sp, fontWeight = FontWeight.Bold)
        }
        if (label != null) Text(label, color = p.muted, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
    }
}
