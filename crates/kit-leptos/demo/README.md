# kit-leptos demo

The kit's design system around six pages (`#/` Home: hero, carousels, stat cards; `#/browse`: filter chips, a grid
of covers, skeletons; `#/item/<n>`: an item's page; `#/stats`: stat cards and rings; `#/states`: empty states,
skeletons, a sheet; `#/kit`), in the kit's `AppShell` (tab bar on phones, rail on tablets, sidebar on desktops).
`node scripts/visual-qa.cjs http://127.0.0.1:8080/ out/` screenshots every page at three sizes and checks for
sideways overflow and overlapping text.

The Kit page has every kit-leptos feature, for a neutral app ("Kit Demo" 1.4.0, set with `config::set`), with fake
data: toasts (each kind, an error with Copy, the same error merged, Undo, the cap of three errors), the update
toast and the real self-update flow (Publish a new build, with a switch that holds the page), the confirm dialog
(`modal::confirm` and the `ConfirmDialog` component), the details dialog, What's new (popup and changelog page),
the notice banner with Check again, the offline and connection banners, the outbox (take the fake server away,
star things and add notes, bring it back and they go), a drop zone, the settings form drawn
from the kit's sample schema (`schema_form::SAMPLE_SCHEMA`), the bug report sheet, the Diagnostics page and the command palette (Ctrl+K, with fake sites loaded on opening). A
fake `api::Backend` answers every call on the kit's default paths, so no server is needed.

```sh
cd crates/kit-leptos/demo
trunk serve            # http://127.0.0.1:8080
trunk build --release  # dist/
```

The page loads the kit's tokens (`design/out/tokens.css`) and the kit's stylesheet (`../style/kit.css`). The
Theme menu in the head switches `<html data-theme>` between the four themes: `encore`, `dark`, `oled` and
`light`.

Once drawn, the page checks its own DOM: it shows toasts, a confirm dialog and the banners through the kit's API, keeps a star in the outbox while
the fake server is away and sends it once it is back,
and compares each element with the kit's expected markup, parsed by the same browser (comments and
whitespace-only text left out, attributes in any order). The result is in `<pre id="kit-check">` and in
`<body data-kit-check="pass|fail">`, which a Playwright script can read before it takes a screenshot.

CI builds the demo with `cargo build -p kit-leptos-demo --target wasm32-unknown-unknown`.
