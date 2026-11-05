# Suite — linked apps, one sidebar, one sign-in, one search

Apps on the kit link to each other. Each person sees the sections of every linked app in one sidebar (and
hides, shows and reorders them), signs in once at the app that keeps the accounts, and searches every app at
once. Which apps, sections, words and icons there are is up to each app's manifest; the kit names none.

Server crate: `kit-server-suite`. Client rules: kit-rules `suite`. The paths are fixed, since apps find
each other by them; an app served under a prefix says so in its base URL.

## The manifest

`GET /api/kit/manifest` (public):

```json
{ "kit_suite": 1, "id": "notes", "name": "Notes", "icon": "M4 4h16v16H4z", "version": "1.4.0",
  "base_url": "https://notes.example",
  "sections": [{ "id": "all", "title": "All notes", "icon": "M4 6h16M4 12h16M4 18h10", "path": "/notes" }],
  "deep_links": { "note": "/notes/{id}", "tag": "/tags/{tag}" },
  "search": "/api/kit/search",
  "sso": { "issuer": "https://notes.example" } }
```

| Field | Meaning |
| --- | --- |
| `kit_suite` | the manifest format, `1` |
| `id` | the app's id: `a-z`, `0-9`, `-`, at most 64; unique among linked apps |
| `name` | the app's name as people see it |
| `icon` | SVG path data in a 24×24 box, drawn as a 2 px round stroke in the text colour (like the kit's own icons) |
| `version` | the app's version |
| `base_url` | where people reach the app, no trailing `/` (the configured one, else the request's `Host`, behind a proxy `X-Forwarded-Host` / `X-Forwarded-Proto`) |
| `sections` | the app's sidebar entries in its own order: `id` (unique in the app), `title`, `icon` (empty: the app's), `path` from `base_url` |
| `deep_links` | named path patterns with `{placeholders}`, filled percent-encoded |
| `search` | the app's search endpoint (below), or `null` |
| `sso` | `{issuer}` when the app is an account source (an OIDC provider), else `null` |

Clients ignore fields they do not know.

## Keys and links (admin)

Linking app B to app A: an admin of A makes a key for B, and an admin of B pastes A's address and the key.

- `GET /api/kit/suite/keys` → `{"keys": [{id, name, redirect_url, created_at, last_used_at}…]}`
- `POST /api/kit/suite/keys` `{name, redirect_url?}` → the key's object plus `key`, the whole key
  (`ks_<12 hex>.<secret>`). It is shown this once; A keeps only the SHA-256 of the secret. `redirect_url` is
  B's base URL: people may be signed in to B through A only when it is set.
- `DELETE /api/kit/suite/keys/{id}` → `{ok}`; the key stops working at once.
- `GET /api/kit/suite/ping` with `Authorization: Bearer <key>` → `{id, name, key}` of A; 401 for a wrong or
  revoked key.

- `GET /api/kit/suite/links` → `{"links": [<link>…]}`; a link is `{id, base_url, name, account_source,
  manifest, error, ok, checked_at, created_at}` (never the key).
- `POST /api/kit/suite/links` `{url, key, account_source?}`: B reads A's manifest, checks it (format, id, not
  B itself, not linked already) and the key (ping), and keeps the link → the link. 400 with the reason when A
  can't be read or does not take the key; 409 when A is linked already. `account_source` needs A to offer
  `sso`.
- `PATCH /api/kit/suite/links/{id}` `{account_source?, key?}`: make it the account source (at most one link
  is; making one clears the others) or not; or give it a new key (checked first).
- `POST /api/kit/suite/links/{id}/refresh` → the link after reading A's manifest again; when that fails the
  last manifest is kept and `error` says why. Servers also refresh every link on a timer.
- `DELETE /api/kit/suite/links/{id}` → `{ok}`.

## The shared sidebar (signed in)

`GET /api/kit/suite/sidebar` →

