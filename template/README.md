# {{app_title}}

Made from the [Krifmaz kit](https://github.com/Krifmaz/kit) template ({{kit_tag}}): Encore's look and its
ease-of-use features from the first commit.

| Folder | What it is |
| --- | --- |
| `server/` | Axum server with the kit server crates: health, changelog (What's new), notices, bug reports, diagnostics, settings |
| `web/` | Leptos web app with the kit components and the Encore theme, served at `/` by the server |{% if android %}
| `android/` | Compose app with kit-compose, a Baseline Profile and a benchmark module |{% endif %}
| `docs/releases/` | one Markdown file per release: the release notes and What's new |

## Run it

```
cd web && trunk build && cd ..
APP_DATA_DIR=./data APP_STATIC_DIR=./web/dist cargo run -p {{project-name}}
```

Then open <http://localhost:8080>. With Docker:

```
docker build -t {{project-name}} .
docker run --read-only -p 8080:8080 -v {{project-name}}-data:/data {{project-name}}
```

| Variable | Meaning | Default |
| --- | --- | --- |
| `APP_PORT` | the port the server listens on | `8080` |
| `APP_DATA_DIR` | where the database and uploads live (the only folder it writes to) | `/data` |
| `APP_STATIC_DIR` | the built web app | `/app/static` |

## Checks

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
Every merge to main is released as `vX.Y.Z` with the Docker image `{{image}}:<version>` and `:latest`.
