# theme

`KitTheme(theme = KitTheme.Encore) { App() }` gives an app Encore's look: the palettes of the four themes
(Encore, Dark, OLED black, Light, plus System), Material 3 colours, type and shapes, and the focus ring that
appears when a D-pad or keyboard moves the focus (TV remotes).

- `KitTokens.kt` is generated from `design/tokens.json` (`design/out/`), not kept here.
- `KitTheme.kt`: the theme ids, `KitLook` (theme, accent, custom colour, background), `kitPaletteOf`, the
  colour maths of Settings → Appearance (`AppearanceRules`), `kitColorScheme`, `MaterialTheme.palette`.
- `FocusRing.kt`: the focus ring indication and modifier.

From Encore's `ui/theme/Theme.kt`, `Type.kt` and `FocusRing.kt`.
