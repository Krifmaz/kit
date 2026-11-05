# Krifmaz Kit — design

The neutral shared base for every Krifmaz app (Encore, FSView, Filebrary, Photovault and future apps): one
look, the same ease-of-use features, and the same fast, safe Rust and Kotlin building blocks — built once,
configured per app, used everywhere.

| Document | Target | Repository | Date |
| --- | --- | --- | --- |
| Design & delivery plan | kit v0.1.0 | github.com/Krifmaz/kit | November 2026 |

This document follows the owner's brief of 2026-11-04. [`Krifmaz-Kit.pdf`](Krifmaz-Kit.pdf) is reference
material for the pieces and features; where it talks about extracting from Encore or switching Encore over,
this document wins.

## 1. What the kit is

- **A base library**, not an app and not a refactor of any app. It ships nothing but version tags
  (`vMAJOR.MINOR.PATCH`). Apps pin a tag and update when they choose.
- **Neutral.** Nothing app-specific: no app names, no domain concepts (music, libraries, photos, files…),
  no app routes, words or assets. The default theme is `encore` (charcoal and cream); it is only a theme.
- **Configured per app.** App name, version, logo, routes, settings schema, changelog source and release
  targets are all inputs. Defaults are neutral.
- **Self-contained.** Work on the kit changes no other repository. Apps adopt the kit in their own time.

## 2. Principles

| Principle | In practice |
| --- | --- |
| Neutral by default | Every word, path and key an app might want different is a parameter with a plain default. |
| Same words everywhere | Every client (web, Android, desktop) shows the same strings for the same feature; a parity test checks it. |
| Tagged releases | Apps depend on a kit tag (Cargo git tag, Gradle package version). One change can't break every app at once. |
| Fast and safe | Rust on servers and the web, Compose on Android, Iced on desktop; distroless non-root containers; no shell for user input; Baseline Profiles and benchmarks from day one. |
| Plain words, copyable errors | Every message says what happened and what to do; every error can be read in full and copied into a bug report. |

## 3. Architecture

```
Krifmaz/kit                 tagged releases · semver · changelog

  design/                          contracts/                       template/
  tokens.json → CSS · Compose ·    OpenAPI + prose for the shared   cargo-generate app with every
  Iced · Flutter                   endpoints                        feature wired in (the proof)

  kit-server-* (Axum)              kit-rules                        android/kit-compose
  health · changelog · notices ·   pure logic, no UI                Compose library (GitHub Packages)
  bug reports · diagnostics ·
  settings · updates

                 kit-leptos                       kit-iced
                 web components                   desktop components

  .github/workflows: kit-ci · kit-release · kit-docker · kit-android · kit-desktop (apps call them)
```

- Tokens feed every theme, so the look is defined once.
- Contracts pin down the shared endpoints, so every server speaks the same API to every client.
- Server crates implement the contracts; kit-rules holds the client logic once; kit-leptos, kit-iced and
  kit-compose draw it on each platform.
- The template wires everything into a new app in one command.

## 4. Repository layout

```
Krifmaz/kit
├── design/            tokens.json, gen/ (kit-tokens generator), out/ (generated themes)
├── contracts/         OpenAPI + prose for every shared endpoint
├── crates/
│   ├── kit-server-core          shared server plumbing the feature crates build on
│   ├── kit-server-health        /api/health: version + web build
│   ├── kit-server-changelog     release notes, per-account "seen", popup switch
│   ├── kit-server-notices       plain-language banners, periodic + on-demand checks
│   ├── kit-server-bugreports    reports, attachments, crash logs, .txt export
│   ├── kit-server-diagnostics   checks with status, message and fix
│   ├── kit-server-settings      schema → API → validation
│   ├── kit-server-updates       release feed (full release vs hotfix)
│   ├── kit-server-suite         linked apps: manifest, shared sidebar, single sign-on, search
│   ├── kit-rules                toasts, changelog, update detection, offline, uploads
│   ├── kit-leptos               web components (+ demo)
│   └── kit-iced                 desktop components (+ demo)
├── android/kit-compose          Compose library (+ demo, baseline profile, benchmark)
├── template/                    cargo-generate starter
├── .github/workflows/           the kit's CI/release and the shared workflows apps call
├── CHANGELOG.md · README.md · CLAUDE.md · docs/DECISIONS.md
```

