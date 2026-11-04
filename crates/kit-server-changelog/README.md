# kit-server-changelog

What's new ([contract](../../contracts/changelog.md)): an app's release notes for the popup it shows once after
an update, and for the full changelog in Settings, with a per-account switch to turn the popup off.

- Notes are the app's `docs/releases/<MAJOR.MINOR.PATCH>.md`, embedded at build time by `build::embed`.
- New is per account: `prefs.whats_new_seen`; only releases newer than it and not newer than the server.
- **Hotfixes never re-trigger it:** `0.36.0.17` shares `0.36.0`'s notes.
- `prefs.whats_new_popup = false` turns the popup off; the list still shows everything.
- At most 4 releases in the popup (`POPUP_RELEASES`), 6 highlights each (`HIGHLIGHTS`).

```toml
[dependencies]
kit-server-changelog = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
[build-dependencies]
kit-server-changelog = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0", default-features = false }
```

```rust
// build.rs
kit_server_changelog::build::embed(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/releases"), "changelog_gen.rs");

// the server
mod generated { include!(concat!(env!("OUT_DIR"), "/changelog_gen.rs")); }
impl kit_server_changelog::ChangelogHost for AppState {
    fn changelog(&self) -> &kit_server_changelog::Changelog { &self.changelog } // Changelog::new("MyApp", VERSION, generated::NOTES)
}
let app = app.merge(kit_server_changelog::router());
```

Preferences default to the JSON text in the account row's `prefs` column, saved with
`UPDATE users SET prefs=? WHERE id=?`; override `prefs` / `save_prefs` for another store.

Extracted from Encore's `server/crates/api/src/routes/changelog.rs` and `build.rs`.
