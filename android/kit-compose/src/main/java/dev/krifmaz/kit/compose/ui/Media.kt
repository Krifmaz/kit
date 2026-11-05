package dev.krifmaz.kit.compose.ui

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.snapping.rememberSnapFlingBehavior
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.scale
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import coil3.compose.AsyncImage
import dev.krifmaz.kit.compose.icons.KitIcons
import dev.krifmaz.kit.compose.theme.KitMotion
import dev.krifmaz.kit.compose.theme.KitSizes
import dev.krifmaz.kit.compose.theme.palette

/** A cover's shape (kit-leptos `Shape`). */
enum class KitShape(val ratio: Float) {
    /** 2 : 3, a poster or a book. */
    Poster(2f / 3f),

    /** 1 : 1, an album or a photo. */
    Square(1f),

    /** 16 : 9, a video or a page. */
    Wide(16f / 9f),
}

/** The placeholder cover of [seed] and [title] (kit-leptos `placeholder_art`): two colours, soft circles, the initials. */
@Composable
fun PlaceholderArt(seed: Int, title: String, modifier: Modifier = Modifier) {
    val (a, b) = remember(seed) { DesignRules.hues(seed) }
    val r = 60 + Math.floorMod(seed, 50)
    val cx = 40 + Math.floorMod(seed * 37, 220)
    Box(
        modifier
            .background(Brush.linearGradient(listOf(Color.hsl(a.toFloat(), 0.45f, 0.42f), Color.hsl(b.toFloat(), 0.5f, 0.22f))))
            .drawBehind {
                // the web's 300 × 450 box, scaled to this one
                val sx = size.width / 300f
                val sy = size.height / 450f
                drawCircle(Color.hsl(b.toFloat(), 0.6f, 0.7f, 0.22f), radius = r * sx, center = Offset(cx * sx, 140 * sy))
                drawCircle(Color.hsl(a.toFloat(), 0.7f, 0.8f, 0.14f), radius = (r + 30) * sx, center = Offset((300 - cx) * sx, 330 * sy))
            },
        contentAlignment = Alignment.Center,
    ) {
        Text(DesignRules.initials(title), color = Color.White.copy(alpha = 0.85f), fontWeight = FontWeight.Bold, fontSize = 28.sp)
    }
}

/** The cover: [art] when given, else the placeholder. */
@Composable
private fun Cover(art: String?, seed: Int, title: String, modifier: Modifier = Modifier) {
    if (art.isNullOrEmpty()) {
        PlaceholderArt(seed, title, modifier)
    } else {
        AsyncImage(model = art, contentDescription = null, contentScale = ContentScale.Crop, modifier = modifier)
    }
}

/**
 * One cover with its title (kit-leptos `MediaCard`): presses in under a finger, a badge, a progress bar.
 *
 * @param progress how far along (0–1), a bar along the cover's foot; none when null.
 */
