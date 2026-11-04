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
