# template — the kit's new-app starter

```
cargo generate krifmaz/kit template
```

A [cargo-generate](https://github.com/cargo-generate/cargo-generate) template that gives a new app everything the
kit has from the first commit (DESIGN.md §13). This file describes the template; it is not copied into the new
app (its `README.md` is the app's own).

| You get | Where |
| --- | --- |
| Axum server with the kit server crates wired in: health, changelog, notices, bug reports, diagnostics, settings | `server/` |
| Leptos web UI with the kit-leptos components and the kit's default theme | `web/` |
| Optional Compose Android app with kit-compose, Baseline Profile and benchmark modules | `android/` (answer yes to "Add the Android app") |
| Distroless, non-root, read-only-friendly Docker image with a health check | `Dockerfile` |
| CI and releases through the kit's shared workflows | `.github/workflows/` |
| House rules for every session | `CLAUDE.md` |

## Questions it asks

| Name | Meaning | Default |
| --- | --- | --- |
| `project-name` | the repository, crate, binary and image name (`filebrary`) | asked |
| `app_title` | the name people see (`Filebrary`) | the project name, each word capitalised |
| `github_owner` | the GitHub owner of the new repository | `Krifmaz` |
| `kit_tag` | the kit release the app pins | `v0.1.0` |
| `android` | add the Android app | no |

Without questions: `cargo generate krifmaz/kit template --name filebrary --define app_title=Filebrary --silent`.

`hooks/init.rhai` derives `image` (`ghcr.io/<owner>/<name>`), `android_app` (the crate name without underscores),
`android_package` (`dev.krifmaz.<android_app>`) and `app_file` (the title without spaces, for file names).

## How it is tested

The kit's CI (`template` job in `.github/workflows/ci.yml`) generates an app from this folder on every pull
request, points its kit dependencies at the pull request's own crates, and runs its fmt, clippy and tests —
which start the server and check every kit endpoint — then builds the web app and the Docker image, runs the
image read-only and waits for its health check.

## Editing the template

Rust and CSS files (and the Gradle wrapper) are copied as they are (`exclude` in `cargo-generate.toml`), so they
carry no Liquid and build as normal code; names come from `env!("CARGO_PKG_NAME")` and the environment. Files that
need the app's name (`Cargo.toml`, `Dockerfile`, workflows, `index.html`, Gradle files, docs) use Liquid; workflow
lines with GitHub's own `${{ … }}` are wrapped in `{% raw %}`. Kotlin files are templated too, for their package
line (`package {{android_package}}`) and the benchmarks' package name; they must hold no other `{{` or `{%`. Their
folders are `dev/krifmaz/{{android_app}}` (cargo-generate turns a `/` inside a placeholder into `_`).
