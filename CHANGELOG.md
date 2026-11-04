# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Added
- `design/tokens.json`: Encore's look defined once (four themes, status colours, accents, presets, radii,
  type, spacing, motion), with `design/gen` (`kit-tokens`) generating CSS variables, a Compose theme, an Iced
  theme and Flutter ThemeData; tests prove the output equals Encore's current CSS, Compose and desktop themes.
- `contracts/`: the shared APIs (health, changelog, notices, bug reports, diagnostics, settings schema,
  update feed) as prose and OpenAPI, matching Encore's current API.
- CI (fmt, clippy, tests, generated files up to date) and the release workflow.
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
