# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Changed
- `crates/kit-leptos` draws Encore's shell and Settings byte for byte (Encore's web app switched to it with no
  DOM or pixel change): `html::raw`, `html::seq` and `html::opt` add an app's HTML, lists and optional parts
  without Leptos' placeholders; the components keep Encore's attribute order and the white space between its
  tags; toasts fade with Encore's inline styles; `modal::close_all` drops the dialogs a router wipes; the notice
  banner leaves no placeholder in `#server-notice`; the settings card takes an app's HTML before and after its
  fields and a `FieldBlock` (Encore's VPN block); `DiagHooks` gains `check_detail`, `copy`, `loaded` and
  `quiet_errors` for Encore's own Diagnostics parts.

### Added
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
- `crates/kit-leptos`: Encore's web components as Leptos 0.8 components on kit-rules, drawing Encore's HTML
  node for node: toasts (with the global `toast`/`toast_action`), copy to clipboard, modals and the confirm
  dialog, the details dialog, What's new (popup and changelog page), the self-update reload (health polling and
  the service worker), the notice banner, drop zones, the schema settings form (Encore's fixtures pass), the
  bug report sheet, the Diagnostics page, and the offline and connection banners.
- `crates/kit-leptos/demo`: a trunk app showing every component with fake data, checking its DOM against
  Encore's HTML in the browser; CI builds kit-leptos and the demo for wasm32 (check, clippy, build).
- kit-rules: `bugs` (bug report kinds, statuses, the sheet's words and checks), `diagnostics` (the page's
  words, groups and copied report), `dialogs` (`CANCEL`), and `offline::mode::NOT_CONNECTED` / `server_down`.
