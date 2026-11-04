# kit-server-bugreports

Bug reports and crash logs for any kit app. The contract is [contracts/bug-reports.md](../../contracts/bug-reports.md).

- **Reports** — a user files a report (by default kind `bug`, `not_bug`, `howto` or `crash`) with files: PNG,
  JPEG, WebP or GIF images (checked by their bytes and decoded, within a pixel budget) and UTF-8 `.txt` /
  `.log` files. The admin answers in a thread and sets the status (`new`, `looking`, `fixed`, `wontfix`,
  `answered`); both sides see what's unread.
- **Who sees what** — a non-admin sees only their own reports; anyone else's answers 404, not 403.
- **Files** — kept under `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever
  built from a database row.
- **Export** (admin) — a zip of `reports.md`, `reports.json` and `files/…`, or with `format=txt` one plain
  text file with the files named and text files (a crash log) written out in whole.
- **App crash reports** (`crash` module) — `POST/GET/DELETE /api/crash-reports`: an app uploads the crash
  reports it wrote, de-duplicated by `key`; admins see the newest 50.

The tables (`bug_reports`, `bug_report_messages`, `bug_report_files`, `client_crashes`) are created on first use.

## Configuration

Everything an app may change is in `BugReportsConfig` (every part has a `Default`):

| Part | What | Default |
| --- | --- | --- |
| `paths: BugReportPaths` | `base` of every route and file URL | `/api/bug-reports` |
| `limits: Limits` | per file, files per report, title, text, image pixels and side, reports and replies per hour, bytes per user, export size | 8 MiB, 6, 200, 20 000, 128 MiB, 8192 px, 20, 60, 500 MiB, 512 MiB |
| `words: Words` | kinds (ids and words), status words, the export's title and file name, the notification titles, the reply mail's subject and text, the report link | plain English ("{app} bug reports", "New bug report", …) |

`crash::CrashConfig` holds the crash route's path (`CrashPaths`, default `/api/crash-reports`), how many
reports are kept (50) and how long a text may be (256 KiB).

## Using it

The app implements `kit_server_core::Host` and `BugReportsHost` for its state, then merges the routers:

```rust
use kit_server_bugreports::{BugReportsConfig, BugReportsHost, Words};

impl BugReportsHost for App {
    fn data_dir(&self) -> std::path::PathBuf { self.data_dir.clone() }
    fn app_name(&self) -> String { "Demo".into() }
    fn app_version(&self) -> String { env!("CARGO_PKG_VERSION").into() }
    fn limited(&self, key: &str, limit: usize, window_secs: f64) -> bool { self.limiter.hit(key, limit, window_secs) }
    // optional: log, notify, mail_enabled, mail_address, base_url, send_mail, decodes
}

let words = Words { export_title: "{app} feedback".into(), ..Words::default() };
let app = axum::Router::new()
    .merge(kit_server_bugreports::router_with(BugReportsConfig { words, ..BugReportsConfig::default() }))
    .merge(kit_server_bugreports::crash::router())
    .with_state(state);
```

The optional methods are how the app tells people: `notify` (a chat webhook, a push message …) for a new report
and a user's reply, and `send_mail` for an admin's reply to the reporter (when `mail_enabled`; the link is
`base_url` followed by `Words::report_link`). `decodes` limits how many uploads are decoded at once (two by
default).
