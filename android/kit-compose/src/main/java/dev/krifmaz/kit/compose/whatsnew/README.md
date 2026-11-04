# whatsnew

What's new after an update, and the whole changelog (contracts/changelog.md):

- **`WhatsNewOnStart(backend, onFullChangelog)`** — put once in the signed-in root. Asks for what is new; when `show`
  is true it shows **`WhatsNewPopup`** once and marks the version seen. Hotfix builds share their release's notes,
  so the server never sets `show` for a hotfix alone.
- **`WhatsNewPopup`** — "What's new": each release's title, summary and highlights, the switch "Show this after
  updates", **Full changelog** and **Close**.
- **`ChangelogScreen(backend)`** — Settings → What's new: the server's version, the switch "Show what's new after
  updates", and every release as a card that opens to its notes (the newest open).
- **`WhatsNewRules`** — the words, when the popup shows, JSON parsing, and grouping a release's blocks (`h`, `p`,
  `li`, `row`) into headings, paragraphs, lists and tables. Pure; tested in `WhatsNewRulesTest`.
- **`ChangelogBackend`** — the four calls. **`ApiChangelogBackend(api)`** implements them over the app's `KitApi`
  at `KitApp.routes` (`changelog`, `changelogNew`, `changelogSeen`, and `prefs` for the switch); an app with
  another source implements the interface itself.

Turning the switch says so in a toast ("What's new will show after updates" / "What's new won't show after updates –
it stays in Settings"); a failure shows the server's message as an error toast with Copy.
