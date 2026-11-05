package dev.krifmaz.kit.compose.ui

import kotlin.math.roundToInt

/**
 * One section of the app's navigation (kit-leptos `ui::NavItem`).
 *
 * @property route where it goes, in the app's own form ("library", "#/library").
 * @property label its name.
 * @property icon a kit icon name ([dev.krifmaz.kit.compose.icons.KitIcons.byName]).
 */
data class KitNavItem(val route: String, val label: String, val icon: String) {
    /**
     * Whether the screen at [current] belongs to this section: the same route or one under it ("library/3" under
     * "library"). The home section ("", "/", "#/", "#") is active only on itself. The web's rule.
     */
    fun isActive(current: String): Boolean {
        fun home(s: String) = s.trimEnd('/') in setOf("", "#")
        if (home(route)) return home(current)
        val base = route.trimEnd('/')
        if (current == base) return true
        if (!current.startsWith(base)) return false
        val rest = current.substring(base.length)
        return rest.startsWith("/") || rest.startsWith("?")
    }
}

/** The design system's rules, the same as kit-leptos `ui` (and tested against the same cases). */
object DesignRules {
    /** Below this width (dp) an app is laid out for phones: app bar, bottom bar, sheets. */
    const val PHONE_BELOW = 768

    /** From this width (dp) the full sidebar shows instead of the rail. */
    const val WIDE_FROM = 1024

    /** How the bottom bar splits [count] sections with room for [tabs] (2–5): the tabs it shows, and whether More holds the rest. */
    fun tabSplit(count: Int, tabs: Int = 5): Pair<Int, Boolean> {
        val room = tabs.coerceIn(2, 5)
        return if (count <= room) count to false else (room - 1) to true
    }

    /** A change as people read it ("+12%", "−3%" with a real minus sign, "0%") and its direction: "up", "down" or "flat". */
    fun deltaText(d: Double): Pair<String, String> {
        val pct = (d * 100).roundToInt()
        return when {
            pct > 0 -> "+$pct%" to "up"
            pct < 0 -> "−${-pct}%" to "down"
            else -> "0%" to "flat"
        }
    }

    /** Up to two capital letters of [title] (the first letter or digit of its first two words). */
    fun initials(title: String): String = title
        .split(Regex("\\s+"))
        .mapNotNull { w -> w.firstOrNull { it.isLetterOrDigit() } }
        .take(2)
        .joinToString("") { it.uppercase() }

    /** The two hues (0–359) of the placeholder art of [seed]: the web's `placeholder_art` colours. */
    fun hues(seed: Int): Pair<Int, Int> {
        val h = (seed.toLong() and 0xFFFFFFFFL) * 2_654_435_761L and 0xFFFFFFFFL
        val a = (h % 360).toInt()
        val b = ((a + 40 + (h ushr 9) % 80) % 360).toInt()
        return a to b
    }

    /** A progress value kept between 0 and 1 (not a number is 0). */
    fun unit(v: Double): Double = if (v.isNaN()) 0.0 else v.coerceIn(0.0, 1.0)
}
