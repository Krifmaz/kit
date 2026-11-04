# Settings schema

What the settings routes answer, where the answer comes from, and how a client draws an app's Settings pages
from it. The kit's `kit-server-settings` crate serves it; the kit's clients (kit-leptos, kit-compose,
kit-iced) draw from it.

## Where it comes from

An app keeps one table of server settings, its `settings.toml`. Every setting has its default and, if it has
them, its environment variable and whether it is secret; the ones shown on a settings card also have their
card, label, help, type, options and visibility rule. The app's build script turns the file into Rust with
`kit_server_settings::file::generate`:

- `Keys`: every setting's name as a constant (`Keys::APP_NAME` is `"app_name"`);
- `TABLE`: every setting with its environment variable, default, secret flag, type, card, label and help, A to Z.
  The store (`Store`) reads it;
- `SCHEMA`: the pages, their cards and the fields on each card, in file order;
- optionally a typed accessor per setting on a type the app names (`settings.app_name()` a `String`,
  `settings.session_days()` an `i64`, a bool or `f64` likewise), which reads the value in effect.

`kit_server_settings::file::parse` gives the same table at run time.

### The file

```toml
[sections.general]       # a page, in nav order: title, aliases
title = "General"

[groups.general]         # a card, in page order: section, title, desc, part, test, test_label
section = "general"
title = "General"
desc = "Your server's name."

[keys.app_name]          # a setting: env, default, secret, group, label, help, type, options, when,
env = "APP_NAME"         #            unit, min, max, step, full, advanced
default = "Demo"         # its TOML type is the setting's type
group = "general"        # shown on that card (then label is needed)
label = "App name"

[keys.storage]
default = "local"
type = "select"
options = [["local", "This server"], ["s3", "S3 bucket"]]
group = "general"
label = "Where files go"

[keys.s3_secret]
env = "DEMO_S3_SECRET"
default = ""
secret = true            # a secret with no type is a password
group = "general"
label = "S3 secret key"
when = "storage=s3"
```

A file that does not fit stops the build with the reason: an unknown top-level entry or attribute, a page or
card without a title, a card on a page that does not exist, `test` without `test_label` (or the reverse), a
name (page id, alias or card id) that would open two different pages, a key that is not lower case letters,
digits and `_`, a key that a Rust keyword or a method of the accessor type takes, a missing default, an unknown
type, a type that does not match the default, a password that is not secret, options on a field that is not a
select or checks (or a select or checks without them), an environment variable name that is not upper case
letters, digits and `_`, a field on a card with no label, a secret on a card that is not a password or
textarea, card-only attributes on a key with no card, `unit`/`min`/`max`/`step` on a field that is not a
number, and a `when` rule that does not parse, names no setting, names one that is not a select, names an
option it does not have, or names the field itself with `=`.

## The routes

| Route (default) | Who | What |
|---|---|---|
| `GET /api/settings` | admin | every value; secrets masked; `_meta` |
| `PUT /api/settings` | admin | save some values; answers like `GET`, plus `_warnings` |
| `GET /api/settings/schema` | admin | the pages, cards and fields (below); no values |
| `GET /api/settings/secret/{key}` | owner | one saved value in plain text: `{"key", "value"}` |

An app may serve them on other paths (`SettingsPaths`); clients take the paths as configuration. Without a
session: 401 `{"detail": "Not authenticated"}`; a member who is not an admin: 403 `{"detail": "Admin only"}`;
the secret route for anyone but the owner: 403 `{"detail": "Only the server owner can do this"}`, an unknown
key 404 `{"detail": "Unknown setting"}`, and a secret the app never shows again 403 with the app's reason.
Every field in the schema is a key of the values answer.

### Values (`GET /api/settings`)

```json
{
  "app_name": "Demo",
  "s3_secret": "•••• (saved)",
  "_meta": {
    "app_name": { "source": "default", "env": "APP_NAME" },
    "s3_secret": { "source": "env", "env": "DEMO_S3_SECRET" }
  }
}
```

The value in effect is the environment variable's (quotes and spaces around it dropped), then the saved one,
then the default, each taken to the setting's type. A secret is `"•••• (saved)"` when it has a value and `""`
when not. `_meta.<key>.source` is `env`, `ui` (saved) or `default`; `env` is the variable's name or `null`.

### Saving (`PUT /api/settings`)

The body is `{key: value}`. Unknown keys, masked values (starting with `••••`), keys an environment variable
sets and keys the app keeps out of saves are left out; the rest are saved, each taken to its setting's type.
The app may refuse the whole save (an error, nothing saved) or answer `_warnings: [{"key", "message"}]`.

