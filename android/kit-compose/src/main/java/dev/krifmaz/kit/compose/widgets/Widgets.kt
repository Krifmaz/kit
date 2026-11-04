package dev.krifmaz.kit.compose.widgets

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette

// The small building blocks of an app's screens (the web's .btn, .badge, .card, .input, .error-box, .empty …),
// shared by every kit screen and by the apps.

/** The web's phone gutter (`.main { padding: 16px }`). */
val Gutter: Dp = 16.dp

/** The web's `.error-box` with a retry, centred in the empty page. */
@Composable
fun ErrorBox(message: String, onRetry: () -> Unit) {
    Column(
        Modifier.fillMaxSize().padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(
            Modifier
                .clip(RoundedCornerShape(10.dp))
                .background(KitStatusColors.Err.copy(alpha = 0.12f))
                .border(1.dp, KitStatusColors.Err.copy(alpha = 0.35f), RoundedCornerShape(10.dp))
                .padding(horizontal = 12.dp, vertical = 10.dp),
        ) {
            Text(message, style = MaterialTheme.typography.bodySmall, color = if (MaterialTheme.palette.dark) Color(0xFFFCA5A5) else Color(0xFFB91C1C))
        }
        TextButton(onClick = onRetry) { Text("Try again", color = MaterialTheme.palette.text) }
    }
}

/** The web's `.empty`: an icon, a title and an optional line below it. */
@Composable
fun EmptyState(iconRes: Int, title: String, text: String = "", modifier: Modifier = Modifier) {
    Column(
        modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 50.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(painterResource(iconRes), contentDescription = null, tint = MaterialTheme.palette.faint, modifier = Modifier.size(46.dp))
        Spacer(Modifier.height(10.dp))
        Text(title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.palette.muted, textAlign = TextAlign.Center)
        if (text.isNotEmpty()) {
            Text(
                text,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.palette.muted,
                textAlign = TextAlign.Center,
                modifier = Modifier.padding(top = 6.dp),
            )
        }
    }
}

/** The web's `.section-head`: an h2 (with an optional line under it) and an optional small button on the right. */
@Composable
fun SectionHead(title: String, subtitle: String? = null, modifier: Modifier = Modifier, action: String? = null, onAction: (() -> Unit)? = null) {
    Row(
        modifier.fillMaxWidth().padding(start = Gutter, end = Gutter, top = 24.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleLarge)
            if (!subtitle.isNullOrEmpty()) {
                Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.palette.muted)
            }
        }
        if (action != null && onAction != null) SmallButton(action, onClick = onAction)
    }
}

