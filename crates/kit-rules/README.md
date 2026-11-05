# kit-rules

The client-side rules every kit app shares, written once in plain Rust: no UI, no `web-sys`, no async runtime,
no clock. Time comes in as `now_ms: u64`, answers come in as data, and each function says what to do next. It
builds for `wasm32-unknown-unknown`, so kit-leptos (web), kit-iced (desktop) and, by porting, kit-compose
(Android) all share it. Depends only on `serde` and `serde_json`.

Nothing in it belongs to one app. The app's id, name and version are parameters ([`app::App`]); every word a
person reads is a `pub const` default, gathered per module in a `Words` struct (`ToastWords`, `DialogWords`,
`ChangelogWords`, `UpdateWords`, `CheckWords`, `OfflineWords`, `BugWords`, `DiagWords`, `PaletteWords`) with a `DEFAULT`
const. An app changes any word by passing its own struct (`ChangelogWords { intro: "…", ..ChangelogWords::DEFAULT }`).
Words that name the app or a number are templates (`A new version of {app} is ready`), filled by `app::fill`.
Where platforms differ on purpose, the difference is a policy with one preset per platform (`web()`,
`desktop()`, `android()`), never a silent pick.

```toml
kit-rules = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

## Modules

**`app`** — `App { id, name, version }`: the id prefixes every storage key (`app.key("update.tried")` →
`<id>.update.tried`), event and cache name; `id_of("Kit Demo")` → `kit-demo`; `fill(template, vars)`.

**`toasts`** — `ToastStack<A>` under a `ToastPolicy` (`web()`: 3 errors, others uncapped, 4.2 s with a
0.4 s fade, the same error again is dropped; `desktop()`: 3 errors, 5 toasts in all, 4.2 s; `android()`: 10
errors, 3 others, 4 s, any repeat moves to newest and restarts its time). Errors stay with Copy, except the
copy-failed words (`with_copy_failed` for an app's own); action toasts (Undo) keep their own time.
`add`/`show`/`show_action`, `close`, `expire(id, seq)`, `tick(now)`, `take_action`, `newest`, `next_deadline`;
`ToastWords`.

**`dialogs`** — the words every dialog shares (`DialogWords`: Cancel, Close, OK) and the confirm dialog:
`Confirm::new(title).text(…).ok("Delete").danger(true)` with its button labels and style, and `Answer` (the
first way it closes decides; anything but OK is no).

**`details`** — the "read the whole error" dialog: its title (`Details` when blank), Copy copies exactly the
text shown, a click that ends a selection does not open it, whether a copy took (`copied`), the Copy button's
words.

**`changelog`** — release notes (markdown) into `Block`s (`parse(app, version, md)`, `plain`, `short`),
blocks grouped into lists and tables for drawing (`group`), which releases are new for an account (`unseen`,
hotfixes count as their release), when the popup shows (`popup_on`, `show_popup`), and `ChangelogWords`.

**`version`** — versions compared by their numbers (`parts`, `compare`, `newer`) and a hotfix's release
(`base`).

**`update`** — reloading an open page into a new build: `Health` (the `/api/health` answer), `build_in(html,
stem)` (the build in the page's script name), `Watch` (this page's build against the server's), `Checker` (one
check at a time, every 3 s then 15 s while the server restarts, the 15 s focus throttle), `Gate` (reload now,
or the toast and a reload after 5 s with nothing holding the page; never a reload loop), `waiting_for(&[(held,
words)])` with the app's own reasons, `tried_key(app_id)`, `new_build`/`worker_wait`/`worker_ready` and `Flow`
(the service worker's side, its messages named by the app id); `UpdateWords`.

**`notices`** — `Notice` (`contracts/notices.md`), `parse` (the `{"notices": […]}` list or one notice),
which banners show (admins only, failing with a sentence), Check again's toast from three sentences per notice
(`CheckWords`, `CheckWords::DEFAULT` for a notice the app has none for), and `NoticePoll` (every minute, every
15 s while one fails).

**`offline`** — `reason`: why the server can't be reached (no network, DNS, refused, timeout, HTTPS, a proxy
answering 502–504); `mode`: when offline mode opens (`decide`), the 30 days of grace, a sign-out the server has
not confirmed (`at_boot`), the app's offline pages (`route_path`), retries, `counts_text` and `OfflineWords`;
`outbox`: `Outbox<A, O>`, the per-account outbox of the app's own actions (`A: Action` says each action's key,
subject and kind): kept once, the last state per subject wins, in order, capped, sent in chunks, forgotten once
sent.

**`uploads`** — the accept checks shared by the file picker and drag & drop (`accepts`, `choose`,
`refused_message`), the desktop's ending-only `Accept` and per-file `DropZone`, and `FileLimits` (the bug
report's 6 files, 8 MB each).

**`bugs`** — bug reports: the kinds and statuses with their words and badge classes, the New report sheet's
words (`BugWords`), its limits (200 / 20 000 characters), the line about what is sent along, the `device`
field and what Send says is missing.

**`diagnostics`** — the Diagnostics page's words (`DiagWords`), its groups of checks (the kit's three kinds
first, then the app's own kinds: `groups`, `groups_with`), a check's level and HTTP badge, the summary and
sources lines and the copied report (`report_text`).

**`palette`** — the command palette (Ctrl+K / ⌘K, `is_toggle`): `Item::new(group, label).hint(…).icon(…).dynamic(true)`;
`results(items, query)` gives the indexes that show, in order (no query: every fixed item, then the first
`DYNAMIC_SHOWN` (6) of each dynamic group; a query, trimmed and case-insensitive: label matches, then hint-only
matches); `step(sel, len, Move::Up/Down)` keeps the selection on the list. `ACTIONS` and `GO_TO` are the
usual group names; `PaletteWords` (title, placeholder, empty).

**`suite`** — linked apps (`contracts/suite.md`): the `Manifest` every app serves (sections, deep links,
search, sign-on; `problems()` says what keeps it from being linked), `sidebar(apps, local, prefs)` (every
section in the person's order, hidden ones marked), `visible`, `move_item`, `toggle_hidden`, `prefs_of` (back to
the `SidebarPrefs` kept under `suite_sidebar`), `fill` for deep-link patterns, `join` and `normalize_base` for
addresses, `SearchHit` / `SearchGroup` for one search across apps, and `SuiteWords`.

## Checks

```sh
cargo test -p kit-rules
cargo check -p kit-rules --target wasm32-unknown-unknown
```
