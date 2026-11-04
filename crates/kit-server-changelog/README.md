# kit-server-changelog

What's new ([contract](../../contracts/changelog.md)): an app's release notes for the popup it shows once after
an update, and for the full changelog, with a per-account switch to turn the popup off.

- Notes are the app's own: a folder of `<MAJOR.MINOR.PATCH>.md` embedded at build time (`build::embed`) or
  read at start (`Changelog::from_dir`), or any (version, Markdown) list (`Changelog::new`). The app's name
  and version are arguments.
- New is per account: `prefs.whats_new_seen`; only releases newer than it and not newer than the server.
- **Hotfixes never re-trigger it:** `1.4.0.17` shares `1.4.0`'s notes.
- `prefs.whats_new_popup = false` turns the popup off; the list still shows everything.
- At most 4 releases in the popup (`POPUP_RELEASES`), 6 highlights each (`HIGHLIGHTS`).
- Routes default to `/api/changelog`, `/api/changelog/new`, `/api/changelog/seen`; `router_with(ChangelogPaths {…})`
  serves them elsewhere.

```toml
[dependencies]
kit-server-changelog = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
[build-dependencies]
kit-server-changelog = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0", default-features = false }
```

```rust
// build.rs (notes built in)
kit_server_changelog::build::embed(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/releases"), "changelog_gen.rs");

// the server
mod generated { include!(concat!(env!("OUT_DIR"), "/changelog_gen.rs")); }
let changelog = Changelog::new("Demo", VERSION, generated::NOTES);
// or, read at start: Changelog::from_dir("Demo", VERSION, Path::new("/app/releases"))?
impl kit_server_changelog::ChangelogHost for AppState {
    fn changelog(&self) -> &kit_server_changelog::Changelog { &self.changelog }
    fn prefs(&self) -> &dyn kit_server_core::PrefsStore { &self.prefs } // a SqlitePrefs or pg::PgPrefs
}
let app = app.merge(kit_server_changelog::router());
```

Preferences are kept by the app's `ChangelogHost::prefs()`, a `kit_server_core::PrefsStore`: the kit's
`SqlitePrefs` or `pg::PgPrefs` (table `kit_account_prefs`, one JSON object per account id), or the app's own.
The kit never reads the app's accounts table.
