# Krifmaz Kit

The neutral shared base for every Krifmaz app: one look, the same ease-of-use features (What's new, a
self-updating web app, notice banners, smart toasts, copyable errors, confirm dialogs, bug reports,
diagnostics, settings from a schema, offline mode, drag and drop, a command palette on the web) and the same fast, safe Rust and Kotlin
building blocks — built once, configured per app.

| Piece | Where |
| --- | --- |
| Design tokens → CSS, Compose, Iced, Flutter | [`design/`](design) |
| Shared API contracts | [`contracts/`](contracts) |
| Server crates (Axum) | [`crates/kit-server-*`](crates) |
| Client rules (no UI) | [`crates/kit-rules`](crates/kit-rules) |
| Web components (Leptos) | [`crates/kit-leptos`](crates/kit-leptos) |
| Desktop components (Iced) | [`crates/kit-iced`](crates/kit-iced) |
| Android library (Compose) | [`android/kit-compose`](android/kit-compose) |
| Shared CI and release workflows | [`.github/workflows`](.github/workflows) |
| New-app template | [`template/`](template) — `cargo generate krifmaz/kit template` |

Apps pin a tag (`tag = "v0.1.0"`). Design: [docs/DESIGN.md](docs/DESIGN.md). Goals: [docs/GOALS.md](docs/GOALS.md). MIT licensed.
