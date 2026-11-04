# Contracts

The shared APIs every kit server speaks and every kit client reads. Each one matches Encore's current API
(Krifmaz/Encore at the commit in `design/fixtures/encore/SOURCE`), so Encore's existing web, Android and
desktop clients keep working while its server moves onto the kit crates.

| Contract | Endpoints / format | Server crate | Client logic |
| --- | --- | --- | --- |
| [Health](health.md) | `GET /api/health` | kit-server-health | kit-rules `update` |
| [Changelog](changelog.md) | `GET /api/changelog`, `GET /api/changelog/new`, `POST /api/changelog/seen` | kit-server-changelog | kit-rules `changelog` |
| [Notices](notices.md) | a notice object; `GET`/`POST …/check` at a path the app picks | kit-server-notices | kit-leptos/iced/compose banners |
| [Bug reports](bug-reports.md) | `/api/bug-reports/*`, `/api/diagnostics/client-crash` | kit-server-bugreports | bug report sheets |
| [Diagnostics](diagnostics.md) | `GET /api/system/diagnostics`, `POST /api/system/test/{service}` | kit-server-diagnostics | diagnostics pages |
| [Settings schema](settings-schema.md) | `GET /api/admin/settings/schema`, `GET`/`PUT /api/admin/settings` | kit-server-settings | schema forms |
| [Update feed](update-feed.md) | GitHub releases of the app; `GET`/`POST /api/system/update` | kit-server-updates | in-app updaters |

[`openapi.yaml`](openapi.yaml) has the HTTP shapes in OpenAPI 3.1; the prose files say what the fields mean
and the rules behind them.

**Rule:** a contract change is a kit release with a changelog entry. Servers and clients built against the
same kit version always match. A breaking change raises MAJOR (`docs/releases/<version>.md`).

Common to every endpoint: JSON bodies; errors answer `{"error": "<plain sentence>"}` with the HTTP status
(Encore's `encore_core::error`), and every error text can be shown in full and copied. "Admin" means a
signed-in admin (cookie session, app token or API key); 401 without a session, 403 for a non-admin.
