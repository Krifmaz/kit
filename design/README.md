# design — the kit's look, defined once

- [`tokens.json`](tokens.json): the tokens. Edit this, never the outputs.
- [`gen/`](gen) (`kit-tokens`): reads the tokens and writes [`out/`](out):

| File | For | Use |
| --- | --- | --- |
| `out/tokens.css` | web (Leptos) | import it; `:root` is the default `encore` theme, `data-theme="dark"`, `"oled"` or `"light"` on `<html>` picks another |
| `out/KitTokens.kt` | Android (kit-compose) | `KitPalettes.Encore/Dark/Oled/Light`, `KitAppearance`, `KitShapes`, `KitTypography` |
| `out/iced_tokens.rs` | desktop (kit-iced) | `include!`d: `THEMES`, `ACCENTS`, `PRESETS`, `surface(theme)`, `accent(theme)` |
| `out/kit_tokens.dart` | Flutter | `kitThemeData(KitPalettes.encore)` |

Themes: `encore` (the default, charcoal and cream), `dark`, `oled`, `light`; each has its own surface and
accent colours. Regenerate after a change with `cargo run -p kit-tokens` (CI runs `-- --check`); the tests
(`cargo test -p kit-tokens`) check every output against the tokens and that `out/` is up to date.