One repository: a feature usually touches a contract, a server crate, kit-rules and three clients, so one
PR, one version and one changelog cover it.

## 5. Design tokens

`design/tokens.json` → `design/gen` (`cargo run -p kit-tokens`) → `design/out/`:

| Output | Used by | Form |
| --- | --- | --- |
| `tokens.css` | Leptos web apps | `:root { --bg: … }` (the default theme) and `:root[data-theme="dark"]` … |
| `KitTokens.kt` | Compose apps | `KitPalettes.Encore/Dark/Oled/Light`, `KitAppearance`, `KitShapes`, `KitTypography` |
| `iced_tokens.rs` | Iced apps | `THEMES`, `ACCENTS`, `PRESETS`, `surface(theme)`, `accent(theme)` |
| `kit_tokens.dart` | Flutter apps | `KitPalettes.*`, `kitThemeData(palette)` |

Themes: **`encore`** (default: charcoal and cream), **`dark`** (blue-black, violet accent), **`oled`**
(true black), **`light`**. Each theme has bg, bg-2, panel, panel-2, border, text, muted, faint and its own
accent, accent-2 and on-accent. Status colours ok `#22c55e`, warn `#f59e0b`, err `#ef4444`, info `#38bdf8`.
Accents for the picker: cream, violet, pastel, lilac, pink, sky, teal, green, amber, red, or a custom colour.
Type: Inter 14px/1.5 body, H1 28/750, H2 19/700, H3 15/650, small 12, a ui-monospace stack. Radius 14px
(cards), 12 (tiles), 10 (inputs), 8 (thumbs). Shadow `0 10px 30px rgba(0,0,0,.35)`. Motion .12s / .15s / .2s.

### App icons

