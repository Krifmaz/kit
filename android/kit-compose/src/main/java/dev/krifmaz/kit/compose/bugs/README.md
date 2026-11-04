# bugs — bug reports and crash logs

Bug reports with pictures, and crash logs, for any app with a server that speaks `contracts/bug-reports.md`.
Every path comes from `KitApp.routes` (`bugReports`, `clientCrash`), so an app mounts them where it likes.

- `BugRules` — kinds, statuses, filters, queries, limits, picture checks, the words (unit-tested).
- `BugReportsApi` (+ `BugUnread`) — the calls, over the app's `KitApi` and OkHttpClient (which carry the session);
  `BugReportsApi(api, client, base = KitApp.routes.bugReports)`.
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
KitApp.init(name = "Notes", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE)
CrashLog.uploadPending(this, crashHost)   // crashHost: CrashLogHost (scope, signedIn(), deviceId, api, bugReports)
// screens
BugReportsScreen(host, from = "Settings")  // host: BugReportsHost
```

Manifest: a FileProvider with authority `<applicationId>.files` and a `cache-path` for `shared/` (Share of exports).
Preferences files: `<app id>_bugs` (the list's filter) and `<app id>_crash`, where the app id is `KitApp.id`
("notes" for an app named "Notes"). Exports are named `<app id>-bug-reports-<day>.txt`.

The demo app (`android/demo`) shows all of it against its in-memory server.
