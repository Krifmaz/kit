# kit-icons

An app's icon set from one glyph, on the kit's tile in the theme's colours (DESIGN.md §5 "App icons").

```sh
cargo install --git https://github.com/Krifmaz/kit --tag v0.3.0 kit-icons
kit-icons web/icon.svg web/public/icons --name "My App"            # write the set
kit-icons web/icon.svg web/public/icons --name "My App" --check    # exit 1 when it is out of date
```

Options: `--short-name` (home screens; the name by default), `--start-url` (`/` by default), `--base` (where
the app serves the folder, for `head.html`; `/icons/` by default), `--theme`
(`encore` by default; any theme in `tokens.json`).

The glyph is an SVG (`viewBox="0 0 24 24"` by default) painted with `currentColor` and, for one detail,
`{accent-2}`. Out come `favicon.svg`, `icon-32.png`, `icon-192.png`, `icon-512.png`, `icon-maskable-512.png`,
`apple-touch-icon.png`, `manifest.webmanifest` and `head.html`: serve the folder at `/icons/` (or `--base`) and paste
`head.html` into `index.html`.

In Rust: `Colors::from_theme(&tokens, "encore")`, then `render(glyph, &colors, &App { .. })` gives the files in
memory.
