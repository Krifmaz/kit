# kit-server-bugreports

Bug reports and crash logs for any kit app, extracted from Encore's `routes/bugreports.rs` and
`routes/client_crash.rs`. The contract is [contracts/bug-reports.md](../../contracts/bug-reports.md).

- **Reports** — a user files a report (kind `bug`, `not_bug`, `howto` or `crash`) with up to 6 files: PNG,
  JPEG, WebP or GIF images (checked by their bytes and decoded, at most 128 MiB and 8192 px a side) and
  UTF-8 `.txt` / `.log` files, 8 MiB each. The admin answers in a thread and sets the status (`new`,
  `looking`, `fixed`, `wontfix`, `answered`); both sides see what's unread.
- **Who sees what** — a non-admin sees only their own reports; anyone else's answers 404, not 403.
- **Files** — kept under `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever
  built from a database row.
- **Export** (admin) — a zip of `reports.md`, `reports.json` and `files/…`, or with `format=txt` one plain
  text file with the files named and text files (a crash log) written out in whole. At most 512 MiB.
- **Limits** — title 200 characters, text 20 000, file name 120; 20 reports and 60 replies per user per
  hour; 500 MiB of files per user (admins exempt).
- **App crash reports** (`crash` module) — `POST/GET/DELETE /api/diagnostics/client-crash`: a phone uploads
  the crash reports it wrote, de-duplicated by `key`; admins see the newest 50.

The tables (`bug_reports`, `bug_report_messages`, `bug_report_files`, `client_crashes`) are Encore's and are
created on first use.

## Using it

The app implements `kit_server_core::Host` and `BugReportsHost` for its state, then merges the routers:

```rust
use kit_server_bugreports::BugReportsHost;

impl BugReportsHost for App {
    fn data_dir(&self) -> std::path::PathBuf { self.data_dir.clone() }
    fn app_name(&self) -> String { "Encore".into() }
    fn app_version(&self) -> String { env!("CARGO_PKG_VERSION").into() }
    fn limited(&self, key: &str, limit: usize, window_secs: f64) -> bool { self.limiter.hit(key, limit, window_secs) }
    // optional: log, notify, mail_enabled, mail_address, base_url, send_mail, enrich, decodes
}

let app = axum::Router::new()
    .merge(kit_server_bugreports::router())
    .merge(kit_server_bugreports::crash::router())
    .with_state(state);
```

The optional methods are how the app tells people: `notify` (Encore: its Discord webhook) for a new report
and a user's reply, and `send_mail` for an admin's reply to the reporter (when `mail_enabled`). `enrich`
lets the app add to the list and detail answers (Encore translates titles); `decodes` limits how many
uploads are decoded at once (two by default).
