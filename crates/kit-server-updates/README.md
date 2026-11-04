# kit-server-updates

In-app updates for a kit app (contract: [`contracts/update-feed.md`](../../contracts/update-feed.md)):
the app's GitHub releases as an update feed, and the server's own update and restart routes. Extracted
from Encore's `encore_system::update`, `routes/update.rs`, Android's `UpdateRules.kt` and the desktop
`updater.rs`.

- **`feed`** — what an app updater reads: `Repo::new("Krifmaz", "Encore")` gives the releases API, page and
  download prefix; `full_release(list)` the newest `vX.Y.Z` that is not a draft or prerelease (hotfixes carry
  no apps); `Repo::newest(release, current, Platform::Android)` the newer release with this device's file
  (the APK's version from its file name; a desktop file named by the tag) and its `SHA256SUMS`;
  `Repo::is_release_file(url)` only files under `https://github.com/<owner>/<app>/releases/download/`;
  `parse_sums`, `verify`, `verify_digest`, `sha256_hex`; `notes` cut to 4000 characters; `MAX_APK_BYTES`.
- **`newer_release`** — the updaters' rule: by number, and a build without a release number (`0.0.1-dev`)
  takes every release.
- **`server`** — the server's own update: `parse_release`, `pick_newest` (the highest release a channel counts, never GitHub's "Latest" alone), `Channel`,
  `is_newer` (with the dev flag) and `answer`, the shared part of `GET /api/system/update`.
- **`Restart`** — "Restart server", planned `RESTART_DELAY` later so the answer gets out first; the app's
  server loop waits for `requested()` and exits with `EXIT_RESTART` (75).
- **`Updates`** / **`router()`** — `GET /api/system/update[?force=1]`, `POST /api/system/update {tag?}` → 202
  and `POST /api/system/restart` → 202 `{"ok": true, "message": "Restarting"}` (admins), over the app's
  trait. How the app replaces itself (Encore: a Docker helper container) is the app's.

```rust
impl kit_server_updates::Updates for App {
    async fn update_status(&self, force: bool) -> Result<Value> { Ok(my_update_card(self, force).await) }
    async fn update_now(&self, who: &Account, tag: Option<String>) -> Result<Value> { Err(err(409, "Update now is not available here")) }
    fn restart(&self, who: &Account) { self.restart.schedule(kit_server_updates::RESTART_DELAY); }
}
let router = Router::new().merge(kit_server_updates::router::<App>());
```
