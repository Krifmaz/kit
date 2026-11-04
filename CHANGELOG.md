# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Changed
- `android/kit-compose` is neutral: no Encore names, music words, routes or fixtures. `KitApp.init` takes the app's
  `id` (preferences and export names; was the name in lower case) and `routes`, a `KitRoutes` with every server path
  the kit calls (defaults `/api/health`, `/api/changelog*`, `/api/notices`, `/api/bug-reports`, `/api/diagnostics`,
  `/api/settings/schema`, `/api/settings`, …), used by `BugReportsApi`, `SchemaApi`, `CrashLog`, `probeHealth` and
  the new `ApiChangelogBackend`. `UpdateSource.ENCORE` is gone (the app sets `UpdateRules.source`), and
  `UpdateRules.updatesPlace` names where Updates is. The offline outbox is generic (`OutAction(kind, key, data)`,
  `OutMerge.Each`/`Latest`, the app's nouns in its words). Settings: `FieldBlock` replaces the VPN block, and
  `neverShown` replaces the built-in WireGuard key. `KitLook.accent` `""` means the theme's own accent. `KitApi`
  gains `put` (with a refusing default).
- The template's release no longer re-uploads the apps under its title. `kit-android.yml` and `kit-desktop.yml` upload as `kit-app-android` / `kit-app-desktop-<OS>`, and `kit-release.yml`
  collects `kit-app-*`, so an app whose name has a space ("Kit Demo") still gets its APK and installers attached;
  the kit's CI checks the three agree.
- `crates/kit-rules` and `crates/kit-leptos` are neutral and configured per app: no Encore names, music concepts,
  routes or words. kit-rules gains `app::App { id, name, version }` (the id prefixes storage keys, events and
  service worker names) and a `Words` struct with the kit's `DEFAULT` per module (`ToastWords`, `DialogWords`,
  `ChangelogWords`, `UpdateWords`, `CheckWords`, `OfflineWords`, `BugWords`, `DiagWords`); the offline outbox holds
  the app's own actions (`Outbox<A: Action, O>`); the update toast's wait reasons come from the app
  (`waiting_for(&[(held, words)])`); `build_in` takes the script's stem; notices are a list (`parse`, `find`);
  diagnostics group the app's own check kinds after the kit's three (`groups` returns `String` headings,
  `groups_with`); a toast stack takes an app's failed-copy words (`with_copy_failed`); the changelog page's line
  reads "This is {app} {version}. After an update, a short list of what changed shows once."
- `crates/kit-leptos`: `config::AppConfig` (app, logo, script stem, `Paths`, `Routes`, `Words`, toast policy;
  `config::set`/`get`) is used by every component; the default paths are `/api/health`, `/api/changelog/*`,
  `/api/notices`, `/api/bug-reports`, `/api/diagnostics`, `/api/settings/*`; the notice banner shows every failing
  notice of `GET /api/notices` (class `notice-banner`, per-notice Check again words); `UpdateConfig::hold` names
  what holds the page; `watch_connection()` reads the configured app; the `app` prop of `ChangelogPage`,
  `ChangelogView`, `DiagnosticsPage` and `DiagnosticsView` is optional; the settings form's words are
  `FormWords`; markup tests compare with the kit's own markup; the white space text nodes that copied Encore's
  templates are gone.
- The template's web app sets its kit configuration (its server's settings and diagnostics paths), loads the
  kit's `kit.css`, and `scripts/template-check.sh` compares that copy instead of Encore's stylesheet.
- Design tokens are neutral: `encore` is the default theme (`:root`), with `dark`, `oled` and `light`; each theme
  carries its own accent; accents `logo`/`encore` are now `cream`/`pastel`; Compose palettes are
  `KitPalettes.Encore/Dark/Oled/Light`; Iced tokens add `accent(theme)`. The Encore fixtures and per-platform
  overrides are gone. `docs/DESIGN.md` and `CLAUDE.md` describe the kit as a neutral base.
