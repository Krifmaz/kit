# Sign-up — who may make an account

An admin sets **who can sign up**: Closed (only admins make accounts), Invite only (the default) or Open. Invite
links can be one-time or multi-use, expire, give the new account a role and presets, can be revoked at any time,
and list who used them. Open sign-up is held back by limits per address and per day, and can ask for an email
confirmation and an admin's approval. The app keeps its own accounts: the kit asks it to make, switch on and
remove them. Which roles and presets there are is up to the app; the kit names none.

Server crate: `kit-server-signup`. Client rules: kit-rules `signup` (and kit-compose `signup.SignupRules`).
Screens: kit-leptos `signup`, kit-iced `signup`, kit-compose `signup`. The paths start at `/api/signup` by default
(`SignupConfig::base`).

## The setting, as everyone sees it

`GET /api/signup` (public):

```json
{ "who": "open", "verify_email": true, "approval": false, "managed_by": null }
```

`who` is `closed`, `invite` or `open`. The sign-in page shows "Create an account" for `open`, a line saying an
invite link is needed for `invite`, and nothing for `closed`.

`managed_by` is set when the app shares accounts with others (single sign-on) and another app keeps them:
`{ "name": "Accounts", "url": "https://accounts.example", "reachable": true }`. The other fields are then that
app's setting (`who` is `null` while it can't be reached, `reachable: false`). The sign-in page says accounts are
managed there and links to it; every sign-up and admin write answers 409 with the same `managed_by`.

## Signing up

`POST /api/signup` (public) `{username, password, email?, invite?}`:

```json
{ "status": "verify_email", "message": "Check your email for a link to confirm your address.", "account_id": 7 }
```

| `status` | Meaning |
| --- | --- |
| `active` | the account can sign in now |
| `verify_email` | it waits for the link mailed to `email` (open sign-up with `verify_email`) |
| `approval` | it waits for an admin (open sign-up with `approval`) |

- With an `invite` token: works unless the setting is Closed. The account gets the invite's role and presets,
  and skips the email confirmation and the approval. A token that is unknown, revoked, expired or used up
  answers 400 with which.
- Without one: only when the setting is Open, else 403. The account gets the setting's `open_role` and
  `open_presets`. 429 past `per_hour` sign-ups from one address or `per_day` in all.
- Every public call (sign up, check an invite, confirm an email) counts against `tries_per_hour` per address
  (429 past it).
- A name already taken (in any case) is 409, and an invite's use is given back.
- An account that waits is made switched off; signing in to it answers 403 with the `message` of its status.

`POST /api/signup/invite` (public) `{token}`: whether a link works before the form is filled:
`{ok, role, presets, expires_at}` with the labels of the role and presets.

`POST /api/signup/verify` (public) `{token}` (the mailed link, `verify_hours` long): `{status, message}`; the
account is switched on unless it still waits for an admin.

## The admin's page

`GET /api/signup/settings` (admin):

```json
{ "settings": { "who": "invite", "open_role": "", "open_presets": [], "verify_email": false, "approval": false,
                "per_hour": 5, "per_day": 50 },
  "roles": [{ "id": "user", "label": "Member" }], "presets": [{ "id": "shared", "label": "Shared space" }],
  "mail_delivers": false, "managed_by": null, "read_only": false }
```

`mail_delivers` is false while the app's mailer only logs (the page says confirmation mails won't arrive).
`read_only` is true when `managed_by` is set: the page shows that app's setting, with no invites or waiting
sign-ups. `PUT /api/signup/settings` takes `settings`' fields and answers the same shape.

`GET /api/signup/invites` (admin), newest first:

```json
{ "invites": [{ "id": 3, "note": "Team", "role": "user", "presets": ["shared"], "max_uses": 5, "uses": 2,
                "expires_at": 1793836800.0, "revoked_at": 0.0, "status": "active", "created_by": 1,
                "created_by_name": "admin", "created_at": 1793232000.0,
                "used_by": [{ "invite_id": 3, "account_id": 9, "username": "ana", "used_at": 1793300000.0 }] }] }
```

`max_uses` is 1 for a one-time link and 0 for no limit; `expires_at` 0 never expires. `status` is `active`,
`revoked`, `expired` or `used_up`.

`POST /api/signup/invites` (admin) `{note, role, presets, max_uses, expires_in_hours}` (`expires_in_hours` 0:
never; the default 168) answers `{invite, token, link}`. The link (`SignupConfig::invite_link`, on the app's
address) is in this answer only: only the token's SHA-256 is kept. `GET /api/signup/invites/{id}` and
`POST /api/signup/invites/{id}/revoke` answer `{invite}`.

`GET /api/signup/pending` (admin), oldest first: `{pending: [{id, account_id, username, email, email_verified,
needs_approval, approved, address, created_at, waiting_for}]}`. `POST /api/signup/pending/{id}/approve` switches
the account on (an admin's approval counts as the email confirmation too) and answers `{ok, status}`;
`POST /api/signup/pending/{id}/reject` removes the account and answers `{ok}`.
