# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Added
- kit-iced: `Icon::Video` (Encore desktop's "Save video to this device" button).
- `design/tokens.json`: Encore's look defined once (four themes, status colours, accents, presets, radii,
  type, spacing, motion), with `design/gen` (`kit-tokens`) generating CSS variables, a Compose theme, an Iced
  theme and Flutter ThemeData; tests prove the output equals Encore's current CSS, Compose and desktop themes.
- `contracts/`: the shared APIs (health, changelog, notices, bug reports, diagnostics, settings schema,
  update feed) as prose and OpenAPI, matching Encore's current API.
- CI (fmt, clippy, tests, generated files up to date) and the release workflow.
- Shared release workflows in `.github/workflows/` (Encore's pipeline, made reusable): `kit-ci.yml` (fmt, clippy,
  tests, web checks and parity), `kit-release.yml` (next vX.Y.Z — PATCH by default, MINOR/MAJOR from
  `docs/releases/<version>.md` — a normal release with notes and SHA256SUMS, the apps attached or carried over),
  `kit-docker.yml` (multi-arch GHCR `:<version>` and `:latest`), `kit-android.yml` (signed APK, optional Baseline
  Profile check) and `kit-desktop.yml` (Windows, Linux and macOS installers). The kit's CI now runs through
  `kit-ci.yml` and lints every workflow with actionlint.
- `crates/kit-rules`: the client-side decisions Encore writes once per client, as plain Rust (no UI, builds
  for wasm32): toasts (web, desktop and Android presets), the details dialog, What's new (notes parsing,
  grouping, when the popup shows), versions, update detection (health, service worker, back-off, reload gate),
  notices, offline (reasons, offline mode, per-account outbox) and uploads (accept checks, drop zone, limits),
  with Encore's words as constants and Encore's tests ported.
- `crates/kit-iced`: Encore's desktop components on Iced 0.14 and kit-rules, styled from the generated Iced
  tokens: `theme` (every palette, `theme::encore()`, every widget style), `widgets` (text styles, buttons, cards,
  fields, modal, badges, skeletons, tabs, icons, text area), `toast` (desktop toast rules), `details`,
  `whats_new` (popup and changelog page), `update` (release checks, SHA256SUMS, the Updates block), `notice`,
  `drop_zone`, `settings` (the settings pieces), `form` (the settings form from a const table or the server's
  schema), `bug_report` (labels, file picker, New report dialog, reply box), `diagnostics` and `offline`.
  Components hold no network code: their updates return what the app must request.
- `crates/kit-leptos`: Encore's web components as Leptos 0.8 components on kit-rules, drawing Encore's HTML
  node for node: toasts (with the global `toast`/`toast_action`), copy to clipboard, modals and the confirm
  dialog, the details dialog, What's new (popup and changelog page), the self-update reload (health polling and
  the service worker), the notice banner, drop zones, the schema settings form (Encore's fixtures pass), the
  bug report sheet, the Diagnostics page, and the offline and connection banners.
- `crates/kit-leptos/demo`: a trunk app showing every component with fake data, checking its DOM against
  Encore's HTML in the browser; CI builds kit-leptos and the demo for wasm32 (check, clippy, build).
- kit-rules: `bugs` (bug report kinds, statuses, the sheet's words and checks), `diagnostics` (the page's
  words, groups and copied report), `dialogs` (`CANCEL`), and `offline::mode::NOT_CONNECTED` / `server_down`.
- `kit-server-core`: what the server crates share: the `{"detail": …}` error answer, the `Host` trait (the app's
  database and sign-in check) with `SignedIn` / `Admin` extractors, the JSON-row `Db` trait, version compare,
  `SqliteDb` and a `TestHost` for tests.
- `kit-server-health`: `GET /api/health` with the version and the web build hash read from the script's name.
- `kit-server-changelog`: What's new: release notes embedded from `docs/releases` at build time, the popup
  (newest unseen releases, per-account seen, popup switch, hotfixes never re-trigger it) and the full list.
- `kit-server-settings`: an app's server settings from one settings.toml (Encore's form). It checks the file with Encore's refusal reasons and generates `Keys`/`TABLE`/`SCHEMA`/typed accessors in the app's build script, keeps the values (environment, then saved, then default, with `_meta` sources), answers the settings schema exactly as `contracts/settings-schema.md`, and serves GET/PUT `/api/admin/settings` and the saved-secret route through a small `SettingsHost` trait.
- `kit-server-diagnostics`: a registry of checks (name, kind, ok, detail, request, status, fix and the app's own fields) run all at once for `GET /api/system/diagnostics`, the settings cards' Test buttons for `POST /api/system/test/{service}`, and room for the app's own sections in the answer.
- `kit-server-notices`: plain-language banners: register notices (id, severity, sentence, Check again toasts), check them again on a timer and on demand, serve them at `GET /api/notices` / `POST /api/notices/check` or inside the app's own answer.
- `kit-server-updates`: an app's GitHub releases as an update feed (full release vs hotfix, each device's file, the APK version from its name, release-download URLs only, SHA256SUMS checks, notes cut at 4000) and the server's own `GET`/`POST /api/system/update` and `POST /api/system/restart`.
- `kit-server-bugreports`: Encore's bug reports (threads, statuses, unread counts, screenshot and log
  uploads, zip / plain text export) and app crash reports (`/api/diagnostics/client-crash`), behind a
  `BugReportsHost` trait for the app's folder, rate limiter, log, notifications, mail and enrichment.
