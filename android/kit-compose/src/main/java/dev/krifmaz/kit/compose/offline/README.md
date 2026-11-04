# offline

Offline mode that explains itself (DESIGN.md §6, §11), extracted from Encore's
`clients/android/app/src/main/java/app/encore/android/features/offline/`.

| File | What it is | From Encore |
| --- | --- | --- |
| `OfflineReason.kt` | Why the server can't be reached, in plain words: no network, DNS, refused, timeout, HTTPS, proxy 502–504. | `OfflineReason.kt` |
| `OfflineOutbox.kt` | The per-account outbox of plays and likes made while offline (`Outbox`), and its file (`OutboxFile`). | `OfflineOutbox.kt` |
| `OfflineBanner.kt` | The banner (top bar while offline, with Retry), the sheet it opens, and `probeHealth` (one `/api/health` check). | `OfflineUi.kt`, `OfflineMode.kt` (`probe`) |

The app keeps the state (offline or not, a check on its way, the last reason) and passes it in, with its own words
for what still works offline:

```kotlin
val reason = probeHealth(http, server, phoneOnline, waitSeconds = 8) // null: the server answered
OfflineBanner(
    active = offline, trying = checking,
    title = "Offline", line = "Downloaded songs play · tap for details",
    message = "Offline – showing what this phone saved. Downloaded songs play.",
    reason = reason, status = "12 songs saved on this phone",
    onRetry = { recheck() },
)
```

Tests: `src/test/.../offline` (Encore's `OfflineReasonTest`, `OutboxTest`).
