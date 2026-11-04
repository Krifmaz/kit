# Update feed — in-app updates

The app's GitHub releases are its update feed. The repository (`<owner>/<repo>`), the tag prefixes and the
file name of each platform's download are the app's configuration.

## The feed: the app's GitHub releases

`GET https://api.github.com/repos/<owner>/<repo>/releases?per_page=30`, newest first.

- **Full release:** tag `<release prefix><MAJOR>.<MINOR>.<PATCH>` (prefix `v` unless the app sets another),
  not a draft, not a prerelease. The newest full release is the newest version. It carries the apps — one
  file per platform, named by the app's pattern with `{version}` in it (`Demo-android-{version}.apk`,
  `Demo-{version}-linux-x86_64.AppImage`, …) — and `SHA256SUMS` over every file.
- **Hotfix release** (only for an app that sets a hotfix prefix, such as `hotfix-`): tag
  `<hotfix prefix><version>` (`hotfix-0.36.0.17`). It can be GitHub's "Latest" but carries no apps, so the
  phone and desktop updaters never pick it, and it never shows What's new.
- A download's version is read from its file name, not from the tag: a release made without rebuilding an
  app carries that app's last file over under its old version.
- Versions compare by number. A build without a release number (`0.0.1-dev`) counts every release as newer.
- Only files under `https://github.com/<owner>/<repo>/releases/download/` are used. A download may take at
  most 300 MiB (the app may set another limit) when the release does not give its size. An update installs
  only when its file is listed in `SHA256SUMS` with the same hash (`SHA256SUMS` or `SHA256SUMS.txt`). Notes
  are cut to 4000 characters.

## The server's own update (admin)

The paths are the defaults; an app may serve them elsewhere.

- `GET /api/update[?force=1]` → `{current, dev, latest, tag, hotfix, is_newer, notes, notes_cut, url,
  published_at, checked_at, restart: "ready", …}` plus fields of the app's own (whether and how it can
  update itself, its channel, the last error, …). `force` asks GitHub again instead of the kept answer.
- `POST /api/update {tag?}` → 202 with the app's answer: the app replaces the server with that release
  (default the newest), in its own way.
- `POST /api/restart` → 202 `{"ok": true, "message": "Restarting"}`: the server stops gracefully and exits
  with code 75.

Servers on the hotfix channel are also offered hotfixes; a full release is newer than every hotfix before it.
