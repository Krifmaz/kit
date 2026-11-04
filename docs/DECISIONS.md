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
- 2026-11-04 · kit-release.yml is called twice per run (`stage: prepare` → version, web app, server; `stage: publish` → notes, SHA256SUMS, release) instead of nesting kit-docker/-android/-desktop inside it: a `./` call inside a workflow used from another repo is not reliably resolved to the kit, and the version must be known before the image and apps build.
- 2026-11-04 · App-specific parts of Encore's release (install notes, browser extension, the compose file, old installer names) stay in Encore and reach kit-release as small inputs (`files`, `notes`, `carry-over`, `carry-over-drop`, `keep-drop`), so the kit holds no Encore wording.
- 2026-11-04 · Signing secrets are passed to kit-android/kit-desktop by name; `secrets: inherit` only works within one organisation and Encore and the kit belong to a user account.
- 2026-11-04 · kit-android's Baseline Profile check is an input (`baseline-profile`), off for Encore until its app has one (track C), so the switch changes nothing.
- 2026-11-04 · Every merge to Encore's default branch is now a normal vX.Y.Z release (DESIGN.md §19) instead of a hotfix-<v>.<run> build; the push trigger keeps Encore's docs-only and `[skip hotfix]` exceptions, skips the speed table like the hotfixes did, and the server's hotfix channel keeps working (it also counts full releases).
- 2026-11-04 · kit-rules toasts: web, desktop and Android differ (3/3/10 errors, others uncapped/5 in all/3, 4.2 s fade 0.4 s/4.2 s/4 s, a repeated error dropped vs moved to newest, blank skipped only on Android), so `ToastPolicy::{web,desktop,android}` keep each exactly.
- 2026-11-04 · kit-rules toasts expire at `at + life` (timer semantics, as the web and Android); the desktop's `elapsed > life` differs by one tick at most.
- 2026-11-04 · kit-rules changelog holds the notes parser (`parse`, `plain`, `short`, `unseen`) too, not only the grouping, so kit-server-changelog and every client read notes the same way; the app name is a parameter ("Encore" in Encore).
- 2026-11-04 · kit-rules has a `version` module (Encore's `encore_core::version` and `base`), with the `\d+(\.\d+)+` regex written out by hand so the crate needs no regex engine.
- 2026-11-04 · Words that name the app ("A new version of Encore is ready", "This server runs Encore …", the 502 reason) and keys/prefixes (`encore-web-`, `encore.update.tried`, `encore:version`, `encore-shell-`, `encore.offline.me`) take the app's name; Encore passes "Encore" and gets its exact strings.
- 2026-11-04 · The outbox's owner is a trait (`Outbox<i64>` = web `user_id`, `Outbox<String>` = Android `owner` `<server>|<username>`) so both documents keep their fields; Android's parse drops entries without a song, the web's does not.
- 2026-11-04 · `Outbox::parse` reads entry by entry, so one unreadable play no longer empties the whole document as the web's serde parse did (only affects corrupt files).
- 2026-11-04 · `plays_body` returns a serializable struct, not a `serde_json::Value`, so the field order (`track_id, seconds, played_at`) matches Android's body byte for byte without serde_json's `preserve_order`.
- 2026-11-04 · details: the web's blank-title rule (`trim().is_empty()`) is `details::title`, Android's empty-only rule is `details::title_android`; Encore has no "is it cut off" rule in code (CSS/Compose decide), so kit-rules has none.
- 2026-11-04 · Offline reasons are classified from a platform-neutral `Failure` enum (Dns, Timeout, Refused, Tls, Other) that each client maps its errors to, with Android's words.
- 2026-11-04 · Android-only window placement (`ToastRules.covered`, `lift`, `drop`) stays in kit-compose: it is Android window and inset logic, not shared rules.
- 2026-11-04 · kit-iced components hold no network code: they keep state, draw it and return what the app must request (Encore keeps reqwest, rfd and its API paths), so any app can use them.
- 2026-11-04 · kit-iced has one active look (`theme::set`/`look`, as Encore's desktop); Encore's `theme.rs` re-exports it, so Encore and the kit share one look.
- 2026-11-04 · App names in kit-iced words are parameters (`update::describe(p, "Encore")`, `whats_new::changelog(state, "Encore")`), giving Encore's exact words.
- 2026-11-04 · Encore desktop tests of code that moved into kit-iced move with it (ported, same assertions); tests of Encore's own code stay in Encore.
- 2026-11-04 · kit-iced `bug_report` keeps Encore desktop's checks (title and description required, 6 files, 8 MB each); the 200/20 000/120-character limits of contracts/bug-reports.md stay the server's, as in Encore.
- 2026-11-04 · kit-iced turns on iced's `image` feature: the bug report picker's thumbnails need `image::Handle::from_path`, as in Encore.
- 2026-11-04 · kit-iced form: table texts are `Cow<'static, str>` and option lists a static-or-owned enum, so Encore's const table and the server schema share one `Field`/`Card`; Encore's `Card { .. }` literal becomes `card! { .. }`.
- 2026-11-04 · kit-iced form: Encore's hard-coded "no Show saved for vpn_wireguard_config" becomes `Form::never_shown(keys)`, set by the app; a `key!=value` rule is `When { not: true }`; clearing a secret is its own `Msg::Cleared` (same toasts).
- 2026-11-04 · kit-iced diagnostics: a `status` of ok/warn/fail is the kit's level, any other `status` stays Encore's HTTP badge; warn draws in the warn colour; `fix` is a muted line under the detail (Encore never sends it).
