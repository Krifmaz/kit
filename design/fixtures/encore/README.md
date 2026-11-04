# Encore fixtures

Copies of Encore's theme sources at Krifmaz/Encore commit `5c0bdcba82307c522e4516d1bc51a81c8efa4026`, which the token tests compare the
generated themes against:

- `app.css`: the theme blocks of `web-rs/static/css/app.css` (lines 1–72: `:root`, `encore`, `data-brand`, `oled`, `light`).
- `Theme.kt`, `Type.kt`: `clients/android/app/src/main/java/app/encore/android/ui/theme/`.
- `desktop_theme.rs.txt`: `clients/desktop/src/theme.rs` (kept as text so it is not compiled).

Refresh them when Encore's theme changes (and update `tokens.json` to match).
