//! The build-script half: embeds `docs/releases/<MAJOR.MINOR.PATCH>.md` into a Rust file in `OUT_DIR`, so the
//! server carries its notes. (An app that would rather read the folder at start uses
//! `Changelog::from_dir` instead.)
//!
//! ```no_run
//! // build.rs, with `kit-server-changelog = { …, default-features = false }` under [build-dependencies]
//! let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/releases");
//! kit_server_changelog::build::embed(&dir, "changelog_gen.rs");
//! // the crate: mod generated { include!(concat!(env!("OUT_DIR"), "/changelog_gen.rs")); }  → generated::NOTES
//! ```

use std::path::{Path, PathBuf};

/// The release-notes files in `dir`: (version, path) of every `MAJOR.MINOR.PATCH.md`, in name order. Other
/// files (`README.md`, four-part or `-beta` names) are left out. `None` when the folder can't be read.
pub fn notes_in(dir: &Path) -> Option<Vec<(String, PathBuf)>> {
    let rd = std::fs::read_dir(dir).ok()?;
    let mut notes: Vec<(String, PathBuf)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let v = name.strip_suffix(".md")?;
            // plain releases only: 1.4.0, not 1.4.0.1 or 1.4.0-beta
            let parts: Vec<&str> = v.split('.').collect();
            (parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())))
                .then(|| (v.to_string(), e.path().canonicalize().unwrap_or(e.path())))
        })
        .collect();
    notes.sort();
    Some(notes)
}

/// The Rust source of `NOTES`: (version, markdown) pairs, each file `include_str!`-ed.
pub fn source(notes: &[(String, PathBuf)]) -> String {
    let mut code =
        String::from("/// The release notes as (version, Markdown), in version order.\npub static NOTES: &[(&str, &str)] = &[\n");
    for (v, path) in notes {
        code.push_str(&format!("    ({v:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    code.push_str("];\n");
    code
}

/// Run from a build script: writes `OUT_DIR/<file>` with `NOTES` from the notes in `dir`, and asks Cargo to
/// build again when the folder changes. Without the folder the list is empty (and Cargo prints a warning).
pub fn embed(dir: &Path, file: &str) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let notes = notes_in(dir).unwrap_or_else(|| {
        println!("cargo:warning=no release notes at {}: the changelog is empty", dir.display());
        Vec::new()
    });
    let out = Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR is set in a build script")).join(file);
    std::fs::write(out, source(&notes)).expect("OUT_DIR is writable");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_releases_are_embedded() {
        let dir = std::env::temp_dir().join(format!("kit-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["1.4.0.md", "0.9.1.md", "1.4.0.1.md", "1.4.0-beta.md", "README.md", "1.4.1.txt"] {
            std::fs::write(dir.join(f), "## x\n").unwrap();
        }
        let n = notes_in(&dir).unwrap();
        assert_eq!(n.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>(), vec!["0.9.1", "1.4.0"]);
        let src = source(&n);
        assert!(
            src.starts_with("/// The release notes as (version, Markdown), in version order.\npub static NOTES: &[(&str, &str)] = &[\n")
        );
        assert!(src.contains("(\"1.4.0\", include_str!(") && src.ends_with("];\n"));
        assert!(notes_in(&dir.join("missing")).is_none());
        assert_eq!(
            source(&[]),
            "/// The release notes as (version, Markdown), in version order.\npub static NOTES: &[(&str, &str)] = &[\n];\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
