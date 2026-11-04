# bugs — bug reports and crash logs

Extracted from Encore's `features/bugs/` and `CrashLog.kt` / `CrashRules.kt`, with the same behaviour and words.
The server side is `contracts/bug-reports.md` (`/api/bug-reports…`, `/api/crash-reports`).

- `BugRules` — kinds, statuses, filters, queries, limits, picture checks, the words (unit-tested).
- `BugReportsApi` (+ `BugUnread`) — the calls, over the app's `KitApi` and OkHttpClient (which carry the session).
- `BugReportsEntry`, `BugReportsScreen`, `BugReportScreen`, `NewBugReportSheet`, `AttachmentPicker` — the screens.
  They take a `BugReportsHost`: the calls, the image loader, server URLs, whether the viewer is an admin, and the
  app's navigation.
- `CrashLog` (+ `CrashRules`, `CrashLogReport`) — crash capture, Android's exit records, the phone's own crash log
  (.txt), sending reports to the server and "Send crash log" as a `crash` bug report. Sending takes a `CrashLogHost`.
- `saveExport`, `shareFile`, `exportDir` — exports go straight to Downloads, with Share on the toast.

## Wiring it in

```kotlin
// Application.onCreate, first thing
CrashLog.install(this)
KitApp.init(...)
CrashLog.uploadPending(this, crashHost)   // crashHost: CrashLogHost (scope, signedIn(), deviceId, api, bugReports)
// screens
BugReportsScreen(host, from = "Settings")  // host: BugReportsHost
```

Manifest: a FileProvider with authority `<applicationId>.files` and a `cache-path` for `shared/` (Share of exports).
Preferences files: `<app>_bugs` (the list's filter) and `<app>_crash` ("encore_bugs", "encore_crash" for Encore).