/** The web's `.badge` pill. `kind`: "" (muted), "ok", "warn", "err", "info", "accent". */
@Composable
fun Badge(text: String, kind: String = "", modifier: Modifier = Modifier, onClick: (() -> Unit)? = null) {
    val p = MaterialTheme.palette
    val tone: Color? = when (kind) {
        "ok" -> KitStatusColors.Ok
        "warn" -> KitStatusColors.Warn
        "err" -> KitStatusColors.Err
        "info" -> KitStatusColors.Info
        else -> null
    }
    val fg: Color = if (kind == "accent") p.text else tone ?: p.muted
    val edge: Color = if (kind == "accent") p.accent.copy(alpha = 0.4f) else tone?.copy(alpha = 0.35f) ?: p.border
    val bg: Color = if (kind == "accent") p.accent.copy(alpha = 0.14f) else tone?.copy(alpha = 0.10f) ?: p.panel2
    val shape = RoundedCornerShape(99.dp)
    var m = modifier.clip(shape).background(bg).border(1.dp, edge, shape)
    if (onClick != null) m = m.clickable(onClick = onClick)
    Box(m.padding(horizontal = 9.dp, vertical = 2.dp)) {
        Text(text, style = MaterialTheme.typography.labelSmall, color = fg, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/** The web's `.btn.sm`: a small bordered button on the panel colour (`danger`: the web's `.btn.sm.danger`, in red). */
@Composable
fun SmallButton(
    label: String,
    modifier: Modifier = Modifier,
    iconRes: Int? = null,
    primary: Boolean = false,
    enabled: Boolean = true,
    danger: Boolean = false,
    onClick: () -> Unit,
) {
    val p = MaterialTheme.palette
    val shape = RoundedCornerShape(8.dp)
    val bg = when {
        primary -> Modifier.background(p.gradient)
        danger -> Modifier.background(KitStatusColors.Err.copy(alpha = 0.10f)).border(1.dp, KitStatusColors.Err.copy(alpha = 0.45f), shape)
        else -> Modifier.background(p.panel2).border(1.dp, p.border, shape)
    }
    Row(
        modifier
            .clip(shape)
            .then(bg)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 11.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        val fg = if (primary) {
            p.onAccent
        } else if (danger) {
            KitStatusColors.Err
        } else {
            p.text
        }
        if (iconRes != null) Icon(painterResource(iconRes), contentDescription = null, tint = fg, modifier = Modifier.size(16.dp))
        Text(label, style = MaterialTheme.typography.labelMedium, color = fg)
    }
}

/** The web's `.btn` / `.btn.primary`: the page's main actions (Play, Shuffle, Import…). */
@Composable
fun KitButton(label: String, modifier: Modifier = Modifier, iconRes: Int? = null, primary: Boolean = false, enabled: Boolean = true, onClick: () -> Unit) {
    val p = MaterialTheme.palette
    val shape = RoundedCornerShape(10.dp)
    val bg = if (primary) Modifier.background(p.gradient) else Modifier.background(p.panel2).border(1.dp, p.border, shape)
    val alpha = if (enabled) 1f else 0.5f
    Row(
        modifier
            .clip(shape)
            .then(bg)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 9.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        val fg = (if (primary) p.onAccent else p.text).copy(alpha = alpha)
        if (iconRes != null) Icon(painterResource(iconRes), contentDescription = null, tint = fg, modifier = Modifier.size(16.dp))
        Text(label, style = MaterialTheme.typography.labelLarge, color = fg)
    }
}

/**
 * A row of choices that look like the web's `.tabs` (All / Mine / Shared); scrolls when they don't fit.
 * [end]: the space after it (less when something sits beside it, like the library's Sort chip). [busy]: the choices
 * that show a small spinner after their label (the picker's tabs while their source searches).
 */
@Composable
fun ChoiceRow(options: List<String>, selected: Int, onSelect: (Int) -> Unit, modifier: Modifier = Modifier, end: Dp = Gutter, busy: Set<Int> = emptySet()) {
    val p = MaterialTheme.palette
    val outer = RoundedCornerShape(12.dp)
    Row(modifier.padding(start = Gutter, end = end, top = 8.dp, bottom = 8.dp).horizontalScroll(rememberScrollState())) {
        Row(
            Modifier.clip(outer).background(p.panel).border(1.dp, p.border, outer).padding(4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            options.forEachIndexed { i, label ->
                val on = i == selected
                Box(
                    Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (on) p.panel2 else Color.Transparent)
                        .clickable { onSelect(i) }
                        .padding(horizontal = 14.dp, vertical = 7.dp),
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(label, style = MaterialTheme.typography.labelLarge, color = if (on) p.text else p.muted, maxLines = 1)
                        if (i in busy) CircularProgressIndicator(color = p.accent, trackColor = p.border, strokeWidth = 1.5.dp, modifier = Modifier.size(12.dp))
                    }
                }
            }
        }
    }
}

/** A thin line between list rows (the web's `.list-item` border). */
@Composable
fun RowDivider(modifier: Modifier = Modifier, startIndent: Dp = Gutter) {
    Box(modifier.fillMaxWidth().padding(start = startIndent).height(1.dp).background(MaterialTheme.palette.border))
}

/**
 * One line of a "⋯" menu (the web's `popMenu`). A null `onClick` draws a separator; `badge` is a count after the label.
 * [info]: a muted line of text, not an action ("Added by alex on Oct 13, 2026").
 */
class MenuEntry(val label: String, val iconRes: Int? = null, val danger: Boolean = false, val badge: String? = null, val info: Boolean = false, val onClick: (() -> Unit)?) {
    /** A line that runs something when tapped (not a separator, not an info line). */
    val isAction: Boolean get() = !info && onClick != null
}

/** A separator for [PopMenu]. */
val MenuSeparator: MenuEntry = MenuEntry("-", onClick = null)

/** The web's `.menu-pop`: a panel of actions anchored to its "⋯" button. */
@Composable
fun PopMenu(expanded: Boolean, onDismiss: () -> Unit, entries: List<MenuEntry?>) {
    val p = MaterialTheme.palette
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismiss,
        shape = RoundedCornerShape(12.dp),
        containerColor = p.panel,
    ) {
        for (e in entries) {
            if (e == null) continue
            if (e.info) {
                // text, not an action: nothing to tap
                Text(
                    e.label,
                    style = MaterialTheme.typography.bodySmall,
                    color = p.muted,
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
                )
                continue
            }
            if (e.onClick == null) {
                Box(Modifier.fillMaxWidth().padding(vertical = 4.dp).height(1.dp).background(p.border))
                continue
            }
            val fg = if (e.danger) KitStatusColors.Err else p.text
            DropdownMenuItem(
                text = {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(e.label, color = fg, style = MaterialTheme.typography.bodyLarge)
                        if (!e.badge.isNullOrEmpty()) Badge(e.badge, kind = "warn")
                    }
                },
                onClick = {
                    onDismiss()
                    e.onClick.invoke()
                },
                leadingIcon = if (e.iconRes != null) {
                    { Icon(painterResource(e.iconRes), contentDescription = null, tint = fg.copy(alpha = 0.85f), modifier = Modifier.size(18.dp)) }
                } else {
                    null
                },
            )
        }
    }
}

