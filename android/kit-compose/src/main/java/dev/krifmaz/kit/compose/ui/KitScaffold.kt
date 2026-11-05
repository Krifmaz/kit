package dev.krifmaz.kit.compose.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.NavigationRailItemDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.icons.KitIcons
import dev.krifmaz.kit.compose.theme.KitSizes
import dev.krifmaz.kit.compose.theme.palette

/** The kit icon called [name], or the dot of `ellipsis` when the kit doesn't ship it. */
fun kitIcon(name: String): ImageVector = KitIcons.byName(name) ?: KitIcons.Ellipsis

/**
 * The app's frame (kit-leptos `AppShell`): the navigation for the window's width around the screen's [content].
 *
 * Below 768 dp (phones) a top app bar (back button, title, [actions]) and a bottom bar of up to five sections, the
 * rest under More in a sheet; 768–1023 dp a navigation rail; from 1024 dp the full sidebar with [sideFoot]. The
 * bars keep clear of the status bar, the cutout and the gesture bar.
 *
 * @param brand the app's name (the sidebar's head, the app bar's title when [title] is null).
 * @param nav the sections, in order.
 * @param current the route on screen, in the sections' form ([KitNavItem.isActive]).
 * @param onNavigate opens a section's route.
 * @param title the screen's title in the app bar.
 * @param onBack the app bar's back button; none when null.
 * @param tabs how many tabs fit in the bottom bar (2–5).
 * @param content the screen, with the padding the bars take.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KitScaffold(
    brand: String,
    nav: List<KitNavItem>,
    current: String,
    onNavigate: (String) -> Unit,
    modifier: Modifier = Modifier,
    title: String? = null,
    onBack: (() -> Unit)? = null,
    actions: @Composable RowScope.() -> Unit = {},
    sideFoot: @Composable ColumnScope.() -> Unit = {},
    tabs: Int = 5,
    moreLabel: String = "More",
    backLabel: String = "Back",
    content: @Composable (PaddingValues) -> Unit,
) {
    val p = MaterialTheme.palette
    BoxWithConstraints(modifier.fillMaxSize().background(p.bg)) {
        val width = maxWidth.value.toInt()
        if (width < DesignRules.PHONE_BELOW) {
            val (shown, more) = DesignRules.tabSplit(nav.size, tabs)
            var moreOpen by remember { mutableStateOf(false) }
            val rest = nav.drop(shown)
            Scaffold(
                containerColor = p.bg,
                contentWindowInsets = WindowInsets.safeDrawing,
                topBar = { KitAppBar(title ?: brand, onBack, backLabel, actions) },
                bottomBar = {
                    NavigationBar(containerColor = p.bg2, contentColor = p.text) {
                        val colors =
                            NavigationBarItemDefaults.colors(
                                selectedIconColor = p.accent,
                                selectedTextColor = p.text,
                                indicatorColor = p.accent.copy(alpha = 0.16f),
                                unselectedIconColor = p.muted,
                                unselectedTextColor = p.muted,
                            )
                        nav.take(shown).forEach { item ->
                            NavigationBarItem(
                                selected = item.isActive(current),
                                onClick = { onNavigate(item.route) },
                                icon = { Icon(kitIcon(item.icon), contentDescription = null) },
                                label = { Text(item.label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                colors = colors,
                            )
                        }
                        if (more) {
                            NavigationBarItem(
                                selected = rest.any { it.isActive(current) },
                                onClick = { moreOpen = true },
                                icon = { Icon(KitIcons.Ellipsis, contentDescription = null) },
                                label = { Text(moreLabel, maxLines = 1) },
                                colors = colors,
                            )
                        }
                    }
                },
            ) { padding -> content(padding) }
            KitSheet(open = moreOpen, onClose = { moreOpen = false }, title = moreLabel) {
                rest.forEach { item ->
                    SheetLink(item, item.isActive(current)) {
                        moreOpen = false
                        onNavigate(item.route)
                    }
                }
            }
        } else {
            Row(Modifier.fillMaxSize()) {
                if (width < DesignRules.WIDE_FROM) {
                    NavigationRail(
                        containerColor = p.bg2,
                        contentColor = p.text,
                        modifier = Modifier.fillMaxHeight(),
                        windowInsets = WindowInsets.safeDrawing,
                    ) {
                        Spacer(Modifier.size(8.dp))
                        val colors =
                            NavigationRailItemDefaults.colors(
                                selectedIconColor = p.accent,
                                selectedTextColor = p.text,
                                indicatorColor = p.accent.copy(alpha = 0.16f),
                                unselectedIconColor = p.muted,
                                unselectedTextColor = p.muted,
                            )
                        nav.forEach { item ->
                            NavigationRailItem(
                                selected = item.isActive(current),
                                onClick = { onNavigate(item.route) },
                                icon = { Icon(kitIcon(item.icon), contentDescription = null) },
                                label = { Text(item.label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                colors = colors,
                            )
                        }
                    }
                } else {
                    Sidebar(brand, nav, current, onNavigate, sideFoot)
                }
                Column(Modifier.weight(1f).fillMaxHeight()) {
                    KitAppBar(title ?: brand, onBack, backLabel, actions)
                    Box(Modifier.weight(1f).fillMaxWidth()) { content(PaddingValues(0.dp)) }
                }
            }
        }
    }
}

/** The top app bar: back button (or nothing), the title and the actions, clear of the status bar. */
@Composable
fun KitAppBar(title: String, onBack: (() -> Unit)?, backLabel: String = "Back", actions: @Composable RowScope.() -> Unit = {}) {
    val p = MaterialTheme.palette
    Column(Modifier.fillMaxWidth().background(p.bg.copy(alpha = 0.94f)).windowInsetsPadding(WindowInsets.statusBars)) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = KitSizes.AppBar).padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            if (onBack != null) {
                IconButton(onClick = onBack) { Icon(KitIcons.ArrowLeft, contentDescription = backLabel, tint = p.text) }
            } else {
                Spacer(Modifier.width(12.dp))
            }
            Text(
                title,
                modifier = Modifier.weight(1f),
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = p.text,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            actions()
        }
        HorizontalDivider(color = p.border.copy(alpha = 0.7f))
    }
}

