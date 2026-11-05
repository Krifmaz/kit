# kit-server-selfupdate

"Update now" for a kit app's server running in Docker, over the Docker socket mounted into its container.
The app's name, image, program and its own checks come in as an **`App`**:

```rust
let app = kit_server_selfupdate::App::new("Demo", "ghcr.io/acme/demo", "/usr/local/bin/demo")
    .version(option_env!("DEMO_VERSION"))   // the helper checks the image holds the version its tag names
    .schema(Some(db::SCHEMA_VERSION))        // and that it reads the running server's database
    .first_self_updating("0.3.0")            // older tags have no helper
    .backups("/config/backups")              // the way back by hand after a migration
    .exit_meaning(3, "the new Demo refused the database because a newer Demo wrote it")
    .migration_in_log(db::migration_in_log);
// first thing in main(): the program may have been started as the helper
if let Some(code) = kit_server_selfupdate::swap::main_if_helper(&app).await {
    std::process::exit(code);
}
```

- **`docker`** — a small Docker Engine API client on the unix socket (`Docker::new(sock)`: `get`, `post`,
  `delete`, `inspect`, `image_exists`, `pull`, `logs`, `stream_file` for an image build, `this_container`),
  plus `demux_logs`, `pull_error`, `build_error`/`build_text`, `container_id_from_mountinfo` and `hostname`.
- **`probe`** — `probe(&app, sock)` → `SelfUpdate { ready, reason, id, name, image }`: the socket is there,
  a socket, writable and answering, and this process runs in a container Docker may replace (not `--rm`,
  not a Swarm task). Each "no" in plain words. Whether the owner switched Update now off is the app's setting.
- **`start`** — the server's part: `check_tag` (letters, digits, `._-`; a leading v dropped; tags before
  `first_self_updating` refused), `start_update` (pull, remove an ended helper of an earlier try, create and
  start the helper: the new image as root with only the socket and no network), `pull_error` in plain words.
- **`swap`** — the helper: inspect the server's container, check the image's version and the database schema,
  stop, rename out of the way, create the new one under the old name with the same settings
  (`plan_create_body`, `without_image_defaults`: networks and aliases, anonymous volumes by name, the old
  image's own values left behind), start, and wait for it to keep running. When create or start fails or it
  stops again, it is removed and the old one comes back; when its log shows a database migration, it is
  started again instead and the old one is kept stopped for the way back by hand. Then the old container,
  the old image's tags and the app's untagged images go.
- **`report`** — `Report`, the step-by-step record of the last update (`check`, `pull`, `helper`,
  `inspect`, `schema`, `stop`, `rename`, `create`, `start`, `verify`, `cleanup`), `read_back` (the ended
  helper's log merged in and the helper removed, then the running version checked), `text` for Copy and
  `json` for the app's update answer. The record is plain serde: where it is kept is the app's.

The routes are `kit-server-updates`' (`Updates::update_now` calls `start_update`).
