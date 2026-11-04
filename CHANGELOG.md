# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

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
