# Bug reports and crash logs

From Encore's `server/crates/api/src/routes/bugreports.rs` and `client_crash.rs`. A user writes what went
wrong, attaches screenshots or a log, and the admin answers in a thread; the admin can export the lot.

## Reports

| Request | Answer |
| --- | --- |
| `POST /api/bug-reports` multipart `kind`, `title`, `body`, `page`, `app_version`, `device`, `file`… | `{ok, id}` |
| `GET /api/bug-reports?status=&kind=&q=&mine=1` | `{reports: [{id, kind, title, status, username, user_id, created_at, updated_at, message_count, file_count, unread}]}` |
| `GET /api/bug-reports/unread` | `{count}` |
| `GET /api/bug-reports/export?status=&kind=&ids=` (admin) | `application/zip` (`reports.md`, `reports.json`, `files/…`); with `&format=txt`: `text/plain`, the same reports as one plain text file (files listed by name) |
| `GET /api/bug-reports/{id}` | `{report: {…, files: […]}, messages: [{…, files: […]}]}` |
| `POST /api/bug-reports/{id}/messages` multipart `body`, `file`… | `{ok, id}` |
| `POST /api/bug-reports/{id}/status` `{status}` (admin) | `{ok}` |
| `DELETE /api/bug-reports/{id}` (admin) | `{ok}` |
| `GET /api/bug-reports/{id}/files/{fid}` | the file |

A file: `{id, name, mime, size, url}` (`url` is `/api/bug-reports/{report}/files/{id}`).

- `kind`: `bug`, `not_bug`, `howto`, `crash` (a crash log the phone sends; the Crash logs section).
- `status`: `new`, `looking`, `fixed`, `wontfix`, `answered`.
- A non-admin only sees their own reports; someone else's answers 404, not 403.
- Files live under `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever built
  from a database row.

Limits: 8 MiB per file, 6 files per report or message, title 200 characters, text 20 000, file name 120;
an image may decode to at most 128 MiB and 8192 px a side; 20 reports and 60 replies per user per hour;
500 MiB of files per user (admins exempt); an export holds at most 512 MiB.

## App crash reports

| Request | Answer |
| --- | --- |
| `POST /api/diagnostics/client-crash` `{kind, device, device_id, app_version, os, time, summary, text, key}` | `{ok, id, duplicate}` |
| `GET /api/diagnostics/client-crash` (admin) | `{crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}` (newest 50) |
| `DELETE /api/diagnostics/client-crash` (admin) | `{ok}` |

The Android app writes a report when it crashes (Java/Kotlin stack, or a native crash / ANR / low-memory
kill read back from the system on the next start) and uploads it the next time it runs; `key` de-duplicates.
"Send crash log" files it as a bug report of kind `crash`.
