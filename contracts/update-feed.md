# Update feed — in-app updates

From Encore's `UpdateRules.kt` (Android), `clients/desktop/src/desktop/updater.rs` and the server's
`GET /api/system/update` (`server/crates/api/src/routes/update.rs`).

## The feed: the app's GitHub releases

`GET https://api.github.com/repos/<owner>/<app>/releases?per_page=30`, newest first.

- **Full release:** tag `v<MAJOR>.<MINOR>.<PATCH>`, not a draft, not a prerelease. The newest full release is
  the newest version. It carries the apps: `<App>-android-<version>.apk`, the desktop installers
  (`<App>-<version>-windows-x86_64.exe`, `<App>-<version>-macos-universal.dmg`,
  `<App>-<version>-linux-x86_64.AppImage`, `<app>_<version>_amd64.deb`) and `SHA256SUMS` over every file.
- **Hotfix release:** tag `hotfix-<version>` (`0.36.0.17`). It can be GitHub's "Latest" but carries no apps,
  so the phone and desktop updaters never pick it, and it never shows What's new.
- A release made without rebuilding the apps carries the last APK over under its own version, so the APK's
  version is read from its file name, not from the tag.
- Versions compare by number. A build without a release number (`0.0.1-dev`) counts every release as newer.
- Only files under `https://github.com/<owner>/<app>/releases/download/` are used. An APK download may take
  at most 300 MiB when the release does not give its size. An update installs only when its file is listed
  in `SHA256SUMS` with the same hash (`SHA256SUMS` or `SHA256SUMS.txt`). Notes are cut to 4000 characters.

## The server's own update — `GET /api/system/update[?force=1]` (admin)

`{current, latest, is_newer, notes, url, published_at, checked_at, restart: "ready", self_update: "ready" |
"unavailable", self_update_reason, image, …}`. `POST /api/system/update {tag?}` → 202: the new image is pulled
and a helper container started. `POST /api/system/restart` → 202: the server stops gracefully and exits with
code 75. Servers on the hotfix channel (`update_channel = hotfix`) are also offered hotfixes.
