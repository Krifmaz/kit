# kit-server-setup

First-run setup for a kit server (contract: [`contracts/setup.md`](../../contracts/setup.md)). While the app has no
owner, its clients show **Create your account** instead of the sign-in page, and the first person makes the owner
there, instead of the server making one with a password printed in its log. A one-time **setup code**, printed in
the server's log at start, keeps anyone else on the network from claiming the server first; it stops working once
the owner exists.

The app keeps its accounts; the kit never touches its tables.

- **`SetupCode`** — the code: `at_start(needed, app)` (or `open(app)` later) makes one and prints it in a framed block when `needed` (no
  owner and none made from settings), else `closed()`. `is_open()`, `code()` (`ABCD-EFGH-JKMN`), `close()`,
  `banner(app)`. Its `Debug` never shows the code.
- **`SetupHost`** — what the app gives the route: `setup_code()` (the code it keeps in its state), `has_owner()`
  and `create_owner(NewOwner { username, password, email })`, which checks the name and password with the app's
  own rules and words. Optional: `client_address(headers)` (the first `X-Forwarded-For` hop by default) and
  `log(level, message)`.
- **`router()`** / **`router_with(SetupConfig)`** — `GET`/`POST /api/setup`. `SetupConfig` holds the path, the
  limits on wrong codes (10 per address and 50 in all in 15 minutes) and every word (`Words`).
- **`needed(state)`** — whether the screen shows.

```rust
// at start, after the app made its owner from settings (if any)
let setup = SetupCode::at_start(!has_owner, "Notes");
// the route
let app = Router::new().merge(kit_server_setup::router::<AppState>());
```

After `POST /api/setup` answers 200 the client signs in with the same name and password.
