# kit-rules

The client-side decisions Encore writes once per client, written once in plain Rust: no UI, no `web-sys`, no
async runtime, no clock. Time comes in as `now_ms: u64`, answers come in as data, and each function says what to
do next. It builds for `wasm32-unknown-unknown`, so kit-leptos (web), kit-iced (desktop) and, by porting,
kit-compose (Android) all share it. Depends only on `serde` and `serde_json`.

Every rule, number and word comes from Encore's working code; each item's doc comment names the Encore file and
function it came from, and the tests are Encore's own tests, ported. Where Encore's clients differ, the
difference is a policy with named presets (`web()`, `desktop()`, `android()`), never a silent pick. The words
are `pub const`s (or small functions where Encore fills in a number or the app's name) for the kit-wide parity
test.

```toml
kit-rules = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

## Modules

**`toasts`** — `ToastStack<A>` under a `ToastPolicy` (`web()`: 3 errors, others uncapped, 4.2 s with a
0.4 s fade, the same error again is dropped; `desktop()`: 3 errors, 5 toasts in all, 4.2 s; `android()`: 10
errors, 3 others, 4 s, any repeat moves to newest and restarts its time). Errors stay with Copy, except
`COPY_FAILED`; action toasts (Undo) keep their own time. `add`/`show`/`show_action`, `close`, `expire(id, seq)`,
`tick(now)`, `take_action`, `newest`, `next_deadline`. From `web-rs/src/ui.rs`, the desktop's `app.rs` /
`widgets/toast.rs` and Android's `ToastRules.kt`.

**`details`** — the "read the whole error" dialog: its title (`Details` when blank), Copy copies exactly the
text shown, a click that ends a selection does not open it, whether a copy took (`copied`), the Copy button's
words. From `web-rs/src/details.rs`, the desktop's `widgets/details.rs` and `Details.kt`.

**`changelog`** — release notes (markdown) into `Block`s (`parse`, `plain`, `short`), blocks grouped into
lists and tables for drawing (`group`), which releases are new for an account (`unseen`, hotfixes count as
their release), when the popup shows (`popup_on`, `show_popup`), and What's new's words. From the server's
`routes/changelog.rs` and `web-rs/src/whats_new.rs`.

**`version`** — versions compared by their numbers (`parts`, `compare`, `newer`) and a hotfix's release
(`base`). From `encore_core::version`.

**`update`** — reloading an open page into a new build: `Health` (the `/api/health` answer), `Watch` (this
page's build against the server's), `Checker` (one check at a time, every 3 s then 15 s while the server
restarts, the 15 s focus throttle), `Gate` (reload now, or the toast and a reload after 5 s with nothing
holding the page; never a reload loop), `new_build`/`worker_wait`/`worker_ready` (waiting for the service
worker) and `Flow` (the service worker's side). From `web-rs/src/update.rs` and `web-rs/src/sw.rs`.

**`notices`** — `Notice` (`contracts/notices.md`), which banners show (admins only, failing with a sentence),
Check again's toast from three sentences per notice (`CheckWords::DOWNLOADS` is Encore's), and `NoticePoll`
(every minute, every 15 s while one fails). From `web-rs/src/notice.rs`.

**`offline`** — `reason`: why the server can't be reached (no network, DNS, refused, timeout, HTTPS, a proxy
answering 502–504), in Android's words; `mode`: when offline mode opens (`decide`), the 30 days of grace, a
sign-out the server has not confirmed (`at_boot`), retries and their words; `outbox`: the per-account plays and
likes to send later (`Outbox<i64>` is the web's document, `Outbox<String>` Android's). From
`web-rs/src/offline/` and Android's `features/offline/`.

**`uploads`** — the accept checks shared by the file picker and drag & drop (`accepts`, `choose`,
`refused_message`), the desktop's ending-only `Accept` and per-file `DropZone`, and the bug report's
`FileLimits` (6 files, 8 MB each). From `web-rs/src/dropzone.rs`, the desktop's `widgets/drop_zone.rs` and
`web-rs/src/pages/bugreports.rs`.

## Checks

```sh
cargo test -p kit-rules
cargo check -p kit-rules --target wasm32-unknown-unknown
```
