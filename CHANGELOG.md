# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Added
- `kit-ci.yml`: `apt-packages`, Ubuntu packages the tests need (e.g. `ffmpeg`), installed before the checks run.
- kit-iced: an app's own icons (`Icon::App(AppIcon::stroke(name, body))` or `AppIcon::filled`), drawn like the
  kit's and accepted by every widget that takes an `Icon`; the demo shows one beside the kit's set.
- kit-iced: `whats_new::changelog_with` draws the changelog page in the app's own `ChangelogWords` (re-exported).
- kit-iced: `Details::with_button_icon` puts an icon on the details dialog's main button (e.g. Play).

### Changed
- kit-iced: `widgets::confirm` takes an owned title too (`impl Into<String>`), for titles that carry a name.

### Fixed
- Template: the server, web and desktop manifests are `Cargo.toml.liquid`, so an app that takes the kit as a git
  dependency no longer prints three `invalid character` errors about `{{project-name}}` on every build. The generated
  app is the same; the template needs cargo-generate 0.23.4 or later.

## v0.3.1 — 2026-11-05

### Fixed
- Postgres stores: a statement run with a null where a later call passes a number (a bug report file's
  `message_id`, for one) no longer fails now and then with "invalid byte sequence for encoding UTF8: 0x00";
  statements with a null parameter are no longer kept prepared on the connection.

## v0.3.0 — 2026-11-05

Notes: [docs/releases/0.3.0.md](docs/releases/0.3.0.md).

### Added
- Suite (`contracts/suite.md`): apps on the kit link to each other, share one sidebar, one sign-in and one search.
  - `kit-server-suite`: `GET /api/kit/manifest` (the app's id, name, icon, sections with icons, base URL, deep-link
    patterns, search endpoint, sign-on), keys for other apps (`/api/kit/suite/keys`, shown once, kept hashed) and
    links to them by address and key (`/api/kit/suite/links`), the person's shared sidebar
    (`GET`/`PUT /api/kit/suite/sidebar`), the app's own search for linked apps (`/api/kit/search`) and one search
    across every linked app (`/api/kit/suite/search`, `scope=others` for cross-links). Single sign-on: an account
    source is an OpenID Connect provider (`/.well-known/openid-configuration`, `/api/kit/oidc/*`), and linked apps
    sign people in through it (`/api/kit/sso*`, `SuiteHost::sso_sign_in`). Links and keys in `kit_suite_links` and
    `kit_suite_keys` on SQLite (`SqliteSuite`) or Postgres (`PgSuite`).
  - kit-rules `suite`: the manifest, the sidebar in each person's order with hidden entries, deep-link patterns,
    search groups and the suite's words.

## v0.2.2 — 2026-11-05

Notes: [docs/releases/0.2.2.md](docs/releases/0.2.2.md).

### Fixed
- kit-leptos: `<id>:server-down` (the "server isn't answering" banner) is debounced: it goes out only after 2
  failed calls in a row, or 5 s after a failure with no call answered since (`api::DownWatch`). A failed request
  (no answer at all) counts the same way. Aborted calls, and calls cut off while the page is being left, count
  for nothing and fail with `api::CANCELLED`.

## v0.2.1 — 2026-11-05

### Fixed
- kit-leptos: the connection banner ("The server isn't answering") shows only when the server can't be reached:
  a proxy's own page or an unmarked 502–504. A 502–504 the server itself sends (marked `x-<app id>`) is one
  call's error and no longer raises it; `api::server_down` says which.

## v0.2.0 — 2026-11-04

Notes: [docs/releases/0.2.0.md](docs/releases/0.2.0.md).

### Added
- `kit_server_changelog::prefs_router(path)`: the account's preferences (`GET`, and `PUT` to merge; a null removes a
  key), where the What's new popup switch lives, for apps without a preferences route of their own. The template
  uses it instead of its own copy.
- kit-leptos: `HttpBackend::with_headers(|method, path| …)` adds an app's own headers to every call (a CSRF token,
  `If-Match`); a failed answer's sentence also comes from a `message` or `error` text when there is no `detail`;
  `api::patch` sends `PATCH`.
- kit-leptos: `crash::install` sends the web app's crash logs (panics, uncaught errors, unhandled
  rejections) to the server's crash reports, keeping them until it can.

### Fixed
- kit-server-bugreports: the same crash sent twice at once is kept once instead of failing with a 500.

## v0.1.3 — 2026-11-04

### Changed
- `docs/GOALS.md` keeps only the shared app block and the kit's own goal, with no app names; each app's goals
  live in its own repo.

## v0.1.2 — 2026-11-04

### Changed
- rusqlite 0.37 → 0.39 (libsqlite3-sys 0.37), so an app on sqlx 0.9's SQLite driver links one sqlite3 with the
  kit's `SqliteDb`.

## v0.1.1 — 2026-11-04

### Added
- `docs/GOALS.md`: the goals for the kit and the apps built on it, linked from the README; the template's
  `CLAUDE.md` starts every new app with the shared app block.

## v0.1.0 — 2026-11-04

The first release: the neutral shared base for every app, configured per app. Notes:
[docs/releases/0.1.0.md](docs/releases/0.1.0.md).

### Added
- `design/`: `tokens.json` and `kit-tokens`, generating CSS variables, a Compose theme, an Iced theme and
  Flutter ThemeData; themes `encore` (default), `dark`, `oled`, `light`; accents and presets.
- `contracts/`: health, changelog, notices, bug reports, diagnostics, settings schema and update feed, as prose
  and OpenAPI.
- `kit-server-core` and seven server crates: `kit-server-health`, `-changelog`, `-notices`, `-bugreports`,
  `-diagnostics`, `-settings`, `-updates`. Routes, words, checks and notices are the app's. Storage sits behind
  a trait per crate with a SQLite store and a Postgres store (sqlx, `postgres` feature), in `kit_*` tables
  created by migrations.
- `kit-rules`: client decisions as plain Rust (no UI, wasm32): toasts, details, What's new, versions, update
  detection, notices, offline and outbox, uploads, confirm dialogs, bug reports, diagnostics; `App` and `Words`
  configure them per app.
- `kit-leptos` (+ demo): web components for every feature, configured by `AppConfig`, with `kit.css`.
- `kit-iced` (+ demo): desktop components for every feature on Iced 0.14, with headless tests.
- `android/kit-compose` (+ demo, Baseline Profile, benchmark): `dev.krifmaz.kit:kit-compose` on GitHub Packages,
  configured by `KitApp.init(id, routes, …)`; the demo covers every feature, including updates, the bug report
  sheet, the crash log and the offline banner.
- Shared workflows: `kit-ci.yml`, `kit-release.yml`, `kit-docker.yml`, `kit-android.yml`, `kit-desktop.yml`; app
  artifacts travel under the `kit-app-` prefix.
- `template/`: `cargo generate krifmaz/kit template` → server (SQLite), Leptos web app, optional Android app,
  desktop app, distroless non-root Docker image, CI and releases on the shared workflows, CLAUDE.md. The kit's CI
  generates it on every pull request and runs its demo on web, Android and desktop, and runs the server crates'
  tests on Postgres.
- The kit's own CI and release workflow (no release before `docs/releases/0.1.0.md`), and
  `cleanup-branches.yml`, which deletes merged branches after every push to main.
