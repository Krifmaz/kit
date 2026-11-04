# kit-leptos

Encore's web components as [Leptos](https://leptos.dev) 0.8 components (client-side rendering, WebAssembly), on
top of [kit-rules](../kit-rules). Each one draws the HTML and CSS classes Encore's hand-written `web-rs` draws,
node for node, so Encore's stylesheet works unchanged and Encore looks the same when it switches. Every
decision comes from kit-rules and every word is a kit-rules constant.

```toml
kit-leptos = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

Leptos is pinned to `=0.8.23` (the latest stable) in the workspace; the wasm-bindgen family resolves to the
versions in Encore's `Cargo.lock` (wasm-bindgen 0.2.129, web-sys/js-sys 0.3.106, wasm-bindgen-futures 0.4.79).

## The shell

The components write into Encore's shell, which an app's `index.html` keeps:

```html
<div id="server-notice"></div>   <!-- notice::mount -->
<div id="toasts"></div>          <!-- toasts::mount -->
<div id="modal-root"></div>      <!-- modal::open, confirm_dialog, details::show, bug_report::open, whats_new -->
```

At start-up:

```rust
kit_leptos::api::set_backend(kit_leptos::api::HttpBackend::new("Encore")); // the default; a test can swap it
kit_leptos::toasts::mount();
kit_leptos::details::init();      // a click on [data-details] opens the dialog
kit_leptos::dropzone::init();     // drag and drop onto every [data-dropzone]
kit_leptos::notice::start(kit_leptos::notice::NoticeSource::encore_downloads(), is_admin);
kit_leptos::notice::mount();
kit_leptos::offline::watch_connection("Encore");
kit_leptos::update::start(kit_leptos::update::UpdateConfig::new("Encore"));
kit_leptos::whats_new::start(navigate);   // after signing in
```

## Components

| Module | Use it | From Encore |
| --- | --- | --- |
| `toasts` | `toast(msg, "ok"/"err"/"warn"/"")`, `show(msg, ToastKind)`, `toast_action(msg, "Undo", ms, f)`, `show_update(text, label, on_reload)`, `close`, `clear`; `<Toasts/>` or `mount()` into `#toasts` | `ui.rs`, `update.rs` |
| `clipboard` | `copy_text(t)`, `copy_then(t, toast)`, with the plain-http fallback | `ui.rs` |
| `modal` | `open(ModalOpts::new(title_html, body_html).footer(html).wide(true)) -> Modal` (`el`, `bg`, `qs`, `qsa`, `close`, `on_close`, at once); `ModalOpts::with_view` for Leptos bodies; `confirm_dialog(title, text, ok, danger).await`, `confirm_dialog_html` | `ui.rs` |
| `details` | `show(title, text)`, `attrs(title, text)` for `data-details`, `<CopyButton/>`, `init()` | `details.rs` |
| `whats_new` | `start(navigate)`, `popup(releases, navigate)`, `<ChangelogPage app/>`, `<ChangelogView app answer/>` | `whats_new.rs` |
| `update` | `start(UpdateConfig)`, `check_now()`, `register_worker(url)`, `apply_update()` | `update.rs`, `sw.rs` |
| `notice` | `start(NoticeSource, admin)`, `mount()`, `keep(answer)`, `<ServerNotice/>`, `<NoticeBanner text busy on_check/>` | `notice.rs` |
| `dropzone` | `init()`, `<DropZone input="id">…</DropZone>` | `dropzone.rs` |
| `schema_form` | `Schema::from_json`, `visible_cards`, `<SettingsCards cards values reveal focus hooks/>`, `<SettingsCard …/>`, `form_values` | `settings/schema_form.rs`, `fields.rs`, `groups.rs` |
| `bug_report` | `open(SheetOpts::new(app, page, version)) -> Modal` | `pages/bugreports.rs` |
| `diagnostics` | `<DiagnosticsPage app hooks/>`, `<DiagnosticsView app data/>`, `DiagHooks` for an app's own blocks | `settings/diagnostics.rs` |
| `offline` | `show_banner(grace_ended, on_retry)`, `set_trying`, `conn_banner(on, text)`, `watch_connection(app)` | `offline/mode.rs`, `app.rs` |
| `api` | `get`/`post`/`post_empty`/`put`/`del`/`post_form`, `Backend`, `HttpBackend` | `api.rs` |

Hand-written code keeps its calls: Encore's `ui::toast`, `ui::toast_action`, `ui::modal` and
`ui::confirm_dialog` become one-line wrappers over `toasts::toast`, `toasts::toast_action`, `modal::open`
(which mounts synchronously, so `m.qs("#br-send")` finds the element right after) and `modal::confirm_dialog`.

## Matching Encore's DOM

Leptos 0.8 differs from hand-written HTML in three ways, and the components handle each:

- `view!` puts `class` and `style` before the other attributes. An element whose first attribute is not
  `class` starts with an empty spread, `<button {..()} type="button" class="…">`, which keeps the written order.
- An element without children gets an empty `<!---->` placeholder, also after `inner_html`. HTML goes in as
  `{..html(s)}` with an empty text child `{""}` (`html::html`), which adds nothing to `outerHTML`.
- A `<For>` leaves an end marker in its parent. `#toasts` and `#modal-root`, which other code also writes into,
  get one separately mounted view per toast or dialog instead.

Inside parts the kit draws alone (a settings card, the Diagnostics page) conditional parts may still leave
placeholder comments; they change neither the elements nor the text.

## Tests

`cargo test -p kit-leptos` renders each component to HTML (the `ssr` feature, a dev-dependency only) and
compares it with Encore's own builders, copied verbatim into the tests, through a canonical DOM compare (tags,
attributes as sets, class words, text; whitespace between tags and render markers ignored). The settings form
is checked against Encore's fixtures (`src/schema_form/fixtures`, from Encore's `form_tests.rs`), all 45 pages
both ways. Server-side rendering always writes `class` last, so attribute order and exact bytes are checked in
the browser: the [demo](demo) compares the live `outerHTML` of the toasts, a dialog and the banners with
Encore's HTML.
