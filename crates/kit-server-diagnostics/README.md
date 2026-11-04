# kit-server-diagnostics

The diagnostics page of a kit server (contract: [`contracts/diagnostics.md`](../../contracts/diagnostics.md)):
one page that checks every connection, folder and tool the app depends on, and says what's wrong and how to
fix it. The checks are the app's; the kit runs them, orders them and serves them.

- **`Check`** — one line: `{name, kind, request?, ok, detail, status?, fix?, …}` plus fields of the app's
  own. Build one with `Check::new(name, Kind::Path, ok, detail).request(path).fix("…")`, or keep an object
  the app built as JSON (`Check::from(value)` comes out byte for byte the same). `outcome()` is the kit
  status: `status` when it is `ok`/`warn`/`fail`, else `ok` → ok and anything else → fail (an app may carry
  an HTTP status code under `status`; that is not a kit status).
- **`Registry`** — the app's checks in groups (`group(id, |st| async { Ok(vec![...]) })`, none when
  something is not set up, several for every folder) and its settings cards' Test buttons
  (`test(id, |st, who| …)`, or `tests(|st, id, who| …)` for an app that looks them up itself).
  Every group runs at once; the page lists them in the order they were added.
- **`Diagnostics`** — the trait the app implements: `diagnostics()` gives the registry, `answer(checks)`
  may add sections of its own to the answer (default `{"checks": [...]}`).
- **`router()`** — `GET /api/diagnostics` and `POST /api/diagnostics/{id}/test` (admins);
  **`router_with(DiagnosticsPaths { list, test })`** serves them at the app's own paths. A test answers
  `{ok, message, …}`; a test that failed on the way `{ok: false, message}`; an unknown one 404
  "Unknown check".

```rust
impl kit_server_diagnostics::Diagnostics for App {
    fn diagnostics(&self) -> Registry<App> {
        Registry::new()
            .group("folders", |app: App| async move {
                Ok(vec![Check::new("Data folder", Kind::Path, true, "OK – 12 entries").request("/data")])
            })
            .test("email", |app: App, who: Account| async move { Ok(json!({ "ok": true, "message": "Sent" })) })
    }
}
let router = Router::new().merge(kit_server_diagnostics::router::<App>());
```