/** A "⋯" button with its menu. */
@Composable
fun MoreButton(entries: List<MenuEntry?>, modifier: Modifier = Modifier, tint: Color = Color.Unspecified, iconSize: Dp = 24.dp, onOpen: () -> Unit = {}) {
    var open by remember { mutableStateOf(false) }
    Box(modifier) {
        IconButton(onClick = {
            open = true
            onOpen()
        }) {
            Icon(
                painterResource(R.drawable.kit_ic_more_vert),
                contentDescription = "More",
                tint = if (tint != Color.Unspecified) tint else MaterialTheme.palette.muted,
                modifier = Modifier.size(iconSize),
            )
        }
        PopMenu(open, { open = false }, entries)
    }
}

/** The web's `.input`: an outlined field in the panel colours. */
@Composable
fun KitField(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    placeholder: String = "",
    label: String? = null,
    leadingIcon: Int? = null,
    enabled: Boolean = true,
    singleLine: Boolean = true,
    minLines: Int = 1,
    maxLines: Int = Int.MAX_VALUE,
    password: Boolean = false,
    keyboardType: KeyboardType = KeyboardType.Text,
    imeAction: ImeAction = ImeAction.Done,
    onAction: (() -> Unit)? = null,
) {
    val p = MaterialTheme.palette
    val focus = LocalFocusManager.current
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        placeholder = if (placeholder.isNotEmpty()) ({ Text(placeholder, color = p.faint, maxLines = 1, overflow = TextOverflow.Ellipsis) }) else null,
        label = if (label != null) ({ Text(label) }) else null,
        leadingIcon = if (leadingIcon != null) ({ Icon(painterResource(leadingIcon), contentDescription = null, tint = p.faint) }) else null,
        singleLine = singleLine,
        minLines = minLines,
        maxLines = if (singleLine) 1 else maxLines,
        enabled = enabled,
        shape = RoundedCornerShape(10.dp),
        visualTransformation = if (password) PasswordVisualTransformation() else VisualTransformation.None,
        keyboardOptions = KeyboardOptions(keyboardType = keyboardType, imeAction = imeAction),
        keyboardActions = KeyboardActions(onAny = {
            focus.clearFocus()
            onAction?.invoke()
        }),
        colors = OutlinedTextFieldDefaults.colors(
            focusedBorderColor = p.accent,
            unfocusedBorderColor = p.border,
            focusedContainerColor = p.bg2,
            unfocusedContainerColor = p.bg2,
            cursorColor = p.accent,
        ),
        modifier = modifier,
    )
}

/** The web's `.card`: a panel with a border and 14dp corners. */
@Composable
fun CardBox(modifier: Modifier = Modifier, flat: Boolean = false, content: @Composable () -> Unit) {
    val p = MaterialTheme.palette
    val shape = RoundedCornerShape(14.dp)
    Box(
        modifier
            .fillMaxWidth()
            .clip(shape)
            .background(if (flat) p.bg2 else p.panel)
            .border(1.dp, p.border, shape)
            .padding(if (flat) 0.dp else 16.dp),
    ) { content() }
}

/** A small spinner for inline "working" states. */
@Composable
fun Spinner(size: Dp = 16.dp) {
    CircularProgressIndicator(color = MaterialTheme.palette.accent, trackColor = MaterialTheme.palette.border, strokeWidth = 2.dp, modifier = Modifier.size(size))
}

/** A back arrow and a title, for pages stacked on a tab (kept for screens that don't use a scrolling top bar). */
@Composable
fun BackBar(title: String, onBack: () -> Unit) {
    Row(Modifier.fillMaxWidth().padding(top = 4.dp, end = 16.dp), verticalAlignment = Alignment.CenterVertically) {
        IconButton(onClick = onBack) { Icon(painterResource(R.drawable.kit_ic_arrow_back), contentDescription = "Back") }
        Text(title, style = MaterialTheme.typography.titleLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
