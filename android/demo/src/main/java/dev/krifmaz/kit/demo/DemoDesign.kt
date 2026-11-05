package dev.krifmaz.kit.demo

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableDoubleStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.icons.KitIcons
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.ui.Carousel
import dev.krifmaz.kit.compose.ui.DetailHeader
import dev.krifmaz.kit.compose.ui.FilterChips
import dev.krifmaz.kit.compose.ui.Hero
import dev.krifmaz.kit.compose.ui.KitEmptyState
import dev.krifmaz.kit.compose.ui.KitIllustration
import dev.krifmaz.kit.compose.ui.KitNavItem
import dev.krifmaz.kit.compose.ui.KitScaffold
import dev.krifmaz.kit.compose.ui.KitSections
import dev.krifmaz.kit.compose.ui.KitShape
import dev.krifmaz.kit.compose.ui.KitSheet
import dev.krifmaz.kit.compose.ui.MediaCard
import dev.krifmaz.kit.compose.ui.MediaGrid
import dev.krifmaz.kit.compose.ui.ProgressRing
import dev.krifmaz.kit.compose.ui.SectionTitle
import dev.krifmaz.kit.compose.ui.SkeletonGrid
import dev.krifmaz.kit.compose.ui.SkeletonLines
import dev.krifmaz.kit.compose.ui.SkeletonRail
import dev.krifmaz.kit.compose.ui.StatCard
import dev.krifmaz.kit.compose.widgets.Gutter
import dev.krifmaz.kit.compose.widgets.KitButton
import kotlinx.coroutines.delay

/** The made-up items of the design system's screens (the web demo's): title, kind, year. */
val DEMO_ITEMS: List<Triple<String, String, Int>> =
    listOf(
        Triple("Northern Lights", "Story", 2024),
        Triple("Paper Boats", "Guide", 2023),
        Triple("Glass Garden", "Story", 2022),
        Triple("Quiet Hours", "Notes", 2024),
        Triple("Salt and Cedar", "Guide", 2021),
        Triple("The Long Weekend", "Story", 2020),
        Triple("Copper Sky", "Notes", 2024),
        Triple("Field Notes", "Notes", 2019),
        Triple("Small Wonders", "Guide", 2023),
        Triple("Harbour Lights", "Story", 2018),
        Triple("Open Windows", "Guide", 2024),
        Triple("Blue Hour", "Story", 2022),
    )

/** The sections of the design system's screens, as on the web demo. */
private val SECTIONS =
    listOf(
        KitNavItem("home", "Home", "house"),
        KitNavItem("browse", "Browse", "layout-grid"),
        KitNavItem("stats", "Stats", "chart-column"),
        KitNavItem("states", "States", "square-stack"),
    )

/** The design system: the kit's scaffold (bottom bar on a phone, rail or sidebar on wider screens) around four screens and an item's screen. */
@Composable
fun DemoDesign(onBack: () -> Unit) {
    var route by rememberSaveable { mutableStateOf("home") }
    val item = route.removePrefix("item/").toIntOrNull()
    KitScaffold(
        brand = "Kit Demo",
        nav = SECTIONS,
        current = if (item != null) "browse" else route,
        onNavigate = { route = it },
        title = item?.let { DEMO_ITEMS[it].first } ?: SECTIONS.firstOrNull { it.route == route }?.label,
        onBack = if (item != null) ({ route = "browse" }) else onBack,
        actions = {
            IconButton(onClick = { Toasts.show("Search") }) { Icon(KitIcons.Search, contentDescription = "Search", tint = MaterialTheme.palette.text) }
        },
    ) { padding ->
        val open: (Int) -> Unit = { route = "item/$it" }
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(padding).padding(bottom = 24.dp)) {
            when {
                item != null -> ItemScreen(item, open)
                route == "browse" -> BrowseScreen(open)
                route == "stats" -> StatsScreen()
                route == "states" -> StatesScreen()
                else -> HomeScreen(open)
            }
        }
    }
}

/** One item's card. */
@Composable
private fun Card(i: Int, open: (Int) -> Unit, shape: KitShape = KitShape.Poster) {
    val (title, kind, year) = DEMO_ITEMS[i % DEMO_ITEMS.size]
    MediaCard(
        title = title,
        onClick = { open(i % DEMO_ITEMS.size) },
        seed = i + 1,
        subtitle = "$kind · $year",
        badge = if (year == 2024) "New" else null,
        progress = if (i % 5 == 1) 0.35 + (i % 3) * 0.2 else null,
        shape = shape,
    )
}

