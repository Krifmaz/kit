# First-run setup

While a new server has no owner, its clients show **Create your account** instead of the sign-in page. The first
person makes the owner there. The server is kit-server-setup; the clients' rules and words are kit-rules `setup`
(kit-leptos `SetupPage`, kit-iced `setup::Setup`, kit-compose `SetupForm`).

## The setup code

So nobody else on the network can claim a new server first, the screen asks for a **setup code**:

- The server makes it at start when it has no owner and makes none from its settings (an unattended install
  sets the owner's name and password, in the template `APP_ADMIN_USER` and `APP_ADMIN_PASSWORD`; then there is
  no code and no screen).
- It is printed in the server's log, in a framed block, as three groups of four (`ABCD-EFGH-JKMN`): digits and
  capitals without the ones that look alike (0 O 1 I L U), about 59 bits. Only whoever can read the log knows it.
- It is kept in memory only. A restart before the owner exists prints a new one.
- It stops working the moment the owner exists, whoever made it.
- Typed in any case, with or without the dashes and spaces.
- Wrong codes are limited: 10 per address and 50 from everyone together in 15 minutes (429 after that).

## Endpoints

```text
GET  /api/setup  → {needed}
POST /api/setup  {code, username, password, email?} → {ok, username}
```

`GET` is public and never gives the code. `needed` is true only while there is no owner and the code is open. A
server without this route (404) means "sign in as before".

`POST` is public. In order:

| Status | When |
| --- | --- |
| 409 | the server already has its owner ("Sign in instead"), or it takes no setup code |
| 429 | too many wrong codes from this address, or from everyone |
| 400 | no code; an email that doesn't look like one |
| 403 | a wrong code (counted and logged) |
| 400 / 409 | the app doesn't take the name or password (its own words); the code still works |
| 200 | the owner is made; the code is closed |

Only one owner can be made with the code: it is taken while the account is made and put back only if the app
turns the name or password down.

After `200` the client signs in with the same name and password, the way its sign-in page does (a web session
cookie, a phone's or desktop's app token).
