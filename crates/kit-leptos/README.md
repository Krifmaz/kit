# kit-leptos

The kit's web components as [Leptos](https://leptos.dev) 0.8 components (client-side rendering, WebAssembly), on
top of [kit-rules](../kit-rules). Every decision comes from kit-rules; every word is a kit-rules default an app
can change; every server path, route, the app's id, name, version and logo are configured per app
(`config::AppConfig`). The components draw the class names of the kit's own stylesheet,
[`style/kit.css`](style/kit.css), which is built only on the design tokens (`design/out/tokens.css`), so every
theme (`encore`, `dark`, `oled`, `light`) draws every component.

```toml
kit-leptos = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

Leptos is pinned to `=0.8.23` in the workspace.

## Stylesheet

Load the tokens first, then the kit's stylesheet. With trunk:

```html
<link data-trunk rel="css" href="path/to/design/out/tokens.css">
<link data-trunk rel="css" href="path/to/kit-leptos/style/kit.css">
```

The same file is `kit_leptos::CSS` (a `&str`), for an app that writes it out itself. `<html data-theme="…">`
picks the theme.

## The shell

The components write into these elements, which an app's `index.html` keeps:

```html
<div id="server-notice"></div>   <!-- notice::mount -->
<div id="toasts"></div>          <!-- toasts::mount -->
<div id="modal-root"></div>      <!-- modal::open, modal::confirm, details::show, bug_report::open, whats_new -->
```

At start-up:

```rust
use kit_leptos::config::{self, AppConfig};
use kit_rules::app::App;

let mut cfg = AppConfig::new(App::named("Kit Demo").with_version("1.4.0")); // id `kit-demo`
cfg.logo = Some("/logo.svg".into());
cfg.paths.diagnostics = "/checks".into();   // any path the app's server uses
cfg.words.update.ready = "{app} has a new version";     // any word
config::set(cfg);