@Composable
fun MediaCard(
    title: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    art: String? = null,
    seed: Int = 0,
    subtitle: String? = null,
    badge: String? = null,
    progress: Double? = null,
    shape: KitShape = KitShape.Poster,
) {
    val p = MaterialTheme.palette
    val press = remember { MutableInteractionSource() }
    val pressed by press.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.97f else 1f, tween(KitMotion.FAST), label = "press")
    Column(
        modifier
            .clickable(interactionSource = press, indication = null, role = Role.Button, onClick = onClick)
            .semantics(mergeDescendants = true) {},
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Box(
            Modifier
                .fillMaxWidth()
                .aspectRatio(shape.ratio)
                .scale(scale)
                .clip(RoundedCornerShape(12.dp))
                .background(p.panel2),
        ) {
            Cover(art, seed, title, Modifier.fillMaxSize())
            if (badge != null) {
                Text(
                    badge,
                    color = p.text,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(8.dp).clip(RoundedCornerShape(99.dp)).background(p.bg.copy(alpha = 0.7f)).padding(horizontal = 8.dp, vertical = 2.dp),
                )
            }
            if (progress != null) {
                Box(Modifier.align(Alignment.BottomCenter).padding(8.dp).fillMaxWidth().height(4.dp).clip(RoundedCornerShape(99.dp)).background(Color.Black.copy(alpha = 0.45f))) {
                    Box(Modifier.fillMaxWidth(DesignRules.unit(progress).toFloat()).height(4.dp).background(p.accent))
                }
            }
        }
        Column(Modifier.padding(horizontal = 2.dp)) {
            Text(title, color = p.text, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (subtitle != null) Text(subtitle, color = p.muted, fontSize = 12.sp, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/**
 * A grid of cards inside a scrolling screen (kit-leptos `MediaGrid`): as many columns as fit, three on a phone. For
 * a long list on its own screen use a `LazyVerticalGrid` with [columnWidth].
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun <T> MediaGrid(items: List<T>, modifier: Modifier = Modifier, shape: KitShape = KitShape.Poster, item: @Composable (T) -> Unit) {
    BoxWithConstraints(modifier.fillMaxWidth()) {
        val min = columnWidth(shape, maxWidth)
        val gap = 12.dp
        val cols = ((maxWidth + gap) / (min + gap)).toInt().coerceAtLeast(1)
        val w = (maxWidth - gap * (cols - 1)) / cols
        FlowRow(horizontalArrangement = Arrangement.spacedBy(gap), verticalArrangement = Arrangement.spacedBy(16.dp), maxItemsInEachRow = cols) {
            items.forEach { Box(Modifier.width(w)) { item(it) } }
        }
    }
}

/** The smallest column of a grid of [shape] covers on a screen [width] wide (the web's grid). */
fun columnWidth(shape: KitShape, width: Dp): Dp {
    val phone = width < DesignRules.PHONE_BELOW.dp
    return when (shape) {
        KitShape.Poster -> if (phone) 104.dp else 150.dp
        KitShape.Square -> if (phone) 140.dp else 170.dp
        KitShape.Wide -> if (phone) 240.dp else 280.dp
    }
}

/**
 * A titled row of cards that scrolls sideways and snaps each card into place (kit-leptos `Carousel`).
 *
 * @param onSeeAll the "See all" button; none when null.
 */
@Composable
fun <T> Carousel(
    title: String,
    items: List<T>,
    key: (T) -> Any,
    modifier: Modifier = Modifier,
    shape: KitShape = KitShape.Poster,
    onSeeAll: (() -> Unit)? = null,
    seeAllLabel: String = "See all",
    gutter: Dp = 16.dp,
    item: @Composable (T) -> Unit,
) {
    val p = MaterialTheme.palette
    val state = rememberLazyListState()
    val width =
        when (shape) {
            KitShape.Poster -> 132.dp
            KitShape.Square -> 148.dp
            KitShape.Wide -> 260.dp
        }
    Column(modifier.fillMaxWidth()) {
        SectionTitle(title, Modifier.padding(horizontal = gutter)) {
            if (onSeeAll != null) TextButton(onClick = onSeeAll) { Text(seeAllLabel, color = p.muted) }
        }
        LazyRow(
            state = state,
            flingBehavior = rememberSnapFlingBehavior(lazyListState = state),
            contentPadding = PaddingValues(horizontal = gutter, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.semantics { contentDescription = title },
        ) {
            items(items, key = key, contentType = { "card" }) { Box(Modifier.width(width)) { item(it) } }
        }
    }
}

/** A section's heading with room for an action at its end, 44 dp tall. */
@Composable
fun SectionTitle(title: String, modifier: Modifier = Modifier, action: @Composable () -> Unit = {}) {
    Row(modifier.fillMaxWidth().heightIn(min = KitSizes.Touch), verticalAlignment = Alignment.CenterVertically) {
        Text(
            title,
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.palette.text,
            modifier = Modifier.weight(1f),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        action()
    }
}

/** A banner (kit-leptos `Hero`): the art blurred behind (Android 12 and later; a scrim before), the cover, the title, a line, the [actions]. */
@Composable
fun Hero(title: String, modifier: Modifier = Modifier, eyebrow: String? = null, text: String? = null, art: String? = null, seed: Int = 0, actions: @Composable () -> Unit = {}) {
    val p = MaterialTheme.palette
    Box(modifier.fillMaxWidth().clip(RoundedCornerShape(bottomStart = 20.dp, bottomEnd = 20.dp)).background(p.panel)) {
        Cover(art, seed, title, Modifier.matchParentSize().blur(42.dp))
        Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(p.bg.copy(alpha = 0.35f), p.bg.copy(alpha = 0.92f)))))
        Column(Modifier.padding(start = 16.dp, end = 16.dp, top = 24.dp, bottom = 20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Cover(art, seed, title, Modifier.width(88.dp).aspectRatio(2f / 3f).clip(RoundedCornerShape(12.dp)))
            if (eyebrow != null) Text(eyebrow.uppercase(), color = p.accent2, fontSize = 12.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
            Text(title, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.ExtraBold, color = p.text)
            if (text != null) Text(text, color = p.muted, maxLines = 3, overflow = TextOverflow.Ellipsis)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) { actions() }
        }
    }
}

/** The top of an item's screen (kit-leptos `DetailHeader`): the blurred cover behind, the cover, the title, [chips] and the [actions]. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun DetailHeader(
    title: String,
    modifier: Modifier = Modifier,
    subtitle: String? = null,
    art: String? = null,
    seed: Int = 0,
    shape: KitShape = KitShape.Poster,
    chips: List<String> = emptyList(),
    actions: @Composable () -> Unit = {},
) {
    val p = MaterialTheme.palette
    Box(modifier.fillMaxWidth()) {
        Cover(art, seed, title, Modifier.matchParentSize().blur(42.dp))
        Box(Modifier.matchParentSize().background(Brush.verticalGradient(listOf(p.bg.copy(alpha = 0.2f), p.bg))))
        Column(
            Modifier.fillMaxWidth().padding(start = 16.dp, end = 16.dp, top = 40.dp, bottom = 16.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Cover(art, seed, title, Modifier.width(if (shape == KitShape.Wide) 280.dp else 148.dp).aspectRatio(shape.ratio).clip(RoundedCornerShape(14.dp)))
            Text(title, style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.ExtraBold, color = p.text, textAlign = TextAlign.Center)
            if (subtitle != null) Text(subtitle, color = p.muted, textAlign = TextAlign.Center)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                chips.forEach { KitChip(it) }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) { actions() }
        }
    }
}

/** A small pill of metadata (kit-leptos `Chip`). */
@Composable
fun KitChip(label: String, modifier: Modifier = Modifier, icon: String? = null) {
    val p = MaterialTheme.palette
    Row(
        modifier.clip(
            RoundedCornerShape(99.dp),
        ).background(p.panel2.copy(alpha = 0.8f)).border(1.dp, p.border, RoundedCornerShape(99.dp)).padding(horizontal = 12.dp, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        if (icon != null) Icon(kitIcon(icon), contentDescription = null, tint = p.muted, modifier = Modifier.size(14.dp))
        Text(label, color = p.muted, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, maxLines = 1)
    }
}

/** A row of filter chips, one of them on (kit-leptos `FilterChips`): scrolls sideways, each chip 44 dp tall. */
@Composable
fun FilterChips(options: List<String>, selected: String, onSelect: (String) -> Unit, modifier: Modifier = Modifier, gutter: Dp = 16.dp) {
    val p = MaterialTheme.palette
    LazyRow(modifier, contentPadding = PaddingValues(horizontal = gutter), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        items(options, key = { it }) { o ->
            val on = o == selected
            Box(
                Modifier
                    .heightIn(min = KitSizes.Touch)
                    .clip(RoundedCornerShape(99.dp))
                    .background(if (on) p.gradient else Brush.linearGradient(listOf(p.panel2, p.panel2)))
                    .border(1.dp, if (on) Color.Transparent else p.border, RoundedCornerShape(99.dp))
                    .clickable(role = Role.Tab) { onSelect(o) }
                    .semantics { this.selected = on }
                    .padding(horizontal = 16.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(o, color = if (on) p.onAccent else p.muted, fontWeight = FontWeight.SemiBold, fontSize = 13.sp)
            }
        }
    }
}

/** A column of a screen's sections with the kit's spacing between them. */
@Composable
fun KitSections(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(32.dp), content = content)
}
