# kit-server-settings

An app's server settings from one table, its `settings.toml`, taken from Krifmaz/Encore
(`server/crates/config`: `settings.toml`, `build.rs`, `settings.rs`, `when.rs`, the `Config` loader, and the
settings routes of `server/crates/api/src/routes/admin.rs`). The answers are the contract
[`contracts/settings-schema.md`](../../contracts/settings-schema.md).

| Part | Feature | What it does |
|---|---|---|
| `file` | `toml` | Reads and checks a settings.toml (every check Encore's build script made, with the same refusal reasons) and generates `Keys`, `TABLE`, `SCHEMA` and a typed accessor per setting for the app's build script. `parse` gives the same `Table` at run time. |
| `table` | – | The table as Rust data: `KeyDef`, `Section`, `Group`, `Field`, `DefaultValue`, `Table`. |
| `when` | – | A field's visibility rule, `key=value` or `key!=value`. |
| `schema` | – | `GET /api/admin/settings/schema`'s answer for a table. |
| `values` | – | How a saved or environment value takes its setting's type, the secret mask `•••• (saved)`, and the `Values` trait. |
| `Store` | `server` | The values on the app's database (`settings` table): environment, then saved, then default, with `_meta` sources. |
| `router` | `server` | `GET/PUT /api/admin/settings`, `GET /api/admin/settings/schema`, `GET /api/admin/settings/secret/{key}` for an app that implements `SettingsHost`. |

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
    let gen = Codegen { accessors_on: Some("crate::Config"), methods: &["load", "get", "str", "bool", "int", "float"] };
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
    fn settings(&self) -> &dyn kit_server_settings::Values { &self.store } // a Store<YourDb>
}
let app = Router::new().merge(kit_server_settings::router()).with_state(state);
```

`SettingsHost`'s other methods have defaults and are where an app adds its own behaviour; Encore's are the
examples: `owner` (who may see a secret), `dump` (Encore credits plugin fields), `kept` (`api_key`),
`unrevealed` (the WireGuard config), `before_save` (the signup mode check, folder warnings as `_warnings`),
`after_save` (plugins and Soulseek follow the save) and `log` (the activity log).

The generated code and the schema answer are Encore's byte for byte: Encore's build script uses this crate,
and its recorded schema fixture (`web-rs/src/settings/fixtures/settings-schema.json`) still matches.
