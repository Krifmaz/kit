# notices

Plain-language banners for a problem the server found (contracts/notices.md): one sentence on what is wrong and what
to check, with **Check again**. Admins only.

- `Notice(ok, text, checking, id, severity)` and `Severity` (`info`, `warn`, `err`).
- `NoticeRules` — reading `GET <notices>` (`{"notices": [...]}`, or one notice object), which banners show, the toast
  after Check again (`CheckWords` per notice: still failing, still checking, fixed), and the timing (`NoticePoll`:
  every 15 s while one fails, else every minute). Unit-tested.
- `rememberNotices(api, admin)` — polls `KitApp.routes.notices` while the screen shows; Check again is
  `POST <notices>/check`. `NoticesState(load, check)` takes other calls for a server that keeps its notice elsewhere.
- `NoticeBanners(state, admin, words)` / `NoticeBanner` — the banners: an alert icon in the severity's colour, the
  sentence, and Check again (Checking… while it runs).

```kotlin
val notices = rememberNotices(api, admin = me.admin)
NoticeBanners(notices, admin = me.admin, words = mapOf("mail" to CheckWords(fixed = "The mail server answers again")))
```
