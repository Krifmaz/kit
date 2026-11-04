# Diagnostics

One page that checks every connection, folder and tool an app depends on, and says what's wrong and how to
fix it. The checks are the app's own; the kit runs them and serves the answer. Both paths below are the
defaults; an app may serve them elsewhere.

## `GET /api/diagnostics` (admin)

Every check runs at once; the answer lists them in the order the app registered them:

```json
{ "checks": [
  { "name": "Mail server", "kind": "service", "request": "GET https://mail.example.com/health", "ok": true, "detail": "200 OK · 180 ms" },
  { "name": "Data folder", "kind": "path", "ok": false, "detail": "…", "status": "fail", "fix": "…" }
] }
```

| Field | Meaning |
| --- | --- |
| `name` | what was checked, in plain words |
| `kind` | `service` (a connection), `path` (a folder), `tool` (a program) |
| `ok` | passed |
| `detail` | the result in one plain sentence (what happened) |
| `request` | what was asked, when it was a request (shown so it can be copied) |
| `status` | `ok`, `warn` or `fail`; when left out (or not one of these words, such as an HTTP status code), `ok` → `ok`, otherwise `fail` |
| `fix` | how to fix it, in plain words; optional |

Apps may add fields per check (`ms`, a response `body`, …) and sections of their own next to `checks`
(their version, the time, …).

## `POST /api/diagnostics/{id}/test` (admin)

The Test button of a settings card (`test` in the settings schema) runs that one check and answers
`{ok, message}` (plus fields of its own). A test that failed on the way answers `{ok: false, message}` with
the whole error; an id without a test answers 404 "Unknown check".