Every app's icon is the same tile with the app's own glyph on it, made by `design/icons` (`kit-icons`). The app
draws only its glyph: one small SVG on a 24 × 24 grid, painted with `currentColor` (the theme's text, cream on
Encore) and, for one detail at most, `{accent-2}`; strokes 2 units with round ends, nothing finer than 1.5 units
so it still reads at 16 px. The kit puts it on a rounded tile (panel-2 at the top fading to bg, a border-colour
hairline, corner radius 22 % of the side), with the glyph filling the middle 60 % (50 % on the maskable icon,
inside Android's safe circle). `cargo run -p kit-icons -- glyph.svg out/ --name "App"` writes `favicon.svg`,
`icon-32.png`, `icon-192.png`, `icon-512.png`, `icon-maskable-512.png`, `apple-touch-icon.png`,
`manifest.webmanifest` and `head.html` (the tags for `index.html`, serving the folder at `/icons/` or `--base`);
`--check` says whether a folder is up to date. Desktop and Android icons use the same `icon-512.png`.

## 6. Features

Every feature exists on web (kit-leptos), Android (kit-compose) and desktop (kit-iced), on the same rules
(kit-rules) and the same server contract.

| Feature | What the user gets |
| --- | --- |
| What's new & changelog | Release notes pop up once after an update; the full changelog in Settings with a switch to turn the popup off. Hotfixes never re-trigger it. |
| Self-updating web app | An open tab notices the server updated and reloads into the new build by itself, waiting while something holds the page (the app says what); waits out restarts, then reconnects. |
| In-app updates | Checks the app's releases, downloads, verifies SHA256SUMS and installs (APK on Android, installer on desktop). |
| Notice banners | A plain-language banner when something's wrong, with Check again; re-checked automatically. |
| Smart toasts | Error toasts stay until closed and have Copy; duplicates merge and move to newest; Undo toasts; a stacking cap; toasts show above dialogs. |
| Copyable errors | Any cut-off text opens in full, selectable, with Copy. |
| Confirm dialogs | One dialog for "are you sure?" with plain words, a danger style and keyboard support. |
| Bug reports & crash logs | In-app reports with screenshots and attachments; crash logs filed for the admin; .txt export. |
| Diagnostics | One page that runs the app's checks and says what's wrong and how to fix it. |
| Offline mode | Stays usable from cached data, says why it's offline (no network, DNS, refused, timeout, HTTPS, proxy 502–504), queues the app's actions to send later. |
| Drag & drop | Every upload accepts dropped files with the same checks as the file picker. |
| Settings from a schema | The server describes its settings; every client draws the forms. |
| Suite | Apps link to each other by address and key; one sidebar shows every linked app's sections (each person hides, shows and reorders them); one sign-in through the app that keeps the accounts (OIDC); one search across apps and "related in …" cross-links. |

## 7. Contracts

`contracts/` holds OpenAPI 3.1 (`openapi.yaml`) and prose for: health, changelog/what's-new, notices, bug
reports, diagnostics, settings schema, update feed and suite. Default paths are neutral (`/api/health`,
`/api/changelog/*`, `/api/notices`, `/api/bug-reports`, `/api/diagnostics`, `/api/settings/schema`…); an app
may mount them elsewhere. **Rule:** a contract change is a kit release with a changelog entry; a breaking one
raises MAJOR.

## 8. Server crates

Axum crates, one per feature, each added to an app in a few lines and configured with the app's name,
version, routes, data directory and sources. kit-server-core holds the shared plumbing (state, auth and
storage hooks the app provides).

| Crate | Provides |
| --- | --- |
| kit-server-health | Health endpoint with version and web build. |
| kit-server-changelog | Notes from the app's changelog source, per-account seen tracking, popup switch, hotfix rule. |
| kit-server-notices | Register checks; periodic and on-demand re-check; the notices endpoint. |
| kit-server-bugreports | Reports, attachments, crash logs, .txt export, limits. |
| kit-server-diagnostics | Check registry with status, message and fix. |
| kit-server-settings | Schema loading from the app's settings file, settings API, validation. |
| kit-server-updates | Release feed for in-app updates (the app's repo and asset names). |
| kit-server-suite | The app's manifest, links and keys, the shared sidebar, single sign-on (OIDC provider and relying party), search across linked apps. |

## 9. kit-rules

Pure Rust, no UI, tested on its own: toasts (errors stay with Copy, duplicates merge, Undo, stack cap, how
long each kind stays), changelog (grouping notes into lists and tables, when the popup shows), update
detection (when to reload, when to wait for the service worker, back-off while the server restarts), offline
(classifying why the server can't be reached; a per-account outbox of the app's actions), uploads (accept
checks shared by the picker and drag & drop).

## 10. Clients

- **kit-leptos** (web): every feature as a Leptos component on kit-rules, styled by `tokens.css` and the
  kit's own stylesheet; a demo app shows every component. The Leptos version is pinned.
- **kit-compose** (Android, Kotlin + Jetpack Compose, `dev.krifmaz.kit:kit-compose` on GitHub Packages):
  theme from the tokens, every feature, Coil image setup, Baseline Profile + Macrobenchmark setup, JankStats in
  debug. Smoothness rules: Baseline Profile in every release; judge smoothness on release builds; stable keys
  and contentType on lazy lists; fast-changing state read where shown; images decoded at display size;
  disk/network/decoding off the main thread (StrictMode in debug).
- **kit-iced** (desktop, Iced): every feature as Iced components on kit-rules, styled from the tokens.

## 11. Shared release workflows

Apps call them in a few lines; the kit itself only tags.

| Workflow | Does |
| --- | --- |
| `kit-ci.yml` | fmt, clippy, tests, web checks and parity on every PR. |
| `kit-release.yml` | On every push to main: next vX.Y.Z (PATCH by default, MINOR/MAJOR from `docs/releases/<version>.md`), a normal release (never pre-release, rc, beta or edge), notes, SHA256SUMS. |
| `kit-docker.yml` | Multi-arch GHCR image `:<version>` and `:latest`. |
| `kit-android.yml` | Signed APK with Baseline Profile (signing secrets passed by the caller). |
| `kit-desktop.yml` | Windows, Linux and macOS installers. |

Everything app-specific (names, paths, build commands, assets) is an input.

## 12. Template — the proof

`cargo generate krifmaz/kit template` gives a working app: Axum server with every kit server crate, a
Leptos web UI with every kit component and the default theme, an optional Compose Android app with
kit-compose, Baseline Profile and benchmark modules, a desktop app on kit-iced, a distroless non-root
read-only-friendly Docker image with a health check, CI and releases calling the shared workflows, and a
CLAUDE.md with the house rules (no shell for user input, full doc comments, plain wording, copyable errors,
the kit's look, small validated changes).

**Proof:** the template's demo app runs every kit feature on web, Android and desktop with its tests passing
in the kit's CI. That is the only proof needed.

## 13. Using the kit in an app

```toml
[dependencies]
kit-server-health = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
kit-leptos        = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

```kotlin
repositories { maven { url = uri("https://maven.pkg.github.com/Krifmaz/kit") } }
dependencies { implementation("dev.krifmaz.kit:kit-compose:0.1.0") }
```

Updating: read CHANGELOG.md for the new tag, bump it in one app, run its tests, then the next app.

## 14. Technology choices

| Layer | Choice | Why |
| --- | --- | --- |
| Server | Rust · Axum · Tokio | Fast, small, safe. |
| Database | SQLite (Postgres optional) | No extra containers for single-user and family apps. |
| Web | Leptos (Rust → WASM) | Fine-grained updates, little code, router/forms/SSR built in. |
| Android | Kotlin · Jetpack Compose | Native, light, smooth. |
| Desktop | Iced | Pure Rust, GPU-drawn, no browser engine. |
| Images (Android) | Coil 3 | Caching, de-duplication, preloading. |
| Releases | the shared workflows (cargo-dist where it fits) | Same builds and notes for every app. |

## 15. Delivery

1. **Step 1 (first, alone):** tokens and contracts — everything depends on them.
2. **Then in parallel, one PR per track:** server crates; kit-rules; kit-leptos; kit-compose; kit-iced;
   shared workflows; template (merged last, since it wires everything in).

## 16. Definition of done (v0.1.0)

- Tokens generate CSS, Compose, Iced and Flutter themes (encore default, dark, OLED black, light).
- Contracts for health, changelog, notices, bug reports, diagnostics, settings schema and update feed.
- All seven server crates, kit-rules, kit-leptos, kit-compose and kit-iced exist with every feature, tests,
  READMEs and full doc comments, and nothing app-specific.
- The shared workflows exist and the template calls them.
- The template's demo app runs every feature on web, Android and desktop with tests passing.
- CHANGELOG.md lists v0.1.0; v0.1.0 is released. Every merge after that ships its own normal release.

## 17. Working hands-off

- Decide yourself (this document → the PDF → common sense); log non-obvious calls as one line in
  `docs/DECISIONS.md`.
- One PR per track; fix CI and bot findings until green; squash-merge yourself; delete the branch; merge main
  into branches when needed.
- Releases: every merge to main makes a normal `vX.Y.Z` release; PATCH by default, MINOR/MAJOR by adding
  `docs/releases/<version>.md` in the PR (also the release notes). No release before `docs/releases/0.1.0.md`.
- Never skip or disable tests; never force-push main.
- Stop and ask only for a needed secret, paid service or new account.
- One summary at the end: what shipped, tags, PR links, decisions log, and anything app-shaped that was
  removed or generalised.
