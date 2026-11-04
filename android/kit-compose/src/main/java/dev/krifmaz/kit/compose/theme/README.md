# theme

`KitTheme(theme = KitTheme.Encore) { App() }` gives an app the kit's look: the palettes of the four themes
(`KitPalettes.Encore` — charcoal and cream, the default — `Dark`, `Oled` and `Light`, plus System), Material 3
colours, type and shapes, and the focus ring that appears when a D-pad or keyboard moves the focus (TV remotes).

- `KitTokens.kt` is generated from `design/tokens.json` (`design/out/`), not kept here.
- `KitTheme.kt`: the theme ids, `KitLook` (theme, accent — `""` for the theme's own — custom colour,
  background), `kitPaletteOf`, the colour maths of Settings → Appearance (`AppearanceRules`), `kitColorScheme`,
  `MaterialTheme.palette`.
- `FocusRing.kt`: the focus ring indication and modifier.
