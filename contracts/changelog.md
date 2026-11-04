# Changelog — What's new

Release notes for the What's new popup (shown once after an update) and the full What's new list. The paths
are the defaults; an app can serve them elsewhere (kit-server-changelog `ChangelogPaths`).

The notes are the app's own: a folder of `<MAJOR.MINOR.PATCH>.md`, built into the server or read at start,
or a list the app makes. The app's name and version are the app's. What counts as new is per
account: `prefs.whats_new_seen` holds the version the person last saw, and only releases with notes newer than
it (and not newer than this server) are new. **Hotfix builds** (`1.4.0.17`) share their release's notes
(`MAJOR.MINOR.PATCH` of the version), so a hotfix alone never shows the popup again.
`prefs.whats_new_popup = false` turns the popup off; the full list still shows everything.

## `GET /api/changelog/new` (signed in)

```json
{ "version": "1.4.0", "popup": true, "show": true,
  "releases": [ { "version": "1.4.0", "title": "Demo 1.4.0", "summary": "…", "highlights": ["…"] } ] }
```

| Field | Meaning |
| --- | --- |
| `version` | this server's version |
| `popup` | the account's popup switch (`prefs.whats_new_popup`, default `true`) |
| `show` | `popup` and at least one unseen release: the client shows the popup |
| `releases` | unseen releases, newest first: at most 4 (`POPUP_RELEASES`); an account that never saw the popup gets only the newest one |
| `highlights` | at most 6 per release (`HIGHLIGHTS`): the bold leads of the bullets, else the first bullets |

## `POST /api/changelog/seen` (signed in)

Marks this server's version seen for the account: `{ "ok": true, "seen": "1.4.0" }`.

## `GET /api/changelog` (signed in)

Every release with notes, newest first, each `{version, title, summary, highlights, blocks}`, with the
account's popup switch. `blocks` are the notes part by part, inline markdown taken out (bold, italics, code,
links keep their words), tagged by `k`:

| `k` | Fields | From |
| --- | --- | --- |
| `h` | `text` | `### Heading` |
| `p` | `text` | a paragraph |
| `li` | `lead` (bold start, left out when empty), `text` | a bullet |
| `row` | `cells`, `head` | a table row (`head` for the first) |

`title` is "<App> <version>", or what follows the version in the notes' heading. `summary` is the paragraph
before the first section. Grouping of blocks into lists and tables for display is kit-rules `changelog`.
