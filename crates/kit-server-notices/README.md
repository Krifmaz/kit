# kit-server-notices

Plain-language banners for a kit server (contract: [`contracts/notices.md`](../../contracts/notices.md)):
the server tells signed-in admins, in one plain sentence, that something is wrong and what to check, with a
Check again action. The notices, their sentences and their toasts are the app's; the kit keeps, re-checks and
serves them.

- **`Notice`** — the object a check makes: `{ok, text?, checking?, …}` plus fields of the app's own, kept
  as it is (`Notice::failing("…")`, `Notice::fine()`, `Notice::from(value)`). `shows()`: not ok and a
  sentence to show. `plain()`: only `{ok}`, what anyone but an admin is told.
- **`NoticeCheck`** — one registered notice: id, `Severity` (`info`/`warn`/`err`), how it reads now
  (`current`, cheap, run on the blocking pool) and how it is checked again (`check`).
  `NoticeCheck::kept(id, severity, probe)` makes one from a single probe and keeps its last result; Check
  again waits for a probe at most `CHECK_WAIT` (15 s) and then answers with `checking`. `.every(d)` checks
  it again on a timer (`spawn_rechecks(state)`), `.toasts(Toasts::new(failing, checking, fixed))` keeps
  the app's three Check again sentences (`Toasts::after(&notice)` picks one).
- **`Notices`** / **`NoticesHost`** — the app's list and the trait that hands it to the kit.
  `Notices::current` / `Notices::check` give one notice's own object, for an app that shows it inside an
  answer of its own.
- **`router()`** — `GET /api/notices` (signed in) → `{"notices": [{id, severity, ok, text, checking, …}]}`
  and `POST /api/notices/check` (admins), plus one notice at `GET /api/notices/{id}` and
  `POST /api/notices/{id}/check`; **`router_with(NoticePaths { list, check, one, check_one })`** serves them
  at the app's own paths.

```rust
impl kit_server_notices::NoticesHost for App {
    fn notices(&self) -> Notices<App> { self.notices.clone() } // kept in the state, for kept notices
}
let notices = Notices::new().register(
    NoticeCheck::kept("disk", Severity::Warn, |app: App| async move { probe_disk(&app).await })
        .every(Duration::from_secs(60))
        .toasts(Toasts::new("The disk is still full", "Still checking the disk", "The disk has room again")),
);
let router = Router::new().merge(kit_server_notices::router::<App>());
kit_server_notices::spawn_rechecks(state.clone());
```