## The schema answer (`GET /api/settings/schema`)

```json
{
  "sections": [
    {
      "id": "storage",
      "title": "Storage",
      "aliases": [],
      "groups": [
        {
          "id": "files",
          "title": "Files",
          "part": "",
          "desc": "Where the server keeps uploaded files.",
          "test": "files",
          "test_label": "Test connection",
          "fields": [
            {
              "key": "s3_secret",
              "label": "S3 secret key",
              "help": null,
              "type": "password",
              "options": [],
              "when": "storage=s3",
              "secret": true,
              "env": "DEMO_S3_SECRET",
              "env_locked": false,
              "default": "",
              "unit": null,
              "min": null,
              "max": null,
              "step": null,
              "full": false,
              "advanced": false
            }
          ]
        }
      ]
    }
  ]
}
```

Every attribute is always there; one that does not apply is `null` (or `[]`, `false`, `""` as listed).

### Section (a page)

| Field | Meaning |
|---|---|
| `id` | the page's id in links |
| `title` | the nav entry and the page heading |
| `aliases` | other names that open this page in a link (`[]` for none) |
| `groups` | the cards on the page, in order. **Empty** for a page the app draws with its own code: it is listed so the nav order comes from one place |

Sections come in nav order. A link name is looked up as a page id first, then as an alias, then as a card id
(which opens the card's page with that card open); no name opens two different pages.

### Group (a card)

| Field | Meaning |
|---|---|
| `id` | the card's id |
| `title`, `desc` | the card's heading and the line under it (`""` for none) |
| `part` | the heading over a run of cards on the page; `""` for none. A new heading starts where `part` changes |
| `test` | the id of the check the card's Test button runs (the kit's clients save the card, then call `POST /api/diagnostics/<test>/test`); `null`: no button |
| `test_label` | the button's words; set whenever `test` is |
| `fields` | the card's fields, in order |

### Field

| Field | Meaning |
|---|---|
| `key` | the setting's name in the values answer |
| `label` | the field's label |
| `help` | the line under the field; `null` for none |
| `type` | how to draw it, below |
| `options` | `[{"value", "label"}]` for `select` and `checks`, in order; `[]` otherwise. Values are always strings, even when the setting is a number: compare the setting's value as text |
| `when` | visibility rule, below; `null`: always shown |
| `secret` | the server never sends the value (see Values). A password field is always secret; a secret textarea too |
| `env` | the environment variable that sets it, `null` for none |
| `env_locked` | that variable is set on this server: the value comes from it, saving it does nothing (the field is shown disabled). Same as `_meta.<key>.source == "env"` |
| `default` | the value when nothing is saved and no environment variable is set; its JSON type is the setting's type |
| `unit` | what a number counts (`"days"`, `"MB"`, …) |
| `min`, `max` | the sensible range of a number (a hint; the server does not refuse other values) |
| `step` | the number input's step (`0.1`); `null`: whole numbers |
| `full` | the field takes the card's whole width |
| `advanced` | a field for people who know what they are doing |

### Types

| `type` | Drawn as | Value |
|---|---|---|
| `text` | text input | string |
| `path` | text input; the value is a folder on the server | string |
| `url` | text input; the value is an address | string |
| `password` | password input that starts empty (empty keeps the saved value) | string |
| `textarea` | text area (always full width) | string |
| `number` | number input with `step` | number |
| `bool` | switch | boolean |
| `select` | drop-down of `options` | string (or a number, see `options`) |
| `checks` | a row of check boxes, one per option | comma-separated option values (`"a,b"`), compared without case and punctuation |

A setting's type, when the file does not give one, follows from its default: a boolean is `bool`, a number
`number`, a string `password` when secret and `text` otherwise. `path` and `url` may be drawn exactly like
`text`; the type tells what the value is, for clients that want to offer a folder picker or check an address.

### Visibility rules (`when`)

`"key=value"`: the field is shown only while the select `key` has `value`; `"key!=value"`: only while it has
any other value. `key` is always a `select` and `value` one of its options (the build checks both). A field
hidden by a rule is still saved with its card.

## What is not in the schema

Settings with no card (`group`) are in the table and the values answer but not in the schema: values the
server keeps for itself, or that the app edits on a screen of its own.

## Changing the table

A new setting is a `[keys.<name>]` entry in `settings.toml`; with `group` it shows on that card, after the
card's other fields in file order. Nothing else is needed: the store, the routes, the schema and the accessors
follow from the file.
