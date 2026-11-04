# design — Encore's look, defined once

- [`tokens.json`](tokens.json): the tokens. Edit this, never the outputs.
- [`gen/`](gen) (`kit-tokens`): reads the tokens and writes [`out/`](out):

| File | For | Use |
| --- | --- | --- |
| `out/tokens.css` | web (Leptos, Encore web) | import it and set `data-theme="encore"` on `<html>` |
| `out/KitTokens.kt` | Android (kit-compose) | `KitPalettes`, `KitAppearance`, `KitShapes`, `KitTypography` |
| `out/iced_tokens.rs` | desktop (kit-iced) | `include!`d: `THEMES`, `ACCENTS`, `PRESETS`, `surface(theme)` |
| `out/kit_tokens.dart` | Flutter (Photovault's Ente fork) | `kitThemeData(KitPalettes.encore)` |

Regenerate after a change: `cargo run -p kit-tokens` (CI runs `-- --check`). The tests
(`cargo test -p kit-tokens`) compare the outputs with Encore's theme sources in
[`fixtures/encore`](fixtures/encore): the CSS blocks of `app.css`, the palettes, accents, presets, shapes
and type of `Theme.kt`/`Type.kt`, and the desktop app's `theme.rs`. Where Android differs from the web today
(its palettes carry the logo accent; OLED text is cream), `tokens.json` → `compose.overrides` records it.
