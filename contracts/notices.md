# Notices — plain-language banners

Encore's storage notice (`web-rs/src/notice.rs`, the `downloads` part of `GET /api/requests/storage`),
generalised: a server tells signed-in admins, in one plain sentence, that something is wrong and what to
check, with a "Check again" action.

## The notice object

```json
{ "ok": false, "text": "The downloads folder /downloads is read-only – check the disk (dmesg, fsck) or the mount",
  "checking": false, "id": "downloads", "severity": "warn" }
```

| Field | Meaning |
| --- | --- |
| `ok` | `false`: the banner shows; `true`: everything is fine, nothing shows |
| `text` | the sentence the banner shows, as is (no banner without one) |
| `checking` | a check still runs (a folder that does not answer) |
| `id` | the notice's id (kit addition; Encore's storage notice is `downloads`) |
| `severity` | `info`, `warn` or `err` (kit addition; Encore's storage notice is `warn`) |

Encore serves the object as the `downloads` field of `GET /api/requests/storage`, and Check again is
`POST /api/requests/storage/check`, which probes every folder at once and answers the same shape. The kit
crate lets an app mount a notice at its own path (Encore keeps these two) or at the kit's default
`GET /api/notices` (`{"notices": [<notice>…]}`) and `POST /api/notices/check`.

## Timing (clients)

Asked when the app starts and every minute; every 15 s while a notice fails, so the banner goes soon after
it is fixed. Nothing for non-admins, and nothing from a server without the endpoint.

Check again's toast: still failing → "A downloads folder still can't be written" (warn); still checking →
"Still checking the downloads folders" (warn); fixed → "The downloads folders can be written" (ok). The
generalised crate takes these three sentences per notice from the app.

Banner markup (web): `<div class="storage-banner" role="status">{alert icon}<span>{text}</span><button
class="btn sm" data-notice-check>{refresh icon}Check again</button></div>` in the shell's `#server-notice`.
