# {{app_title}} — rules for every session

{{app_title}} is built on [Krifmaz/kit](https://github.com/Krifmaz/kit) ({{kit_tag}}): its server crates, kit-leptos
and the Encore theme. Read the kit's `CHANGELOG.md` before bumping the kit tag.

- **No shell for user input.** Never pass anything a person typed, uploaded or named to a shell. Run programs
  with an argument list (`std::process::Command::new(..).arg(..)`), never `sh -c`.
- **Full doc comments.** Every module, type, function and field says what it is for, in plain words.
- **Plain wording.** Say what happened and what to do next, in short sentences a person who is not a
  programmer understands. No codes, no jargon, no blame.
- **Copyable errors.** Every error is shown in full and can be copied (the kit's toasts and details dialog do
  this); errors answer `{"detail": "<plain sentence>"}` with the right HTTP status (kit-server-core's `err`).
- **Encore's look.** Use the kit's theme (`data-theme="encore"`) and components; no colours, fonts or sizes of
  your own outside the generated tokens.
- **Small validated changes.** One change at a time, each with its test. `cargo fmt`, `cargo clippy --workspace
  --all-targets -- -D warnings` and `cargo test --workspace` pass before every commit{% if android %}; ktlint and
  `./gradlew test` for `android/`{% endif %}. Every change gets a line in `CHANGELOG.md`.
- **Safety.** The container is distroless and runs as a non-root user with a read-only root filesystem; write
  only to the data folder (`APP_DATA_DIR`, `/data` in the image).
- **Releases.** Every merge to main is a normal release `vMAJOR.MINOR.PATCH` (never a pre-release). PATCH by
  default; add `docs/releases/<version>.md` in the PR for MINOR or MAJOR (it is also the release notes and
  What's new).
