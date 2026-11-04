# kit-leptos demo

Every kit-leptos component on one page with fake data: toasts, the update toast, the confirm dialog, the
details dialog, What's new (popup and page), the notice banner, the offline and connection banners, a drop zone,
Encore's General settings page drawn from Encore's schema fixture, the bug report sheet and the Diagnostics page.
A fake `api::Backend` answers every call, so no server is needed.

```sh
cd crates/kit-leptos/demo
trunk serve            # http://127.0.0.1:8080
trunk build --release  # dist/
```

The page loads the kit's tokens (`design/out/tokens.css`) with `data-theme="encore"`, then `encore-app.css`: a
copy of Encore's `web-rs/static/css/app.css` after its colour tokens, with a SOURCE note naming the commit. The
kit ships no stylesheet of its own yet; the copy is for the demo only.

Once drawn, the page checks its own DOM: it shows toasts, a dialog and the banners through the kit's API and
compares each element's `outerHTML` byte for byte with Encore's HTML parsed by the same browser. The result is
in `<pre id="kit-check">` and in `<body data-kit-check="pass|fail">`, which a Playwright script can read before
it takes a screenshot.

CI builds the demo with `cargo build -p kit-leptos-demo --target wasm32-unknown-unknown`.
