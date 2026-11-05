# Changelog

All notable changes to the kit. Versions are `vMAJOR.MINOR.PATCH` git tags; apps pin a tag.

## Unreleased

### Added
- kit-server-core: `Migrations::run(&dyn Db)` makes and updates the kit's `kit_*` tables on any SQLite `Db`
  (`SqliteDb::migrate` now uses it), and `SharedDb` (`Arc<dyn Db>`).
- kit-leptos `crash`: a panic puts a banner over the page (what stopped it, where, and Reload; `CrashWords` in
  `Words::crash`, an empty sentence turns it off) and posts its log at once with a plain `fetch`, since after a
  panic the app's tasks may never run again. The log still waits in the pending list until the server has it; the
  server keeps the same key once. The log's text now carries the JavaScript stack and the page.

### Changed
- The SQLite stores (`SqlitePrefs`, `SqliteSettings`, `SqliteNotices`, `SqliteReports`, `SqliteSuite`, `SqliteSignup`) take any
  `Db` (`new(db: impl Db + 'static)`), not only `SqliteDb`: an app with its own SQLite layer keeps the kit's
  tables in its own database, on its own connections. Code that passes a `SqliteDb` is unchanged.

## v0.6.0 — 2026-11-05

Notes: [docs/releases/0.6.0.md](docs/releases/0.6.0.md).

### Added
- Sign-up (`contracts/signup.md`): who may make an account.
  - `kit-server-signup`: the admin setting "Who can sign up" (Closed, Invite only by default, Open); invite links
    (one-time or multi-use, expiry, most uses, the role and presets the account gets, revoke, who used each;
    the link shown once, kept as its SHA-256); open sign-up with limits per address and per day, an optional
    email confirmation (`Mailer`, `LogMailer` by default) and an optional admin approval; the app makes,
    switches on and removes accounts through `SignupHost`. Under single sign-on the account source's setting
    applies and is shown read-only (`SignupHost::account_source`; `suite_account_source` with feature `suite`).
    Data in `kit_signup_settings`, `kit_invites`, `kit_invite_uses` and `kit_signup_requests` on SQLite
    (`SqliteSignup`) or Postgres (`PgSignup`).
  - kit-rules `signup`; kit-leptos `signup` (`SignInLine`, `SignUpPage`, `VerifyPage`, `SignupAdmin`); kit-iced
    `signup` (`SignUp`, `Admin`); kit-compose `signup` (`SignInLine`, `SignUpScreen`, `SignupAdminScreen`,
    `SignupRules`), with `KitRoutes.signup` and kit-leptos `Paths::signup`.
  - The template: sign-up on the web, desktop and Android sign-in pages, a Sign-up admin page on each, and the
    server's accounts made through the hooks (`public_url` setting for the links). The Android demo has the
    admin page on its in-app server.

## v0.5.0 — 2026-11-05

Notes: [docs/releases/0.5.0.md](docs/releases/0.5.0.md).

### Added
- `kit-icons` (design/icons): an app's glyph (one 24 × 24 SVG in `currentColor` and `{accent-2}`) becomes its whole
  icon set on the kit's tile in the theme's colours: `favicon.svg`, `icon-32.png`, `icon-192.png`, `icon-512.png`,
  `icon-maskable-512.png`, `apple-touch-icon.png`, `manifest.webmanifest` and `head.html`. `--check` for CI.
  DESIGN.md §5 "App icons" gives the rules for the glyph.

## v0.4.1 — 2026-11-05

### Fixed
- kit-leptos: `style/kit.css` draws the drop places of `dropzone` (`.drop-ready` while files are dragged over the
  window, `.drop-over` under the pointer); they had no outline without an app's own rules.

## v0.4.0 — 2026-11-05

Notes: [docs/releases/0.4.0.md](docs/releases/0.4.0.md).

### Added
- Suite screens (`contracts/suite.md`) on every client:
  - kit-leptos `suite`: `<SuiteNav/>` (the shared sidebar with Edit sidebar), `<SuiteSearch/>`, `<Related/>`,
    `<SsoButton/>` and `<LinkedApps/>` (the admin page for links and keys).
  - kit-iced `suite`: `Sidebar`, `Search`, `related`, `path_icon`.
  - kit-compose `suite`: `SuiteNav`, `SuiteSearch`, `Related`, `SuiteRules`.
  - The demos show a shared sidebar and a search across apps.
- Template: the app is a suite member. It has a manifest of its pages, its pages in search and single sign-on
  (as the account source, or through one). The settings are `public_url`, `suite_id` and `account_source`, with
  Settings → Linked apps. The web, desktop and Android apps show the shared sidebar and Search. A test runs two
  linked copies side by side.

### Changed
- Template: new apps pin the kit's `v0.4.0` (`kit_tag`).

## v0.3.2 to v0.3.4 — 2026-11-05

### Added
- kit-compose `theme`: `KitPaletteSet`, an app's own palette for any of the four themes, taken by `basePaletteOf`,
  `kitPaletteOf` and `KitTheme(theme, …, palettes)`; accents, the faded background and Material colours are worked
  out from it. The kit's palettes stay the default.
- kit-compose `offline`: `OutboxFile(file, upgrade)` reads an outbox file the app wrote in an earlier form of its
  own, so changes waiting on a phone survive the switch to the kit's outbox.
- `kit-ci.yml`: `apt-packages`, Ubuntu packages the tests need (e.g. `ffmpeg`), installed before the checks run.
- kit-iced: an app's own icons (`Icon::App(AppIcon::stroke(name, body))` or `AppIcon::filled`), drawn like the
  kit's and accepted by every widget that takes an `Icon`; the demo shows one beside the kit's set.
- kit-iced: `whats_new::changelog_with` draws the changelog page in the app's own `ChangelogWords` (re-exported).
- kit-iced: `Details::with_button_icon` puts an icon on the details dialog's main button (e.g. Play).
- Command palette (web):
  - kit-leptos `palette`: a command palette. Ctrl+K / ⌘K (or `palette::open()`, say from a search button) opens it;
    Escape, a press beside it or Ctrl+K again closes it. The app installs its commands once
    (`palette::install(Palette::new(commands).source(|| async { … }))`): fixed ones (`Command::new(group, label, run)`
    with `.hint(…)` and `.icon(…)`, under `ACTIONS`, `GO_TO` or groups of its own) and, optionally, a source that
    loads more each time it opens. Up and Down move the selection (kept in view), Enter or a click runs the command,
    a new query selects the first result again. A dialog with a combobox and a listbox of options under their
    groups; the words are `config.words.palette` (`PaletteWords`). Styled by `kit.css` (`.palette-*`), so an app
    that loads the kit's stylesheet needs nothing else.
  - kit-rules `palette`: what shows and in which order (`results`: with no query every fixed item, then the first
    six of each loaded group; with one, label matches then hint-only matches, case-insensitive), `step` for Up and
    Down, `is_toggle` for Ctrl+K / ⌘K, `PaletteWords`.
  - kit-leptos: the `search` icon.
  - Template: the web app installs the palette with its pages and Report a bug, and the sidebar has a search
    button; the browser walk opens it with Ctrl+K, runs a command and closes it with Escape.

### Changed
- kit-leptos: `style/kit.css` (and the template's copy) draws the kit's look of docs/DESIGN.md §5, which it had
  drifted from: Inter 14px/1.5 body, H1 28 / H2 19 / H3 15·650, small 12, the outlined danger button (red text,
  red border on hover) instead of a filled one, and the same buttons, badges, lists, tables, dialogs, toasts and
  banners the encore theme was drawn from. An app's stylesheet that loads after it no longer mixes two looks.
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
