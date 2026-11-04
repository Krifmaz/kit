# Health — `GET /api/health`

What a client asks to learn which server version and which web build is running, and whether the server
is up at all. No sign-in needed.

```json
{ "ok": true, "version": "0.36.4", "server": "rust", "web": "3f9a0c1b2d4e" }
```

| Field | Meaning |
| --- | --- |
| `ok` | always `true` when the server answers |
| `version` | the server's version: `MAJOR.MINOR.PATCH`, or `MAJOR.MINOR.PATCH.N` for a hotfix build |
| `server` | the server's implementation (`"rust"`) |
| `web` | the build of the web app served at `/`: the hash in `<app>-web-<hash>.js` (12+ hex digits) in the static folder; `null` when the server serves no web build. Read once at start: a new web build only arrives with a new server process |

Apps may add fields; clients ignore fields they do not know.

## How clients use it (kit-rules `update`, from Encore's `web-rs/src/update.rs`)

- The page knows its own build from the script it loaded. It asks every minute, when the tab comes back into
  view and when the browser is online again.
- A server that does not answer is being restarted (or is down): ask every 3 s, after two minutes every 15 s,
  until it answers, then compare.
- A server that names no build is compared by `version` against the first one this page saw.
- A new build: reload at once when nothing holds the page (music playing, an upload, songs being saved for
  offline, an open dialog); otherwise a toast "A new version of Encore is ready" says why it waits and offers
  Reload; the page reloads by itself once nothing has held it for 5 s. A reload never happens while a song plays.
- With a service worker in charge, first ask the browser for the new worker and give it a while to install,
  so the reload lands in the new shell. A reload that did not bring the new build is not repeated in a loop
  (the build reloaded for is kept in sessionStorage).
