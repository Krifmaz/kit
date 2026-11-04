# kit-server-updates

In-app updates for a kit app (contract: [`contracts/update-feed.md`](../../contracts/update-feed.md)):
the app's GitHub releases as an update feed, and the server's own update and restart routes. The repository,
tag prefixes, file names and how the server replaces itself are the app's.

- **`feed`** — what an app updater reads, over the app's **`Feed`**:
  `Feed::new("acme", "demo").hotfix_prefix("hotfix-").asset("android", "Demo-android-{version}.apk")`
  gives the releases API, page and download prefix; `release_prefix` (default `v`), `hotfix_prefix`
  (default none), one `asset(platform, pattern)` per platform id the app picks, and `max_download_bytes`
  (default 300 MiB). `full_release(list)` is the newest full release that is not a draft or prerelease
  (hotfixes carry no apps); `newest(release, current, "android")` the newer release with this device's file
  (its version read from the file name) and its `SHA256SUMS`; `is_release_file(url)` only files under
  `https://github.com/<owner>/<repo>/releases/download/`; `parse_sums`, `verify`, `verify_digest`,
  `sha256_hex`; `notes` cut to 4000 characters.
- **`newer_release`** — the updaters' rule: by number, and a build without a release number (`0.0.1-dev`)
  takes every release.
- **`server`** — the server's own update: `parse_release`, `pick_newest` (the highest release a channel
  counts, never GitHub's "Latest" alone), `Channel`, `is_newer` (with the dev flag) and `answer`, the shared
  part of `GET /api/update`.
- **`Restart`** — "Restart server", planned `RESTART_DELAY` later so the answer gets out first; the app's
  server loop waits for `requested()` and exits with `EXIT_RESTART` (75).
- **`Updates`** / **`router()`** — `GET /api/update[?force=1]`, `POST /api/update {tag?}` → 202 and
  `POST /api/restart` → 202 `{"ok": true, "message": "Restarting"}` (admins), over the app's trait;
  **`router_with(UpdatePaths { update, restart })`** serves them at the app's own paths.

```rust
impl kit_server_updates::Updates for App {
    async fn update_status(&self, force: bool) -> Result<Value> { Ok(my_update_card(self, force).await) }
    async fn update_now(&self, who: &Account, tag: Option<String>) -> Result<Value> { Err(err(409, "Update now is not available here")) }
    fn restart(&self, who: &Account) { self.restart.schedule(kit_server_updates::RESTART_DELAY); }
}
let router = Router::new().merge(kit_server_updates::router::<App>());
```