```json
{ "items": [{ "key": "notes/all", "app": "notes", "app_name": "Notes", "section": "all", "title": "All notes",
              "icon": "M4 6h16…", "url": "https://notes.example/notes", "path": "/notes",
              "hidden": false, "local": true }],
  "apps": [{ "id": "notes", "name": "Notes", "icon": "…", "base_url": "https://notes.example", "local": true, "error": null }] }
```

Every section of this app (`local: true`) and of every linked app, in the person's order: the keys they
ordered first, then the rest in the apps' own order (this app first, then links in the order they were made).
Hidden entries are in the list with `hidden: true`; clients show them only while the person edits the
sidebar. A click on a local entry opens its `path` in place; any other opens `url` (the other app, at that
section).

`PUT /api/kit/suite/sidebar` `{order: [key…], hidden: [key…]}` keeps the person's choices (in their
preferences, under `suite_sidebar`) and answers as `GET`. `{}` puts everything back in the apps' order.

## Search

**An app's own search.** `GET <search>?q=&limit=` (the manifest's `search`; the kit serves
`/api/kit/search`) answers `{"results": [<hit>…]}`:

```json
{ "title": "Shopping list", "subtitle": "Notes · last week", "url": "/notes/41", "section": "all", "icon": null }
```

`url` is a path in the app or a full URL. The app's own signed-in people may call it, and linked apps with a
key the app made (`Authorization: Bearer <key>`); a linked app also names the person it searches for in
`X-Kit-Suite-User` (their user name there), a hint the app may use or ignore. 400 for an empty `q` or one over
200 characters; `limit` is 1 to 50 (default 10).

**Every app at once.** `GET /api/kit/suite/search?q=&scope=&apps=&limit=` (signed in) →
`{"query": "…", "groups": [{app, app_name, icon, local, results: [<hit>…], error}…]}`: this app first, then each
linked app with a search, asked at the same time (5 s each), every hit's `url` made full. Only apps that found
something or failed are listed; a failed app's `error` says why in one sentence and the others still answer.
`scope=others` leaves this app out (cross-links: "Related in …"), `scope=self` leaves the others out;
`apps=a,b` asks only those app ids.

## Single sign-on

One app is the account source: an OpenID Connect provider (authorization code flow). The others mark their
link to it as the account source and sign people in through it.

**Account source** (only when it is one; 404 otherwise):

- `GET /.well-known/openid-configuration`: issuer (its base URL), endpoints, `HS256`,
  `client_secret_post` / `client_secret_basic`.
- `GET /api/kit/oidc/authorize?response_type=code&client_id=<key id>&redirect_uri&state&nonce`: the client is
  a key the source made; `redirect_uri` must be its `redirect_url` or a path below it (400 otherwise, without
  a redirect). A signed-in person is sent back with `code` and `state`; anyone else goes to the source's
  sign-in page first and comes back after. No consent page: an admin linked the apps.
- `POST /api/kit/oidc/token` (form) `grant_type=authorization_code&code&redirect_uri` with the key id and
  secret → `{access_token, token_type: "Bearer", expires_in, id_token}`. Codes last 60 s and work once.
- `GET /api/kit/oidc/userinfo` with the access token → the claims.

The ID token is an HS256 JWT signed with the key's secret (OIDC Core §10.1) with `iss`, `aud` (the key id),
`sub` (the account's id at the source), `iat`, `exp` (5 minutes), `nonce`, `preferred_username`, `name`,
`kit_admin`, `kit_owner`, and whatever more the source adds (`email`).

**Linked app:**

- `GET /api/kit/sso` (public) → `{enabled, source: {name, icon, base_url}, start}`, so the sign-in page can
  offer "Sign in with …".
- `GET /api/kit/sso/start?next=/path` → off to the source with a new `state` and `nonce` (kept 10 minutes).
- `GET /api/kit/sso/callback?code&state` → the token exchange and the checks (signature, `iss`, `aud`, `exp`,
  `nonce`), then the app's own session and a redirect to `next` (a path in the app; anything else becomes
  `/`). A failure goes back to the sign-in page with the reason in `sso_error`.

The linked app matches its accounts on issuer + `sub` and decides itself what `kit_admin` means there.
