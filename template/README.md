# {{app_title}}

Made from the [Krifmaz kit](https://github.com/Krifmaz/kit) template ({{kit_tag}}): the kit's look and its
ease-of-use features from the first commit.

| Folder | What it is |
| --- | --- |
| `server/` | Axum server with the kit server crates: health, changelog (What's new), notices, bug reports, diagnostics, settings |
| `web/` | Leptos web app with the kit components and the kit's default theme, served at `/` by the server |{% if android %}
| `android/` | Compose app with kit-compose, a Baseline Profile and a benchmark module |{% endif %}
| `docs/releases/` | one Markdown file per release: the release notes and What's new |

## First steps

1. Make the lock files and commit them (CI and the Docker build use `--locked`):
   `(cd server && cargo generate-lockfile) && (cd web && cargo generate-lockfile)`.
2. Push to GitHub. Every pull request runs the checks; every merge to main is a release.
3. The first start makes the owner account: set `APP_ADMIN_PASSWORD` (and `APP_ADMIN_USER`, default `admin`),
   or read the password it prints (also in `initial_admin_password.txt` in the data folder).

## Run it

```
cd web && trunk build && cd ..
cd server && APP_DATA_DIR=../data APP_STATIC_DIR=../web/dist cargo run
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
| `APP_ADMIN_USER`, `APP_ADMIN_PASSWORD` | the owner made at the first start | `admin`, a printed password |
| `APP_NAME`, `APP_SESSION_DAYS`, `APP_API_KEY` | settings (settings.toml); set here they are locked in Settings | |

## Checks

In `server/` and `web/`: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
Every merge to main is released as `vX.Y.Z` with the Docker image `{{image}}:<version>` and `:latest`.
