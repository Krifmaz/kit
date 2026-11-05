# kit-server-suite

Suite for a kit server (contract: [`contracts/suite.md`](../../contracts/suite.md)): apps on the kit link to
each other by address and key, each person sees every linked app's sections in one sidebar (and hides, shows
and reorders them), signs in once at the app that keeps the accounts, and searches every app at once. The
app names, sections, icons and search are the app's; the kit names none.

- **`SuiteConfig`** — what the app says about itself: id, name, icon (SVG path data), version, base URL
  (else each request's), `section(id, title, icon, path)`, `deep_link(name, "/things/{id}")`,
  `searchable()` (the kit's `/api/kit/search`, answered by `SuiteHost::search`), `account_source(true)` (an
  OIDC provider for linked apps) and `login_path("/login?next={next}")`.
- **`Suite`** — made once from the config, kept in the app's state: `manifest(headers)`, `base_url(headers)`,
  `account_source(store)` (the link people sign in through), `fetch_json(link, path)` (GET another app's
  route with its key), `refresh(store, link)`, `check_key(store, bearer)`. `spawn_refresh(state, every)`
  re-reads every link's manifest on a timer.
- **`SuiteHost`** — what the app gives the routes: `suite()`, `suite_store()` (links and keys),
  `sidebar_prefs()` (where each person's sidebar choices go, the kit's `PrefsStore`), and optionally
  `search(caller, q, limit)` (its own search; `Caller::Account` for its people, `Caller::App` for a linked
  app), `sso_sign_in(user, next)` (make its own session for a person the account source vouched for) and
  `oidc_claims(account)` (more claims, `email`, in the ID tokens it issues as an account source).
- **`SuiteStore`** — links and keys: `SqliteSuite`, or `PgSuite` with feature `postgres`, in the kit's
  `kit_suite_links` and `kit_suite_keys` tables (`store::suite_contract` tests an implementation). Keys are
  kept as the SHA-256 of their secret half; a link keeps the other app's key to call it.
- **`router()`** — every route at its fixed path: `GET /api/kit/manifest`, `GET /api/kit/suite/ping`,
  `GET /api/kit/search`, `/api/kit/suite/links*` and `/api/kit/suite/keys*` (admins), `GET`/`PUT
  /api/kit/suite/sidebar`, `GET /api/kit/suite/search` (every app at once; `scope=others` for "related in …"),
  `/api/kit/sso*` (signing in through the account source) and, on an account source,
  `/.well-known/openid-configuration` and `/api/kit/oidc/{authorize,token,userinfo}`.

```rust
let suite = Suite::new(
    SuiteConfig::new("demo", "Demo").version(VERSION).icon(ICON)
        .section("things", "Things", THINGS_ICON, "/things")
        .deep_link("thing", "/things/{id}")
        .searchable()
        .account_source(true),
);

impl kit_server_suite::SuiteHost for App {
    fn suite(&self) -> &Suite { &self.suite }
    fn suite_store(&self) -> &dyn SuiteStore { &self.links }   // SqliteSuite::new(db.clone())?
    fn sidebar_prefs(&self) -> &dyn PrefsStore { &self.prefs }
    async fn search(&self, caller: Caller, q: String, limit: usize) -> Result<Vec<SearchHit>> { … }
    async fn sso_sign_in(&self, user: SsoUser, next: String) -> Result<Response> { … } // session cookie + redirect
}
let app = Router::new().merge(kit_server_suite::router::<App>());
kit_server_suite::spawn_refresh(state.clone(), Duration::from_secs(900));
```

Single sign-on is OpenID Connect's authorization code flow with the account source as the provider: the
client is a key the source made (its `redirect_url` is where people may be sent back), ID tokens are HS256
signed with the key's secret, codes last 60 s and work once. Codes, access tokens and sign-ins under way live
in memory, so a restart only means signing in again.

Calls to linked apps time out after 5 s, read at most 1 MB, follow no redirects and fail with one sentence
that names the address; a linked app that is down is one search group's `error`, not the whole answer's.

Tests: two apps on real ports, linked both ways (manifest, keys, links, the sidebar, search and single
sign-on end to end), and the store contract on SQLite and, with `KIT_TEST_POSTGRES_URL`, Postgres.
