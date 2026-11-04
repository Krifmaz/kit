# Krifmaz/kit — rules for every session

The design is [docs/DESIGN.md](docs/DESIGN.md). [docs/Krifmaz-Kit.pdf](docs/Krifmaz-Kit.pdf) is reference
material only: where it talks about extracting from Encore or switching Encore over, DESIGN.md wins.

- **A neutral base library.** Not an app, not an app refactor. It ships nothing but version tags.
- **Nothing app-specific.** No Encore (or other app) names, domain concepts, routes, text or assets. The
  `encore` colours are only the default theme.
- **Configured per app.** App name, version, logo, routes, settings schema, changelog source, release targets
  are inputs with neutral defaults.
- **No other repos.** Do not open PRs in or change any other repository from kit work.
- **Proof** is the template's demo app running every kit feature on web, Android and desktop with tests passing.
- **Quality.** `cargo fmt`, `cargo clippy -D warnings`, tests (ktlint for Kotlin); full doc comments; a README
  per crate and module; a CHANGELOG.md entry for every change. No shell for user input; distroless non-root
  containers; plain words; copyable errors.
- **Decisions.** Decide yourself (DESIGN.md → the PDF → common sense); log non-obvious calls as one line in
  [docs/DECISIONS.md](docs/DECISIONS.md).
- **PRs.** One per track; fix CI and bot findings until green; squash-merge yourself; delete the branch; merge
  main in when needed.
- **Releases.** Every merge to main makes a normal `vMAJOR.MINOR.PATCH` release (never pre-release, rc, beta
  or edge). PATCH by default; add `docs/releases/<version>.md` for MINOR/MAJOR (also the notes). The first
  is v0.1.0, when DESIGN.md §16 passes.
- **Never** skip or disable tests; never force-push main.
- **Stop and ask only** for a needed secret, paid service or new account.