kit_leptos::toasts::mount();
kit_leptos::crash::install();    // crash logs to /api/crash-reports
kit_leptos::details::init();      // a click on [data-details] opens the dialog
kit_leptos::dropzone::init();     // drag and drop onto every [data-dropzone]
kit_leptos::notice::start(kit_leptos::notice::NoticeSource::default(), is_admin);
kit_leptos::notice::mount();
kit_leptos::offline::watch_connection();
kit_leptos::update::start(kit_leptos::update::UpdateConfig::from_config().hold("It reloads when the upload is done", uploading));
kit_leptos::whats_new::start(navigate);   // after signing in
kit_leptos::palette::install(kit_leptos::palette::Palette::new(commands)); // Ctrl+K / ⌘K
```

## Configuration

`AppConfig` holds:

- `app`: `App { id, name, version }`. The id prefixes storage keys (`<id>.update.tried`), the API events
  (`<id>:server-down`, `<id>:server-up`, `<id>:unauthorized`, `<id>:bugs-changed`), the `X-Requested-With`
  header and the service worker's names. `<id>:server-down` means the server gave no answer of its own (the call
  failed, or a 404/5xx came without the server's `x-<id>` header), after 2 failed calls in a row or 5 s with no
  call answered; a cancelled call counts for nothing.
- `logo`, and `script`: the stem of the page's script name (`<id>-web` by default, whose hash is the build).
- `paths`: every server path, relative to `api` (`/api`): `health` `/health`, `changelog` `/changelog`,
  `changelog_new` `/changelog/new`, `changelog_seen` `/changelog/seen`, `popup` `/auth/prefs` with
  `popup_field` `whats_new_popup`, `notices` `/notices`, `notices_check` `/notices/check`, `bug_reports`
  `/bug-reports`, `diagnostics` `/diagnostics`, `test` `/diagnostics/{name}/test`, `settings` `/settings`,
  `settings_schema` `/settings/schema`, `settings_secret` `/settings/secret/{key}`; and `worker` `/sw.js`.
- `routes`: the app's pages the components link to (`changelog`: where Full changelog goes).
- `words`: one struct per component (`ToastWords`, `DialogWords`, `ChangelogWords`, `UpdateWords`,
  `OfflineWords`, `BugWords`, `DiagWords`, `CheckWords`, `FormWords`, `PaletteWords`), each with the kit's `DEFAULT`.
- `toast_policy`: how toasts stack and go (`ToastPolicy::web()` by default).

## Components

| Module | Use it |
| --- | --- |
| `config` | `AppConfig`, `Paths`, `Routes`, `Words`; `set`, `get`, `app_name` |
| `toasts` | `toast(msg, "ok"/"err"/"warn"/"info"/"")`, `show(msg, ToastKind)`, `toast_action(msg, "Undo", ms, f)`, `show_update(text, label, on_reload)`, `close`, `clear`; `<Toasts/>` or `mount()` into `#toasts` |
| `clipboard` | `copy_text(t)`, `copy_then(t, toast)`, with the plain-http fallback |
| `modal` | `open(ModalOpts::new(title_html, body_html).footer(html).wide(true)) -> Modal`; `close_all()`; `confirm(Confirm::new(title).text(…).ok("Delete").danger(true)).await -> bool`; `<ConfirmDialog confirm on_answer/>`; `confirm_dialog(title, text, ok, danger)` |
| `details` | `show(title, text)`, `attrs(title, text)` for `data-details`, `<CopyButton/>`, `init()` |
| `whats_new` | `start(navigate)`, `popup(releases, navigate)`, `<ChangelogPage/>`, `<ChangelogView answer/>` (`app` optional) |
| `update` | `start(UpdateConfig)` (`from_config`, `hold(words, held)`), `check_now()`, `register_worker(url)`, `apply_update()` |
| `notice` | `start(NoticeSource, admin)` (`default()`, `words_for(id, words)`), `mount()`, `keep(answer)`, `<ServerNotice/>`, `<NoticeBanner text busy on_check id severity/>` |
| `dropzone` | `init()`, `<DropZone input="id">…</DropZone>` |
| `schema_form` | `Schema::from_json`, `visible_cards`, `<SettingsCards cards values reveal focus hooks/>`, `<SettingsCard …/>`, `<SchemaFields/>`, `form_values`, `FormWords`, `SAMPLE_SCHEMA` |
| `bug_report` | `open(SheetOpts::from_config(page))` or `SheetOpts::new(app, page, version)` |
| `diagnostics` | `<DiagnosticsPage hooks/>`, `<DiagnosticsView data/>` (`app` optional), `DiagHooks` for an app's own blocks |
| `signup` | `<SignInLine sign_up_href/>` (under the sign-in form: "Create an account", the invite hint, or where accounts are managed), `<SignUpPage app invite sign_in_href/>`, `<VerifyPage app token sign_in_href/>` and `<SignupAdmin/>` (who can sign up, waiting sign-ups, invites with the link shown once) on `paths.signup` |
| `crash` | `install()` catches panics, uncaught errors and unhandled rejections and posts them to `paths.crash_reports` (`/crash-reports`), keeping them in `localStorage` until the server takes them; `report(kind, summary, text)`, `flush()` |
| `offline` | `show_banner(grace_ended, on_retry)`, `set_trying`, `conn_banner(on, text)`, `watch_connection()`, `<OfflineBanner/>`, `<ConnBanner/>` |
| `palette` | `install(Palette::new(vec![Command::new(ACTIONS, "New note", f).hint("…").icon("plus"), …]).source(\|\| async { load().await }))`; `open()`, `close()`, `toggle()`, `is_open()`, `set_commands(…)`. Ctrl+K / ⌘K toggles, Escape or a press beside it closes; Up/Down, Enter, click. Loaded commands follow the fixed ones (six per group until something is typed). Classes `.palette-bg`, `.palette`, `.palette-search`, `.palette-input`, `.palette-list`, `.palette-group`, `.palette-item[aria-selected]`, `.palette-icon`, `.palette-label`, `.palette-hint`, `.palette-empty` in `kit.css` |
| `suite` | linked apps (`contracts/suite.md`): `<SuiteNav active/>` (the shared sidebar with Edit sidebar), `<SuiteSearch/>`, `<Related query/>` ("Related in …"), `<SsoButton next/>` ("Sign in with …"), `sso_error()`, `<LinkedApps/>` (the admin page: links and keys), `set_words(SuiteWords)` |
| `api` | `get`/`post`/`post_empty`/`put`/`patch`/`del`/`post_form`, `Backend`, `HttpBackend` (`from_config`, `new(id)`, `with_base`, `with_headers(|method, path| …)` for the app's own headers, such as a CSRF token) |

## Tests

`cargo test -p kit-leptos` renders each component to HTML (the `ssr` feature, a dev-dependency only) and
compares it with the kit's expected markup through a canonical DOM compare (tags, attributes as sets, class
words, text; whitespace between tags and render markers ignored), and checks that an app's words and paths
reach the markup. The settings form is tested on the kit's sample schema (`src/schema_form/sample-schema.json`).
In the browser, the [demo](demo) compares the live DOM of the toasts, the confirm dialog and the banners with
the same markup.
