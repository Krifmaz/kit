# kit-server-settings

An app's server settings from one table, its `settings.toml`: every setting with its environment variable,
default and whether it is secret, and the Settings pages, cards and fields that show them. The answers are the
contract [`contracts/settings-schema.md`](../../contracts/settings-schema.md).

| Part | Feature | What it does |
|---|---|---|
| `file` | `toml` | Reads and checks a settings.toml (a file that does not fit is refused with the reason) and generates `Keys`, `TABLE`, `SCHEMA` and, optionally, a typed accessor per setting for the app's build script. `parse` gives the same `Table` at run time. |
| `table` | – | The table as Rust data: `KeyDef`, `Section`, `Group`, `Field`, `DefaultValue`, `Table`. |
| `when` | – | A field's visibility rule, `key=value` or `key!=value`. |
| `schema` | – | `GET /api/settings/schema`'s answer for a table. |
| `values` | – | How a saved or environment value takes its setting's type, the secret mask `•••• (saved)`, and the `Values` trait. |
| `Store` | `server` | The values in effect: environment, then saved, then default, with `_meta` sources. The saved values are behind `SettingsStore`: `SqliteSettings` (feature `server`) or `PgSettings` (feature `postgres`) keep them in the kit's own `kit_settings` table, made by migrations; an app may implement the trait on its own storage. |
| `router` | `server` | `GET/PUT /api/settings`, `GET /api/settings/schema`, `GET /api/settings/secret/{key}` (admins; the secret for the owner) for an app that implements `SettingsHost`. `router_with(SettingsPaths { … })` puts them elsewhere. |

Both features are on by default. A build script takes the crate without the server half:

```toml
[build-dependencies]
kit-server-settings = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0", default-features = false, features = ["toml"] }
```

```rust
// build.rs
use kit_server_settings::file::{generate, Codegen};
fn main() {
    println!("cargo:rerun-if-changed=settings.toml");
    let text = std::fs::read_to_string("settings.toml").unwrap();
    // accessors_on: Some("crate::Settings") also writes `settings.app_name()` and friends, which call that
    // type's `bool`/`int`/`float`/`str`; `methods` are its own method names, which no setting may take.
    let gen = Codegen { accessors_on: None, methods: &[] };
    let code = generate(&text, &gen).unwrap_or_else(|e| panic!("settings.toml: {e}"));
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("settings_gen.rs"), code).unwrap();
}
```

```rust
// the app
pub use kit_server_settings::{DefaultValue, Field, Group, KeyDef, Section, Table};
include!(concat!(env!("OUT_DIR"), "/settings_gen.rs"));
pub static SETTINGS: Table = Table { keys: TABLE, sections: SCHEMA };

impl kit_server_settings::SettingsHost for AppState {
    fn settings(&self) -> &dyn kit_server_settings::Values { &self.store } // a Store
}
// at start: the saved values on SQLite (or PgSettings::new(pg).await? with feature `postgres`)
// let store = Store::load(SqliteSettings::new(db.clone())?, SETTINGS).await?;
let app = Router::new().merge(kit_server_settings::router()).with_state(state);
// or, on other paths:
// .merge(kit_server_settings::router_with(SettingsPaths { values: "/api/config".into(), ..Default::default() }))
```

`SettingsHost`'s other methods have defaults and are where an app adds its own behaviour: `owner` (who may see
a secret; the account whose role is `owner`), `dump` (what `GET` answers), `kept` (keys a save never changes),
`unrevealed` (secrets that are never shown again), `before_save` (refuse a save, or add `_warnings`),
`after_save` (follow a save at once) and `log` (the app's activity log; `tracing` by default).

The table's keys are listed A to Z (`TABLE`, `GET /api/settings`); pages, cards and fields keep the file's order.