@Composable
private fun HomeScreen(open: (Int) -> Unit) {
    KitSections {
        Hero(DEMO_ITEMS[0].first, eyebrow = "Featured", text = "A made-up item to show the hero banner: the cover's colours blurred behind it.", seed = 1) {
            KitButton("Open", primary = true) { open(0) }
            KitButton("My list") { Toasts.show("Added to your list") }
        }
        Carousel("Trending", (1 until 12).toList(), key = { it }, onSeeAll = { Toasts.show("See all") }) { Card(it, open) }
        Carousel("Recently added", (6 until 12).toList(), key = { it }, shape = KitShape.Wide) { Card(it, open, KitShape.Wide) }
        Column(Modifier.padding(horizontal = Gutter)) {
            SectionTitle("At a glance")
            StatRow()
        }
    }
}

@Composable
private fun StatRow() {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            StatCard("Items", "1,284", Modifier.weight(1f), icon = "layers", delta = 0.12, hint = "this month")
            StatCard("Opened today", "86", Modifier.weight(1f), icon = "eye", delta = -0.04)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            StatCard("Storage", "41.2 GB", Modifier.weight(1f), icon = "hard-drive", hint = "of 100 GB")
            StatCard("People", "7", Modifier.weight(1f), icon = "users", delta = 0.0)
        }
    }
}

@Composable
private fun BrowseScreen(open: (Int) -> Unit) {
    val kinds = listOf("All", "Story", "Guide", "Notes")
    var kind by rememberSaveable { mutableStateOf("All") }
    var loading by remember { mutableStateOf(false) }
    LaunchedEffect(kind) {
        loading = true
        delay(450)
        loading = false
    }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp), modifier = Modifier.padding(top = 16.dp)) {
        FilterChips(kinds, kind, { kind = it })
        val shown = DEMO_ITEMS.indices.filter { kind == "All" || DEMO_ITEMS[it].second == kind }
        if (loading) {
            SkeletonGrid(6, Modifier.padding(horizontal = Gutter))
        } else {
            MediaGrid(shown, Modifier.padding(horizontal = Gutter)) { Card(it, open) }
        }
    }
}

@Composable
private fun ItemScreen(n: Int, open: (Int) -> Unit) {
    val (title, kind, year) = DEMO_ITEMS[n]
    KitSections {
        DetailHeader(title, subtitle = "$kind · $year · made up for the demo", seed = n + 1, chips = listOf("$year", kind, "1 h 52 min", "English")) {
            KitButton("Open", primary = true) { Toasts.show("Would open it") }
            KitButton("Star") { Toasts.show("Starred") }
        }
        Row(Modifier.padding(horizontal = Gutter), horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            ProgressRing(0.2 + (n % 7) * 0.11, label = "Read")
            Text("Where you left off, as a ring.", color = MaterialTheme.palette.muted)
        }
        Carousel("More like this", (1..8).map { (n + it) % DEMO_ITEMS.size }, key = { it }) { Card(it, open) }
    }
}

@Composable
private fun StatsScreen() {
    var sync by remember { mutableDoubleStateOf(0.62) }
    Column(Modifier.padding(Gutter), verticalArrangement = Arrangement.spacedBy(24.dp)) {
        StatRow()
        SectionTitle("Rings") { KitButton("Step") { sync = if (sync >= 0.99) 0.1 else minOf(1.0, sync + 0.13) } }
        Row(horizontalArrangement = Arrangement.spacedBy(20.dp)) {
            ProgressRing(sync, label = "Sync")
            ProgressRing(0.41, label = "Storage", color = dev.krifmaz.kit.compose.theme.KitStatusColors.Info)
            ProgressRing(0.92, label = "Health", color = dev.krifmaz.kit.compose.theme.KitStatusColors.Ok)
        }
    }
}

@Composable
private fun StatesScreen() {
    var sheet by remember { mutableStateOf(false) }
    Column(Modifier.padding(Gutter), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        KitButton("Open a sheet", primary = true) { sheet = true }
        KitEmptyState("Nothing here yet", text = "Things you add show up here.", art = KitIllustration.Empty) {
            KitButton("Add one", primary = true) { Toasts.show("Would add one") }
        }
        KitEmptyState("No results", text = "Try fewer words, or another spelling.", art = KitIllustration.Search)
        KitEmptyState("You're offline", text = "What you saw last is still here.", art = KitIllustration.Offline)
        KitEmptyState("All done", text = "Nothing waits for you.", art = KitIllustration.Done)
        SectionTitle("Loading")
        SkeletonLines(4)
    }
    SkeletonRail(gutter = Gutter)
    KitSheet(open = sheet, onClose = { sheet = false }, title = "A sheet") {
        Text("On a phone a sheet comes up from the bottom, full width, clear of the gesture bar.", color = MaterialTheme.palette.muted)
        Row(Modifier.padding(top = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            KitButton("Done", primary = true) { sheet = false }
            KitButton("Cancel") { sheet = false }
        }
    }
}
