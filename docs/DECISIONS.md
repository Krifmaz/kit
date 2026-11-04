# Decisions

One line per non-obvious call (DESIGN.md first, then Encore's behaviour, then common sense).

- 2026-11-04 · The design docs went straight to main as the first commit, since an empty repo has no base for a PR; everything after goes through PRs.
- 2026-11-04 · Tokens keep Android's current differences from the web (logo accent in every palette, cream OLED text, charcoal accent on Light) as `compose.overrides`, so the generated Compose theme equals Encore's exactly.
- 2026-11-04 · The generated CSS is only the theme-variable blocks of Encore's app.css (lines 1–72); the rest of the stylesheet stays Encore's and is reused unchanged by kit-leptos.
- 2026-11-04 · The notice contract keeps Encore's object ({ok, text, checking}) and adds `id` and `severity`; Encore keeps its paths, other apps get `/api/notices`.
- 2026-11-04 · Diagnostics keeps Encore's check shape ({name, kind, ok, detail, request}) and adds optional `status` and `fix`.
- 2026-11-04 · The kit's release workflow makes no release until docs/releases/0.1.0.md exists, so the first tag is v0.1.0 once §18 passes.
- 2026-11-04 · rustfmt max_width 140 (Encore's code is written to long lines; fmt is now enforced in CI).
- 2026-11-04 · Package names: Rust crates `kit-*`; Android `dev.krifmaz.kit:kit-compose` (as DESIGN.md §15).
- 2026-11-04 · The server crates share an eighth small crate, `kit-server-core` (error answer, `Host`/`Db` traits, `SignedIn`/`Admin`, version compare, `SqliteDb`, `TestHost`), so the seven feature crates don't each repeat them.
- 2026-11-04 · Errors answer `{"detail": …}` as Encore's `encore_core::error` really does; the contracts said `{"error": …}`, which no Encore client reads, so the contracts were corrected.
- 2026-11-04 · The kit crates take the app's database as JSON rows and parameters over SQL strings (Encore's `encore_db` shape), so they keep Encore's SQL unchanged; apps without a database layer use `SqliteDb`.
- 2026-11-04 · Admin is the account's `role` being `admin` or `owner` (Encore's `perms::is_admin`); an app can override `Host::admin` to keep its own checks and words.
- 2026-11-04 · kit-server-changelog's notes parser and build-script embedding work without its server half (`default-features = false`), so an app's build.rs needs no Axum.
- 2026-11-04 · kit-server-settings has two features, both on by default: `toml` (read, check and generate settings.toml) and `server` (Store and routes); an app's build script takes only `toml`, so axum stays out of build scripts.
- 2026-11-04 · The table at run time (`file::parse`) leaks its strings to give the same `&'static` types as the generated code, so one renderer and one set of types serve both; a file is meant to be read once.
- 2026-11-04 · The accessor-name check takes the app's method names from the app (`Codegen::methods`) and keeps the Rust keywords in the kit; the refusal text stays Encore's ("taken by a Config method or a Rust keyword").
- 2026-11-04 · Encore's `Config` wraps the kit `Store` (`saved()`, then its one-time migration, then `apply()`), keeping its public API and `defs` field.
- 2026-11-04 · Encore-only save behaviour goes through `SettingsHost` hooks (`kept`, `unrevealed`, `before_save` returning `_warnings`, `after_save`, `log`, `owner`, `dump`); `before_save` runs before the activity-log line, as Encore's signup-mode check did.
- 2026-11-04 · `Store::load` creates the `settings` table if it is missing (for apps without migrations); Encore uses `Store::new` and its own reload, so it runs no new statement.
- 2026-11-04 · The Test button answers `{ok, message}` as Encore does, not `{ok, detail}` as the contract said; diagnostics.md and openapi.yaml are corrected.
- 2026-11-04 · A diagnostics check keeps its JSON object as the app built it, so Encore's numeric HTTP `status`, `body` and `ms` stay; a `status` that isn't ok/warn/fail counts as absent for the kit status.
- 2026-11-04 · Diagnostics groups all run at once (Encore ran only the HTTP checks together, then the folders and tools in turn); the answer and its order are the same.
- 2026-11-04 · On an app's own paths a notice is the app's object unchanged (no `id`/`severity`); only `/api/notices` adds them and gives non-admins `{ok}` alone.
- 2026-11-04 · Encore does not mount `/api/notices`; its storage notice has no kit timer, because Encore's worker already probes the download folders every minute.
- 2026-11-04 · App updaters take a full release only for tag `^v\d+\.\d+\.\d+$` (Android's rule; desktop accepted any 3-part semver), and use Android's newer-than rule on every platform.
- 2026-11-04 · The server-side release fetch and its sync_state cache (ETags, token, check_updates) stay in encore-system; the kit takes the parsing, channel, newer rule, shared answer and restart planning.