/** The full sidebar from 1024 dp: the app's name, every section, the footer. */
@Composable
private fun Sidebar(brand: String, nav: List<KitNavItem>, current: String, onNavigate: (String) -> Unit, foot: @Composable ColumnScope.() -> Unit) {
    val p = MaterialTheme.palette
    Column(
        Modifier
            .width(236.dp)
            .fillMaxHeight()
            .background(p.bg2)
            .windowInsetsPadding(WindowInsets.safeDrawing)
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            brand,
            color = p.text,
            fontWeight = FontWeight.Bold,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.padding(12.dp),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        nav.forEach { item -> SheetLink(item, item.isActive(current)) { onNavigate(item.route) } }
        Spacer(Modifier.weight(1f))
        HorizontalDivider(color = p.border)
        foot()
    }
}

/** One section as a row: icon and name, 44 dp or taller, the current one marked. */
@Composable
private fun SheetLink(item: KitNavItem, active: Boolean, onClick: () -> Unit) {
    val p = MaterialTheme.palette
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = KitSizes.Touch)
            .clip(RoundedCornerShape(12.dp))
            .background(if (active) p.panel2 else p.panel2.copy(alpha = 0f))
            .clickable(role = Role.Tab, onClick = onClick)
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Icon(kitIcon(item.icon), contentDescription = null, tint = if (active) p.accent else p.muted, modifier = Modifier.size(20.dp))
        Text(item.label, color = p.text, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/**
 * A sheet over the screen (kit-leptos `Sheet`): from the bottom, full width on phones, with a grab handle; back, a
 * swipe down or a tap outside closes it.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KitSheet(open: Boolean, onClose: () -> Unit, title: String, closeLabel: String = "Close", content: @Composable ColumnScope.() -> Unit) {
    if (!open) return
    val p = MaterialTheme.palette
    ModalBottomSheet(
        onDismissRequest = onClose,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = p.panel,
        contentColor = p.text,
    ) {
        Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
            IconButton(onClick = onClose) { Icon(KitIcons.X, contentDescription = closeLabel) }
        }
        Column(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, bottom = 20.dp)) { content() }
    }
}
