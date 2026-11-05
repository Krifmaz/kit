# kit-server-signup

Who may make an account, for a kit server (contract: [`contracts/signup.md`](../../contracts/signup.md)). An
admin sets **who can sign up**: Closed (only admins make accounts), Invite only (the default) or Open. Invite
links are one-time or multi-use, expire, give the new account a role and presets, can be revoked at any time and
list who used them. Open sign-up has limits per address and per day, and optionally an email confirmation and
an admin's approval. When apps share accounts (single sign-on), the account source's setting applies to all of
them and the others show it read-only.

The app keeps its accounts; the kit never touches its tables. Roles and presets ("what a new account gets":
folders it may see, what it may do …) are the app's, named by id.

- **`SignupHost`** — what the app gives the routes (on top of kit-server-core's `Host`): `signup()` (the
  store), `app_name()`, `roles()`, `presets()`, and the account hooks `create_account(NewAccount)` (made
  switched off while it waits), `enable_account(id)`, `remove_account(id)`, and `limited(key, n, secs)` (the app's rate
  limiter: true once `key` was hit `n` times in `secs`). Optional: `client_address(headers)` (the first `X-Forwarded-For` hop by
  default), `base_url()` (for links in mails and invites), `mailer()` (`LogMailer` by default: logs, sends
  nothing), `account_source()` (the app that keeps the accounts; `None` by default) and `log(level, message)`.
- **`SignupStore`** — the setting, invites, their uses and the sign-ups that wait: `SqliteSignup`, or
  `PgSignup` with feature `postgres`, in the kit's `kit_signup_settings`, `kit_invites`, `kit_invite_uses`
  and `kit_signup_requests` tables (`store::signup_contract` with feature `testing` tests an implementation).
  Tokens are kept only as their SHA-256; an invite's link is shown once, when it is made.
- **`Mailer`** — `send(to, subject, text)`; `delivers()` says whether mail really goes out (the admin page warns
  when it doesn't). `LogMailer` is the default.
- **`router()`** / **`router_with(SignupConfig)`** — `GET`/`POST /api/signup`, `POST /api/signup/invite`,
  `POST /api/signup/verify`, and for admins `/api/signup/settings`, `/api/signup/invites*`,
  `/api/signup/pending*`. `SignupConfig` holds the base path, the invite and confirmation link patterns (on
  the web app's routes, `/#/sign-up?invite={token}` and `/#/sign-up/verify?token={token}`), how long a
  confirmation link lasts (48 h), the tries per address per hour (30) and every word (`Words`).
- **`waiting(state, account_id, words)`** — why a switched-off account can't sign in yet (for the app's login:
  "Check your email …", "An admin has to approve …"), or `None` when it isn't a sign-up that waits.
- **`suite_account_source(suite, store, "/api/signup")`** (feature `suite`) — the account source of a
  kit-server-suite link with its setting, for `SignupHost::account_source`.

```rust
impl kit_server_signup::SignupHost for App {
    fn signup(&self) -> &dyn SignupStore { &self.signup }          // SqliteSignup::new(db.clone())?
    fn app_name(&self) -> String { "Demo".into() }
    fn roles(&self) -> Vec<Choice> { vec![Choice::new("user", "Member"), Choice::new("admin", "Admin")] }
    fn presets(&self) -> Vec<Choice> { vec![Choice::new("shared", "Shared space")] }
    async fn create_account(&self, a: NewAccount) -> Result<i64> { … }   // the app's users table
    async fn enable_account(&self, id: i64) -> Result<()> { … }
    async fn remove_account(&self, id: i64) -> Result<()> { … }
    fn limited(&self, key: &str, n: usize, secs: f64) -> bool { self.limiter.hit(key, n, secs) }
    async fn account_source(&self) -> Option<AccountSource> {     // with kit-server-suite (feature `suite`)
        kit_server_signup::suite_account_source(&self.suite, &self.links, "/api/signup").await
    }
}
let app = Router::new().merge(kit_server_signup::router::<App>());
```

Rules worth knowing: an invite skips the email confirmation and the approval (an admin made it); an admin's
approval counts as the email confirmation; a taken name gives the invite's use back; a rejected sign-up's
account is removed.

Tests: every route through the router (the setting, one-time and multi-use invites, expiry, revoke, who used
them, open sign-up's limits, confirmation and approval, who may do what, an account source), the store contract
on SQLite and, with `KIT_TEST_POSTGRES_URL`, Postgres, and (feature `suite`) the account source read from a
linked app on a real port.
