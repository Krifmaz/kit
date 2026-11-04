# update — in-app updates from GitHub releases

The app replaces itself with the newest release of its own GitHub repository. Where it looks is the app's input:
`UpdateSource(owner, repo, apkPrefix, releasePackage)`; until it is set (`UpdateSource.NONE`) nothing is offered.

- `UpdateRules` — the rules without Android (unit-tested): newest full release `v<x.y.z>` (hotfixes carry no
  apps), the APK `<apkPrefix><version>.apk`, version compare, only files under the repository's release downloads,
  SHA256SUMS and GitHub digest checks, signer match, release notes as plain text, the status lines.
  `UpdateRules.updatesPlace` names where Updates is in the app ("Settings"), for the "new version" toast.
- `Updater` — check (once a day at start, or by hand; ETag kept), download into `cacheDir/updates`, verify
  (SHA256SUMS, GitHub's digest, package, version code, signing certificate), install through `PackageInstaller`.
- `UpdateInstallReceiver` — the install session's outcome (`ApkInstaller.kt`).
- `UpdatesSection` — the Updates part of the app's settings.

## Wiring it in

```kotlin
// Application.onCreate
KitApp.init(name = "Notes", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE)
UpdateRules.source = UpdateSource(owner = "acme", repo = "notes", apkPrefix = "notes-android-", releasePackage = "com.acme.notes")
// MainActivity.onCreate
Updater.checkAtStart(this)
// MainActivity.onResume (back from "Install unknown apps")
Updater.onResume(this)
```

Only a build whose package is `releasePackage` replaces itself (debug builds with another id only look).
Manifest: `android.permission.REQUEST_INSTALL_PACKAGES` (optionally `UPDATE_PACKAGES_WITHOUT_USER_ACTION`) and
`<receiver android:name="dev.krifmaz.kit.compose.update.UpdateInstallReceiver" android:exported="false" />`.
The updater keeps its settings in the preferences file `<app id>_update` (`KitApp.id`).
