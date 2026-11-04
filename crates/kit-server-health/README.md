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
