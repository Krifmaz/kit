# Goals

The owner's goals for the kit: the block every app project starts with, and the kit's own goal. Each app's
goals live in that app's own repo.

## Shared app block (every app project)
Build on Krifmaz/kit (pin the latest kit tag; read its docs/DESIGN.md and CLAUDE.md first). Use the kit for the look (Encore theme), What's new and changelog, self-updating web app, notice banners, smart toasts with Copy, confirm dialogs, bug reports and crash logs, diagnostics, settings from a schema, offline mode and the shared release workflows. Don't rebuild what the kit has; if something is missing or generic, add it to the kit instead of the app.
Work hands-off: make your own calls (log non-obvious ones in docs/DECISIONS.md), one PR per change, fix CI until green, squash-merge yourself, delete the branch. Every merge ships a normal vX.Y.Z release (no pre-releases). Never skip tests or force-push main. Only stop to ask about deleting user data, a needed secret or paid service. One short summary per finished goal.

## Kit
Keep Krifmaz/kit the neutral shared base for all my apps and future apps. Follow docs/DESIGN.md and CLAUDE.md.
- Nothing app-specific: no app names, domain words, routes, text or assets. Everything is configured per app with neutral defaults. The "encore" theme is only the default look.
- When an app needs a feature that is generic, add it here (server crate, kit-rules, kit-leptos, kit-compose, kit-iced, shared workflows, template) with tests, docs and a demo in the template; never in a way that breaks apps on older tags.
- Keep the template's demo app running every kit feature on web, Android and desktop; CI proves it on every PR, plus the Postgres test run.
- Keep dependencies current (Leptos, Iced, Compose, Axum) in deliberate, tested upgrades; note breaking changes in the release notes with how to update.
- Breaking change = MAJOR, new feature = MINOR, fix = PATCH.
- Same hands-off rules as the shared block (only stop for a needed secret or paid service).
