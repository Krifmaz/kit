# offline

Offline mode that explains itself (DESIGN.md §6, §11).

| File | What it is |
| --- | --- |
| `OfflineReason.kt` | Why the server can't be reached, in plain words: no network, DNS, refused, timeout, HTTPS, proxy 502–504. |
| `OfflineOutbox.kt` | The per-account outbox of changes made while offline (`Outbox` of `OutAction(kind, key, data)`), and its file (`OutboxFile`). Each kind of change either keeps every one (`OutMerge.Each`: comments) or only the latest per key (`OutMerge.Latest`: a star, a setting); at most 5000 per kind wait. `countsText` / `backOnlineText` say what waits and what went, with the app's nouns. |
| `OfflineBanner.kt` | The banner (top bar while offline, with Retry), the sheet it opens, and `probeHealth` (one `KitApp.routes.health` check). |

The app keeps the state (offline or not, a check on its way, the last reason) and passes it in, with its own words
for what still works offline:

```kotlin
val reason = probeHealth(http, server, phoneOnline, waitSeconds = 8) // null: the server answered
OfflineBanner(
    active = offline, trying = checking,
    title = "Offline", line = "Saved notes open · tap for details",
    message = "Offline – showing what this phone saved.",
    reason = reason, status = Outbox.countsText(outbox.counts(), nouns).let { if (it.isEmpty()) "" else "Waiting: $it" },
    onRetry = { recheck() },
)
```

Tests: `src/test/.../offline` (`OfflineReasonTest`, `OutboxTest`).
