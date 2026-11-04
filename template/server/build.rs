//! Build script: the settings table (settings.toml → settings_gen.rs) and the release notes
//! (../docs/releases/<version>.md → changelog_gen.rs), both built into the server, and `APP_TITLE`.

use kit_server_settings::file::{generate, Codegen};
use std::path::Path;

fn main() {
    let out = std::env::var("OUT_DIR").expect("OUT_DIR is set in a build script");
    println!("cargo:rerun-if-changed=settings.toml");
    let text = std::fs::read_to_string("settings.toml").expect("settings.toml is next to Cargo.toml");
    let code = generate(&text, &Codegen { accessors_on: None, methods: &[] }).unwrap_or_else(|e| panic!("settings.toml: {e}"));
    std::fs::write(Path::new(&out).join("settings_gen.rs"), code).expect("OUT_DIR is writable");
    let notes = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/releases");
    kit_server_changelog::build::embed(&notes, "changelog_gen.rs");
    println!("cargo:rerun-if-env-changed=APP_VERSION");
    // the app's name as people see it (settings.toml's default app_name): the web app's CSRF header value
    let table: toml::Table = text.parse().expect("settings.toml is TOML");
    let title = table["keys"]["app_name"]["default"].as_str().expect("keys.app_name has a string default");
    println!("cargo:rustc-env=APP_TITLE={title}");
}
