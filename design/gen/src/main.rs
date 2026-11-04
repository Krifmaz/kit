//! `cargo run -p kit-tokens [-- --check]`: writes design/out/ from design/tokens.json; with `--check`, only
//! says whether design/out/ is up to date (exit code 1 when it is not).

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let check = std::env::args().any(|a| a == "--check");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tokens = match kit_tokens::Tokens::load(&root.join("design/tokens.json")) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let out = root.join("design/out");
    let mut stale = Vec::new();
    for (name, text) in kit_tokens::outputs(&tokens) {
        let path = out.join(name);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
            continue;
        }
        if check {
            stale.push(name);
        } else if let Err(e) = std::fs::create_dir_all(&out).and_then(|()| std::fs::write(&path, text)) {
            eprintln!("Could not write {}: {e}", path.display());
            return ExitCode::FAILURE;
        } else {
            println!("wrote design/out/{name}");
        }
    }
    if stale.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("design/out is stale ({}): run `cargo run -p kit-tokens` and commit the result.", stale.join(", "));
        ExitCode::FAILURE
    }
}
