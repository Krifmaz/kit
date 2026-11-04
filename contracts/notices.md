# Notices — plain-language banners

A server tells signed-in admins, in one plain sentence, that something is wrong and what to check, with a
"Check again" action. Which notices exist, their sentences and their toasts are the app's.

## The notice object

```json
{ "ok": false, "text": "The data folder /data is read-only – check the disk or the mount",
  "checking": false, "id": "disk", "severity": "warn" }
```

| Field | Meaning |
| --- | --- |
| `ok` | `false`: the banner shows; `true`: everything is fine, nothing shows |
| `text` | the sentence the banner shows, as is (no banner without one) |
| `checking` | a check still runs (a folder that does not answer) |
| `id` | the notice's id, chosen by the app |
| `severity` | `info`, `warn` or `err` |

Apps may add fields of their own (when it was checked, what it looked at, …). Anyone but an admin is told
only `{id, severity, ok}`.

## Routes

The defaults; an app may serve them at paths of its own.

- `GET /api/notices` (signed in) → `{"notices": [<notice>…]}`, every notice as it reads now, in the order
  the app registered them.
- `POST /api/notices/check` (admin) → the same, after every notice was checked again. A check that does not
  end within 15 s answers with `checking: true`.

An app may also show one notice's object inside an answer of its own.

## Timing (clients)

Asked when the app starts and every minute; every 15 s while a notice fails, so the banner goes soon after
it is fixed. Nothing for non-admins, and nothing from a server without the endpoint.

Check again's toast comes from the app's three sentences for that notice: still failing (warn), still
checking (warn), fixed (ok).

The banner (web) is a `role="status"` element in the shell holding the notice's text and a "Check again"
button.
