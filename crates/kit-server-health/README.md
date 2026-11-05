# kit-server-health

`GET /api/health` ([contract](../../contracts/health.md)): `{ok, version, server, web}` — the server's version
and the build of the web app it serves at `/`. No sign-in needed. Clients ask it every minute (and every 3 s,
then 15 s, while the server restarts) to notice an update and reload into the new build (kit-rules `update`).

```rust
let web = kit_server_health::web_build(&static_dir, "demo-web-"); // the hash in trunk's script name
let app = app.merge(kit_server_health::router(kit_server_health::Health::new(VERSION, web)));
// another path:
let paths = kit_server_health::HealthPaths { health: "/status".into() };
let app = app.merge(kit_server_health::router_with(kit_server_health::Health::new(VERSION, None), paths));
```

- `web_build(dir, prefix)` reads the hash from `<prefix><hash>.js` (12+ hex digits; trunk drops leading zeros).
  The prefix is the app's own (its web crate's name and a `-`); the kit has no default.
- `HealthPaths` (default `/api/health`) moves the route; `router(health)` uses the default.
- `Health.extra` adds the app's own fields; they never override `ok`, `version`, `server` or `web`.

## The app-shell service worker

`worker_router(Worker::new(app_id, web))` serves `GET /sw.js`: the kit's worker ([`src/sw.js`](src/sw.js)) with
`self.KIT_SW = {app, build, api}` above it, made for the web build the server serves, sent with
`Cache-Control: no-cache`. The web app registers it with kit-leptos `update::register_worker("/sw.js")` (https or
localhost only).

- It keeps the page and the files it names in `<app>-shell-<build>`; pages come from the network first and from
  that cache when the network fails, so the app opens with no connection. Files with a content hash in their name
  come from the cache first.
- It never touches the API (`Worker.api`, `/api/`): what an app keeps of its own data offline is its own.
- Each new web build is a new worker, which installs next to the old one and waits; kit-rules `update::Flow`
  announces the new version and lets it take over (`skip-waiting`), and the worker answers `version?` with
  `{type: '<app>:version', cache}`. An activated worker deletes the older `<app>-shell-` caches.
- `Worker.path` moves it (a worker controls only the pages under its own folder).

