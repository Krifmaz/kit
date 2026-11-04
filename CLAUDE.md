# Krifmaz/kit — rules for every session

The full spec is [docs/DESIGN.md](docs/DESIGN.md) (from [docs/Krifmaz-Kit.pdf](docs/Krifmaz-Kit.pdf)). Read it first.

- **Extract, don't invent.** Every feature, behaviour and word comes from Krifmaz/Encore's working code.
- **Encore first.** Prove each piece by switching Encore to it with no visible change for its users.
- **Same words everywhere.** The kit-wide parity test must pass; never reword a string Encore shows.
- **Quality bar.** `cargo fmt`, `cargo clippy -D warnings` and tests clean (ktlint for Kotlin); full doc
  comments; a README per crate and module; a CHANGELOG.md entry for every change.
- **Safety.** No shell for user input. Distroless, non-root containers. Plain words, copyable errors.
- **Decisions.** Decide yourself (DESIGN.md → Encore's behaviour → common sense) and log each non-obvious call
  as one line in [docs/DECISIONS.md](docs/DECISIONS.md).
- **PRs.** One PR per track; fix CI and bot findings until green; squash-merge yourself; delete the branch.
  Merge order: tokens & contracts → rules & server → leptos, compose, iced → template.
- **Releases.** Every merge to main makes a normal GitHub release `vMAJOR.MINOR.PATCH` (never pre-release,
  rc, beta or edge). PATCH by default; add `docs/releases/<version>.md` in the PR for MINOR/MAJOR (it is
  also the release notes). The first release is v0.1.0, once DESIGN.md §18 passes. The kit ships only tags.
- **Never** skip, disable or delete a test to get green; never force-push main.
- **Stop and ask only** for: deleting user data, a needed secret/paid service/new account, or a visible
  Encore change DESIGN.md doesn't call for.
