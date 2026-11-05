//! `cargo run -p kit-icons -- <glyph.svg> <out-dir> --name <App name> [--short-name <name>] [--start-url <url>]
//! [--theme <id>] [--check]`: writes the app's icon set into `<out-dir>`; with `--check`, only says whether
//! `<out-dir>` is up to date (exit code 1 when it is not).

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: kit-icons <glyph.svg> <out-dir> --name <App name> [--short-name <name>] \
                     [--start-url <url>] [--base <path>] [--theme <id>] [--check]";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let mut paths = Vec::new();
    let (mut base, mut name, mut short_name, mut start_url, mut theme, mut check) = (None, None, None, None, "encore".to_string(), false);
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{arg} needs a value.\n{USAGE}"));
        match arg.as_str() {
            "--name" => name = Some(value()?),
            "--short-name" => short_name = Some(value()?),
            "--start-url" => start_url = Some(value()?),
            "--base" => base = Some(value()?),
            "--theme" => theme = value()?,
            "--check" => check = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            _ if arg.starts_with("--") => return Err(format!("Unknown option {arg}.\n{USAGE}")),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    let [glyph, out] = <[PathBuf; 2]>::try_from(paths).map_err(|_| USAGE.to_string())?;
    let name = name.ok_or_else(|| format!("--name is needed.\n{USAGE}"))?;
    // the kit's tokens are built in, so `cargo install` gives a binary that runs anywhere
    let tokens = kit_tokens::Tokens::parse(include_str!("../../tokens.json")).map_err(|e| e.to_string())?;
    let colors = kit_icons::Colors::from_theme(&tokens, &theme).map_err(|e| e.to_string())?;
    let source = std::fs::read_to_string(&glyph).map_err(|e| format!("Could not read {}: {e}", glyph.display()))?;
    let app = kit_icons::App { name, short_name, start_url, base };
    let files = kit_icons::render(&source, &colors, &app).map_err(|e| e.to_string())?;
    let mut stale = Vec::new();
    for (file, bytes) in files {
        let path = out.join(file);
        if std::fs::read(&path).ok().as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        if check {
            stale.push(file);
            continue;
        }
        std::fs::create_dir_all(&out)
            .and_then(|()| std::fs::write(&path, bytes))
            .map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        println!("wrote {}", path.display());
    }
    if stale.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        eprintln!("{} is stale ({}): run kit-icons again and commit the result.", out.display(), stale.join(", "));
        Ok(ExitCode::FAILURE)
    }
}
