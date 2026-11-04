package dev.krifmaz.kit.compose.perf

import android.app.Activity
import android.os.StrictMode
import android.util.Log
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.metrics.performance.JankStats
import androidx.metrics.performance.PerformanceMetricsState

/**
 * The debug-build performance checks every app inherits (DESIGN.md §11, "Smoothness rules"): StrictMode for disk and
 * network on the main thread, and JankStats for slow frames. Both do nothing unless the app passes `debug = true`
 * (its `BuildConfig.DEBUG`), so release builds carry no cost. Smoothness is judged on release builds (R8 on); these
 * only point at the cause while developing.
 */
object KitPerf {
    /** The log tag of both checks. */
    const val TAG = "KitPerf"

    /**
     * StrictMode in a debug build: disk reads and writes and network on the main thread, and leaked closeables, are
     * logged (not fatal). Call it first thing in `Application.onCreate`.
     *
     * @param debug the app's `BuildConfig.DEBUG`.
     */
    fun strictModeInDebug(debug: Boolean) {
        if (!debug) return
        StrictMode.setThreadPolicy(
            StrictMode.ThreadPolicy
                .Builder()
                .detectDiskReads()
                .detectDiskWrites()
                .detectNetwork()
                .penaltyLog()
                .build(),
        )
        StrictMode.setVmPolicy(
            StrictMode.VmPolicy
                .Builder()
                .detectLeakedClosableObjects()
                .penaltyLog()
                .build(),
        )
    }

    /**
     * JankStats in a debug build: every janky frame of [activity]'s window is logged with its length and the UI
     * state ([markState]), while the activity is resumed. Call it in `onCreate` after `setContent`.
     *
     * @param activity the activity to watch; must be a [LifecycleOwner] (every ComponentActivity is).
     * @param debug the app's `BuildConfig.DEBUG`.
     * @return the tracker, or null in a release build.
     */
    fun jankStatsInDebug(activity: Activity, debug: Boolean): JankStats? {
        if (!debug) return null
        val stats =
            JankStats.createAndTrack(activity.window) { frame ->
                if (frame.isJank) Log.d(TAG, "jank: ${frame.frameDurationUiNanos / 1_000_000} ms ${frame.states.joinToString { "${it.key}=${it.value}" }}")
            }
        (activity as? LifecycleOwner)?.lifecycle?.addObserver(
            object : DefaultLifecycleObserver {
                override fun onResume(owner: LifecycleOwner) {
                    stats.isTrackingEnabled = true
                }

                override fun onPause(owner: LifecycleOwner) {
                    stats.isTrackingEnabled = false
                }
            },
        )
        return stats
    }

    /**
     * Names what the screen is doing for the jank log ("screen" = "Library", "scrolling" = "true"); a null [value]
     * removes it. Does nothing when JankStats isn't tracking this view's window.
     *
     * @param activity the activity whose window is tracked.
     * @param key the state's name.
     * @param value the state's value, or null to remove it.
     */
    fun markState(activity: Activity, key: String, value: String?) {
        val state = PerformanceMetricsState.getHolderForHierarchy(activity.window.decorView).state ?: return
        if (value == null) state.removeState(key) else state.putState(key, value)
    }
}
