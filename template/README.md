# {{app_title}}

Made from the [Krifmaz kit](https://github.com/Krifmaz/kit) template ({{kit_tag}}): the kit's look and its
ease-of-use features from the first commit.

| Folder | What it is |
| --- | --- |
| `server/` | Axum server with the kit server crates: health, changelog (What's new), notices, bug reports, diagnostics, settings, linked apps (suite) |
| `web/` | Leptos web app with the kit components and the kit's default theme, served at `/` by the server |
| `desktop/` | Desktop app (Linux, Windows, macOS) on Iced with kit-iced: connect, app-token sign-in, every kit feature, the update notice on the app's own releases |{% if android %}
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

## Linked apps

Apps made from this template (and any other app on the kit) link to each other: one sidebar with every linked
app's pages (each person hides, shows and reorders them), one search across them, and one sign-in. The app says
what it is at `/api/kit/manifest`: its pages are `SECTIONS` in `server/src/suite.rs`, with its `ICON`.

1. Set each app's address (`APP_PUBLIC_URL`) and, for two copies of this same app, an id each (`APP_SUITE_ID`).
   The app that keeps the accounts sets `APP_ACCOUNT_SOURCE=true`. These are read at start.
2. In that app: Settings → Linked apps → New key, with the other app's address. Copy the key (it shows once).
3. In the other app: Settings → Linked apps → paste the first app's address and the key, and tick Account source.

People then see both apps in the sidebar, find things in both under Search, and sign in to the second app with
"Sign in with …".

## Checks

In `server/`, `web/` and `desktop/`: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
Every merge to main is released as `vX.Y.Z` with the Docker image `{{image}}:<version>` and `:latest`.
Desktop installers (Linux tar.gz, Windows zip, macOS dmg) build with Actions → release → "apps". Signing is optional,
from the secrets `WINDOWS_CERT_PFX_BASE64`, `WINDOWS_CERT_PASSWORD`, `MACOS_CERT_P12_BASE64`, `MACOS_CERT_PASSWORD`,
`MACOS_SIGN_IDENTITY`, `APPLE_ID`, `APPLE_TEAM_ID` and `APPLE_APP_PASSWORD`; without them the zip is unsigned and the
dmg is signed ad hoc.
