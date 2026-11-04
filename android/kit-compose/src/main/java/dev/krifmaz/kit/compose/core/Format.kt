package dev.krifmaz.kit.compose.core

import java.text.DateFormat
import java.util.Date
import java.util.Locale

// The web's formatting helpers (web/js/ui.js `fmt`), so every client writes sizes, days and ages the same way.

/** `fmt.size`: "0 B", "12 KB", "1.5 GB". */
fun formatSize(bytes: Long): String {
    if (bytes <= 0L) return "0 B"
    val units = arrayOf("B", "KB", "MB", "GB", "TB")
    val i = minOf((Math.log(bytes.toDouble()) / Math.log(1024.0)).toInt(), 4)
    val v = bytes / Math.pow(1024.0, i.toDouble())
    return if (i > 1) "%.1f %s".format(Locale.US, v, units[i]) else "%.0f %s".format(Locale.US, v, units[i])
}

/** `fmt.day`: "Oct 13, 2026" in the phone's language; "—" for nothing. */
fun formatDay(epochSeconds: Double): String {
    if (epochSeconds <= 0.0) return "—"
    return DateFormat.getDateInstance(DateFormat.MEDIUM).format(Date((epochSeconds * 1000).toLong()))
}

/** `fmt.ago`: "just now", "5m ago", "3h ago", "12d ago", else the date; "never" for nothing. */
fun formatAgo(epochSeconds: Double): String {
    if (epochSeconds <= 0.0) return "never"
    val d = System.currentTimeMillis() / 1000.0 - epochSeconds
    return when {
        d < 60 -> "just now"
        d < 3600 -> "${(d / 60).toInt()}m ago"
        d < 86400 -> "${(d / 3600).toInt()}h ago"
        d < 86400 * 30 -> "${(d / 86400).toInt()}d ago"
        else -> DateFormat.getDateInstance(DateFormat.SHORT).format(Date((epochSeconds * 1000).toLong()))
    }
}
