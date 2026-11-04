# Diagnostics

One page that checks every connection, folder and tool, and says what's wrong and how to fix it. From
Encore's `GET /api/system/diagnostics` (`server/crates/api/src/routes/stats.rs`) and the Diagnostics page
(`web-rs/src/settings/diagnostics.rs`, `DiagnosticsScreen.kt`).

## `GET /api/system/diagnostics` (admin)

Every check runs at once; the answer lists them:

```json
{ "checks": [
  { "name": "MusicBrainz", "kind": "service", "request": "GET https://musicbrainz.org/…", "ok": true, "detail": "200 OK · 180 ms" },
  { "name": "Music library", "kind": "path", "ok": false, "detail": "…", "status": "fail", "fix": "…" }
] }
```

| Field | Meaning |
| --- | --- |
| `name` | what was checked, in plain words |
| `kind` | `service` (a connection), `path` (a folder), `tool` (a program) |
| `ok` | passed |
| `detail` | the result in one plain sentence (what happened) |
| `request` | what was asked, when it was a request (shown so it can be copied) |
| `status` | kit: `ok`, `warn` or `fail`; when left out, `ok` → `ok`, otherwise `fail` |
| `fix` | kit: how to fix it, in plain words; optional |

Apps may add fields per check (Encore's Soulseek check carries `soulseek`).

## `POST /api/system/test/{service}` (admin)

The Test button of a settings card (`test` in the settings schema) runs that one check and answers
`{ok, detail}` (plus fields of its own).
