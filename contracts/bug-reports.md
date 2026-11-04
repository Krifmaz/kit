# Bug reports and crash logs

A user writes what went wrong, attaches screenshots or a log, and the admin answers in a thread; the admin can
export the lot. The paths below are the defaults: an app can serve them under another base
(kit-server-bugreports `BugReportPaths`, `crash::CrashPaths`), and its limits and words are its own
(`Limits`, `Words`).

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

A file: `{id, name, mime, size, url}` (`url` is `<base>/{report}/files/{id}`, `/api/bug-reports/…` by default).

- `kind`: one of the app's kinds; by default `bug`, `not_bug`, `howto`, `crash` (a crash log an app sends).
  A report without one gets the first kind.
- `status`: `new`, `looking`, `fixed`, `wontfix`, `answered` (fixed ids; the app names them).
- A non-admin only sees their own reports; someone else's answers 404, not 403.
- Files live under `<data_dir>/reports/<report_id>/<file_id>.<ext>`; a path on disk is only ever built
  from a database row.

Default limits: 8 MiB per file, 6 files per report or message, title 200 characters, text 20 000, file name 120;
an image may decode to at most 128 MiB and 8192 px a side; 20 reports and 60 replies per user per hour;
500 MiB of files per user (admins exempt); an export holds at most 512 MiB.

## App crash reports

| Request | Answer |
| --- | --- |
| `POST /api/crash-reports` `{kind, device, device_id, app_version, os, time, summary, text, key}` | `{ok, id, duplicate}` |
| `GET /api/crash-reports` (admin) | `{crashes: [{id, kind, user, device, device_id, app_version, os, time, received_at, first_line, text}]}` (newest 50 by default) |
| `DELETE /api/crash-reports` (admin) | `{ok}` |

A client app writes a report when it crashes (a stack trace, or a native crash / ANR / low-memory kill read
back from the system on the next start) and uploads it the next time it runs; `key` de-duplicates (per
account and device). A "send crash log" action files it as a bug report of kind `crash`.
