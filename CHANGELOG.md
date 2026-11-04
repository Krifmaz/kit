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
- `android/kit-compose` (`dev.krifmaz.kit:kit-compose`): Encore's Android look and ease-of-use features as a
  Compose library: `KitTheme` from the generated tokens, widgets, toasts, details with Copy, What's new popup and
  changelog screen, in-app updater, bug reports and crash logs, offline banner/reasons/outbox, schema settings
  form, diagnostics screen, Encore's Coil setup and JankStats/StrictMode in debug; a demo app, Baseline Profile and
  Macrobenchmark modules; ktlint and unit tests in CI (`android.yml`), published to GitHub Packages for every
  release (`android-publish.yml`).
