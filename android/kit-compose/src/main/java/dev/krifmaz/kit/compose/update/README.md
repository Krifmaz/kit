# update — in-app updates from GitHub releases

Extracted from Encore's `clients/android/app/src/main/java/app/encore/android/features/update/`
(`UpdateRules.kt`, `Updater.kt`, `ApkInstaller.kt`, `UpdatesSection.kt`), with the same behaviour and words.

- `UpdateRules` — the rules without Android (unit-tested): newest full release `v<x.y.z>` (hotfixes carry no
  apps), the APK `<apkPrefix><version>.apk`, version compare, only files under the repository's release downloads,
  SHA256SUMS and GitHub digest checks, signer match, release notes as plain text, the status lines.
- `Updater` — check (once a day at start, or by hand; ETag kept), download into `cacheDir/updates`, verify
  (SHA256SUMS, GitHub's digest, package, version code, signing certificate), install through `PackageInstaller`.
- `UpdateInstallReceiver` — the install session's outcome (`ApkInstaller.kt`).
- `UpdatesSection` — Settings → This device → Updates.

## Wiring it in

```kotlin
// Application.onCreate
KitApp.init(name = "Encore", versionName = BuildConfig.VERSION_NAME, versionCode = BuildConfig.VERSION_CODE,
    userAgent = USER_AGENT)
UpdateRules.source = UpdateSource("Krifmaz", "Encore", "Encore-android-", "app.encore.client") // = UpdateSource.ENCORE
// MainActivity.onCreate
Updater.checkAtStart(this)
```

Manifest: `android.permission.REQUEST_INSTALL_PACKAGES` (optionally `UPDATE_PACKAGES_WITHOUT_USER_ACTION`) and
`<receiver android:name="dev.krifmaz.kit.compose.update.UpdateInstallReceiver" android:exported="false" />`.
The updater keeps its settings in the preferences file `<app name in lower case>_update` ("encore_update").
