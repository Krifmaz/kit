# Goals

The owner's goals for the kit and every app built on it, as written.

## Shared app block (every app project)
Build on Krifmaz/kit (pin the latest kit tag; read its docs/DESIGN.md and CLAUDE.md first). Use the kit for the look (Encore theme), What's new and changelog, self-updating web app, notice banners, smart toasts with Copy, confirm dialogs, bug reports and crash logs, diagnostics, settings from a schema, offline mode and the shared release workflows. Don't rebuild what the kit has; if something is missing or generic, add it to the kit instead of the app.
Work hands-off: make your own calls (log non-obvious ones in docs/DECISIONS.md), one PR per change, fix CI until green, squash-merge yourself, delete the branch. Every merge ships a normal vX.Y.Z release (no pre-releases). Never skip tests or force-push main. Only stop to ask about deleting user data, a needed secret or paid service. One short summary per finished goal.

## Kit
Keep Krifmaz/kit the neutral shared base for all my apps (Encore, FSView, Filebrary, Photovault, Homedit, Caddymium and future apps). Follow docs/DESIGN.md and CLAUDE.md.
- Nothing app-specific: no app names, domain words, routes, text or assets. Everything is configured per app with neutral defaults. The "encore" theme is only the default look.
- When an app needs a feature that is generic, add it here (server crate, kit-rules, kit-leptos, kit-compose, kit-iced, shared workflows, template) with tests, docs and a demo in the template; never in a way that breaks apps on older tags.
- Keep the template's demo app running every kit feature on web, Android and desktop; CI proves it on every PR, plus the Postgres test run.
- Keep dependencies current (Leptos, Iced, Compose, Axum) in deliberate, tested upgrades; note breaking changes in the release notes with how to update.
- Breaking change = MAJOR, new feature = MINOR, fix = PATCH.
- Same hands-off rules as the shared block (only stop for a needed secret or paid service).

## Encore
Move Encore onto the neutral kit with no visible change. Replace Encore's own copies of kit features with the kit's crates, kit-leptos, kit-compose and kit-iced; move the remaining web pages to Leptos page by page; switch releases to the kit's shared workflows (server, Docker, APK, desktop). Prove each step with before/after screenshots and the existing tests.

## FSView
Adopt the kit. Add What's new and changelog, self-updating web app, smart toasts with Copy, notice banners (pool health, SMR, auto-repair events), bug reports, diagnostics and settings from a schema. Move the web UI from React to Leptos page by page. Switch releases to the kit's shared workflows. Keep every storage feature and safety check working exactly as now.

## Filebrary
Rebuild Filebrary on the kit template: user accounts and roles, Encore look, Leptos web UI, kit server crates. It must beat FileBrowser and its alternatives on every point: browsing, search, previews, sharing links, uploads with resume, permissions per source, mobile-friendly UI and speed.

## Photovault
Rust server (Ente-compatible) on the kit's server crates with Postgres; Android app on kit-compose. Add an upload progress notification run as a foreground service (count, progress bar, speed, Pause/Resume, Wi-Fi only, Retry on errors) and the same progress banner in the app. Strip all Ente branding, links and upsells; make choosing a self-hosted server the default first screen.

## Homedit
Adopt the kit: Encore look, What's new, toasts with Copy, bug reports, diagnostics, settings from a schema, shared release workflows. Network scan: accept a bare address like 10.0.1.0 and offer to scan its /24 instead of finding nothing.

## Caddymium
Adopt the kit, and fix updates. If any step of a Caddy update fails after Caddy was asked to stop, kill a hung process, start the previous binary, confirm the admin API and listeners answer, and notify. Add a watchdog that restarts Caddy (last good binary) when the admin API stops answering for 60s. Clear the "update failed" banner once a later update succeeds. New Security defaults must never switch on for existing sites through an update; offer them as suggestions. Tests for all of it.
