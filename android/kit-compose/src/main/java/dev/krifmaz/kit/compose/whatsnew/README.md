# whatsnew

What's new, as Encore's web app has it (`web-rs/src/whats_new.rs`, contracts/changelog.md):

- **`WhatsNewOnStart(backend, onFullChangelog)`** — put once in the signed-in root. Asks `GET /api/changelog/new`;
  when `show` is true it shows **`WhatsNewPopup`** once and calls `POST /api/changelog/seen`. Hotfix builds share
  their release's notes, so the server never sets `show` for a hotfix alone.
- **`WhatsNewPopup`** — "What's new": each release's title, summary and highlights, the switch "Show this after
  updates", **Full changelog** and **Close**.
- **`ChangelogScreen(backend)`** — Settings → What's new: the server's version, the switch "Show what's new after
  updates", and every release as a card that opens to its notes (the newest open).
- **`WhatsNewRules`** — the words, when the popup shows, JSON parsing, and grouping a release's blocks (`h`, `p`,
  `li`, `row`) into headings, paragraphs, lists and tables. Pure; tested in `WhatsNewRulesTest`.
- **`ChangelogBackend`** — the four server calls, implemented by the app with its own client and session.

Turning the switch says so in a toast ("What's new will show after updates" / "What's new won't show after updates –
it stays in Settings"); a failure shows the server's message as an error toast with Copy.

Encore's Android app doesn't show What's new today, so it doesn't use this package yet (showing it would be a visible
change); new apps and Encore's other clients' behaviour are the reference.
