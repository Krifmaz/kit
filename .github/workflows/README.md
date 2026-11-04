# Shared workflows

Encore's CI and release pipeline as reusable workflows (DESIGN.md §13). An app calls them in a few lines and ships
its own Docker image, APK and desktop installers, all attached to its own GitHub release. Pin them like the crates:
`uses: Krifmaz/kit/.github/workflows/kit-release.yml@v0.1.0` (or a commit SHA before a tag exists).

| Workflow | Does | Main inputs |
| --- | --- | --- |
| `kit-ci.yml` | fmt (optional), clippy `-D warnings` and tests of each Rust workspace; the web app's wasm32 check, clippy and tests (strings/parity); the app's extra checks. Job `test`. | `rust-dirs`, `wasm-dirs`, `fmt`, `check`, `advisory` |
| `kit-release.yml` | `stage: prepare` picks the next `vX.Y.Z` (PATCH by default, MINOR/MAJOR from `docs/releases/<version>.md`, or by hand) and builds the web app and server tarballs; `stage: publish` writes the notes, attaches every file the run built (or the previous release's apps when none were built), writes `SHA256SUMS` and publishes a normal release (never a pre-release) marked Latest. | `app-name`, `bump`, `version`, `web-dir`, `server-dir`, `server-bin`, `only`, `apps-result`, `files`, `notes`, `app-files`, `carry-over` |
| `kit-docker.yml` | Multi-arch (amd64, arm64) image on GHCR, `:<version>` and `:latest`; `push: false` only builds it (pull requests). | `image`, `version`, `push`, `build-args` |
| `kit-android.yml` | Signed release APK (the caller passes the signing secrets), unit tests, signature, native libraries, 16 KB pages, optional Baseline Profile check; artifact `<app>-android`. | `app-name`, `version`, `code`, `project-dir`, `version-property`, `ndk`, `native-libs`, `baseline-profile` |
| `kit-desktop.yml` | Windows, Linux and macOS installers from the app's package script, checked against the names its updater looks for; artifact `<app>-desktop-<OS>`. | `app-name`, `version`, `systems`, `project-dir`, `package-script`, `*-features`, `*-files` |

`ci.yml` and `release.yml` are the kit's own: CI runs through `kit-ci.yml`, and every push to main makes a kit
release (none before `docs/releases/0.1.0.md` exists). The kit ships nothing but those tags.

## A release with apps

One run calls `kit-release` twice around the other three, so the version is chosen once and the release waits
for every file. Encore's `.github/workflows/release.yml` is the full example; the shape is:

```yaml
jobs:
  test:
    uses: Krifmaz/kit/.github/workflows/kit-ci.yml@v0.1.0
    with: { rust-dirs: server, wasm-dirs: web }
  prepare:
    needs: test
    uses: Krifmaz/kit/.github/workflows/kit-release.yml@v0.1.0
    with: { stage: prepare, app-name: Notes, web-dir: web, server-dir: server, server-bin: notes }
  docker:
    needs: prepare
    uses: Krifmaz/kit/.github/workflows/kit-docker.yml@v0.1.0
    with: { image: notes, version: "${{ needs.prepare.outputs.version }}" }
  android:
    needs: prepare
    if: inputs.apps
    uses: Krifmaz/kit/.github/workflows/kit-android.yml@v0.1.0
    with: { app-name: Notes, version: "${{ needs.prepare.outputs.version }}", code: "${{ needs.prepare.outputs.code }}" }
    secrets:
      android-keystore-base64: ${{ secrets.ANDROID_KEYSTORE_BASE64 }}
      android-keystore-password: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
      android-key-alias: ${{ secrets.ANDROID_KEY_ALIAS }}
      android-key-password: ${{ secrets.ANDROID_KEY_PASSWORD }}
  publish:
    needs: [prepare, docker, android]
    if: always() && needs.prepare.result == 'success' && needs.docker.result == 'success'
    uses: Krifmaz/kit/.github/workflows/kit-release.yml@v0.1.0
    with:
      stage: publish
      app-name: Notes
      version: ${{ needs.prepare.outputs.version }}
      apps-result: ${{ needs.android.result }}
      app-files: "Notes-android-*.apk"
      carry-over: "Notes-*"
```

Permissions: the caller grants `contents: write` (the release) and `packages: write` (the image). Secrets are passed
by name, never with `secrets: inherit`, which only works inside one organisation.