- kit-iced is app-neutral: no Encore names, words, icons or logo. `theme::encore()` became
  `theme::default_look()`; `player_bar` became `bottom_bar`; the music icons (library, next, prev, music, disc,
  volume, shuffle) are gone; `widgets::logo` draws the app's `Logo` (`Logo::KIT` as a neutral mark); the details
  dialog's links are `title`, `subtitle` and a main `button` that opens a page (no Play); `update::ready_words`
  takes where the Updates block is; `offline::ENCORE_TEXT` became the neutral `offline::TEXT`.
- `crates/kit-leptos` draws Encore's shell and Settings byte for byte (Encore's web app switched to it with no
  DOM or pixel change): `html::raw`, `html::seq` and `html::opt` add an app's HTML, lists and optional parts
  without Leptos' placeholders; the components keep Encore's attribute order and the white space between its
  tags; toasts fade with Encore's inline styles; a modal stays open until it is closed even when its caller drops
  the handle (`details::init` dialogs showed nothing); `modal::close_all` drops the dialogs a router wipes; the notice
  banner leaves no placeholder in `#server-notice`; the settings card takes an app's HTML before and after its
  fields and a `FieldBlock` (Encore's VPN block); `DiagHooks` gains `check_detail`, `copy`, `loaded` and
  `quiet_errors` for Encore's own Diagnostics parts.
- `kit-server-updates`: `pick_newest` takes the channel and picks the highest release it counts on both channels
  (a hotfix numbered below a newer full release can be GitHub's "Latest"), as Encore now does.
- Shared workflows are app-neutral: no app names, file names or app-specific defaults. `kit-desktop.yml` runs the
  app's own `package` command (with `$V`, `$RELEASE`, `$FEATURES`, `$ARGS`, `$OUT`) instead of a fixed script and
  its flags, takes `linux-args`/`windows-args`/`macos-args`, and defaults to `ubuntu-latest`/`macos-latest`, no
  extra Mac arguments and no NSIS; `kit-release.yml`'s `only: android` is now `keep-previous` (a partial release
  keeps the previous release's other files).

### Added
- `kit-compose` `dialogs` (`Confirm` with type-to-confirm, `ConfirmDialog`, `Confirms`/`ConfirmHost`), `notices`
  (`NoticeBanners`, `rememberNotices`, `NoticeRules`) and `uploads` (`DropZone`, `UploadRules`, `FileLimits`).
- `android/demo` uses every kit-compose feature against an in-memory server (`DemoBackend` behind an OkHttp
  interceptor), with a settings schema of every field kind and unit tests (`DemoServerTest`) in CI.
- `template/`: follows the neutral kit: the web app sets no `data-theme` (the default look is `:root`); the
  desktop app uses `theme::default_look()`, kit-iced's confirm dialog and the app's `Logo` (`Logo::KIT` until
  it has one); the release workflow uses kit-desktop's `package` command, and an `apps` job hands the app
  artifacts to kit-release under the title, so a title with spaces still gets its APK and desktop files attached.
- `template/android`: Sign out asks first in kit-compose's `ConfirmDialog`, as on the web and the desktop.
- `template/web`: Sign out asks first in the kit's confirm dialog (the desktop app asks the same), and the kit's
  browser walk (`scripts/template-smoke.cjs`) checks that Cancel keeps the session and OK signs out.
- `template/desktop/`: the generated app's desktop app (Iced 0.14 on kit-iced and kit-rules): connect to a server
  (remembered in the config folder, `<PROJECT>_SERVER` overrides it), sign in for an app token, then Home (a
  button for every kit feature), Bug reports (list, thread with the reply box, New report with the file dialog
  and drag and drop), Settings (the server's form from its schema, the look, Updates), Diagnostics and What's new
  (popup once per release and the page), with toasts, the details dialog, a confirm dialog, the offline banner
  and the notice banner; the update notice reads the app's own GitHub releases. Unit tests for its rules,
  headless iced_test tests for every page, and `examples/walk.rs`, which makes every request against a running
  server. `template/scripts/desktop-package.sh` makes the Linux tarball, the Windows zip and the macOS dmg the
  update notice looks for; the template's release workflow builds them through `kit-desktop.yml` with "apps",
  its CI tests `desktop/`, and the kit's `template` job checks, packages and walks it.
- `crates/kit-leptos/style/kit.css` (also `kit_leptos::CSS`): the kit's own stylesheet for every component,
  built only on `design/out/tokens.css`, so all four themes draw it.
- A confirm dialog: `kit_rules::dialogs::{Confirm, Answer, DialogWords}`, `kit_leptos::modal::confirm(Confirm)
  -> bool` and the `<ConfirmDialog>` component.
- `kit_leptos::schema_form::SAMPLE_SCHEMA`, a neutral settings schema for demos and tests.
- The kit-leptos demo shows every component for a neutral "Kit Demo" app with a theme switcher (encore, dark,
  oled, light) and checks its DOM against the kit's own markup.
- `template/`: the new-app starter (`cargo generate krifmaz/kit template`): an Axum server with every kit server
  crate wired in (health, What's new, notices with a data folder notice, bug reports and crash logs, diagnostics,
  settings from a settings.toml) and Encore's sign-in (owner at first start, session cookie with the CSRF rule,
  API key, phone app tokens); a Leptos web app with the kit-leptos components and the Encore theme (sign-in,
  shell, settings form, Diagnostics, What's new, bug reports, self-update, offline and notice banners); an
  optional Compose Android app on kit-compose with Baseline Profile and benchmark modules; a distroless, non-root,
  read-only-friendly Docker image with its own health check; CI and releases through the shared workflows; and a
  CLAUDE.md with the house rules. The kit's CI generates an app on every pull request and proves it builds, runs,
  shows every kit feature in a browser and ships a healthy image (`scripts/template-check.sh`), and builds its
  Android app on this checkout's kit-compose.
- `kit-android.yml`: the Gradle steps get the run's `GITHUB_TOKEN`, so an app can read kit-compose from GitHub
  Packages (the caller grants `packages: read`).
- kit-iced: `Icon::Video`.
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
- `android/kit-compose` (`dev.krifmaz.kit:kit-compose`): Encore's Android look and ease-of-use features as a
  Compose library: `KitTheme` from the generated tokens, widgets, toasts, details with Copy, What's new popup and
  changelog screen, in-app updater, bug reports and crash logs, offline banner/reasons/outbox, schema settings
  form, diagnostics screen, Encore's Coil setup and JankStats/StrictMode in debug; a demo app, Baseline Profile and
  Macrobenchmark modules; ktlint and unit tests in CI (`android.yml`), published to GitHub Packages for every
  release (`android-publish.yml`).
- `crates/kit-iced`: desktop components for any app on Iced 0.14 and kit-rules, styled from the generated Iced
  tokens: `theme` (every palette, `theme::default_look()`, every widget style), `widgets` (text styles, buttons,
  cards, fields, modal, badges, skeletons, tabs, icons, the app's `Logo`, text area), `toast` (desktop toast
  rules), `details`, `confirm` (the confirm dialog), `whats_new` (popup and changelog page), `update` (release
  checks, SHA256SUMS, the Updates block), `notice`, `drop_zone`, `settings` (the settings pieces), `form` (the
  settings form from a const table or the server's schema), `bug_report` (labels, file picker, New report dialog,
  reply box), `diagnostics` (with `view_with` for an app's own check groups) and `offline`. Components hold no
  network code: their updates return what the app must request. App names, words, routes and logos are the app's.
- `crates/kit-iced/demo` (`kit-iced-demo`): a desktop app showing every kit-iced component with fake data, and
  headless `iced_test` tests for each.
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

### Removed
- `crates/kit-leptos/demo/encore-app.css` and `crates/kit-leptos/src/schema_form/fixtures/` (Encore's stylesheet
  and settings fixtures) and the tests that compared with Encore's HTML builders (each replaced by a test of the
  kit's own markup).
- Encore-only API: `NoticeSource::encore_downloads`, `CheckWords::DOWNLOADS`, `changelog::SETTINGS_PATH`,
  `update::{WAITS_FOR_MUSIC, WAITS_FOR_UPLOAD, WAITS_FOR_DOWNLOAD}`, the outbox's `Play`/`Like`/`plays_body`/
  `play_seconds`/`like_gone`/`DROP_UNKNOWN_TRACKS`, `offline::mode::{PAGE, likes_key}` and the settings form's
  WireGuard special case.

