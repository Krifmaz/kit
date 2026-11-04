# perf

Debug-build checks, and the smoothness rules every app inherits (DESIGN.md §11):

- Baseline Profile shipped in every release APK (see `android/baselineprofile` and `android/benchmark`; the demo app
  shows the setup).
- Judge smoothness only on release builds (R8 on).
- Stable `key` and `contentType` on every lazy list.
- Fast-changing state (e.g. a progress bar's position) read only where it's shown, ideally in the draw phase.
- Images decoded at display size, preloaded ahead of the scroll (`images`).
- Disk, network and decoding never on the main thread (StrictMode in debug).

```kotlin
// Application.onCreate
KitPerf.strictModeInDebug(BuildConfig.DEBUG)
// Activity.onCreate, after setContent
KitPerf.jankStatsInDebug(this, BuildConfig.DEBUG)
KitPerf.markState(this, "screen", "Home")
```

Both log to the `KitPerf` tag and do nothing in release builds.
