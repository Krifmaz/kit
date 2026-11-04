# Contracts

The shared APIs every kit server speaks and every kit client reads. The paths below are the defaults; each
server crate takes an `XPaths` struct (`router_with`) so an app can serve a feature elsewhere, and its clients
are pointed at the same paths.

| Contract | Endpoints / format | Server crate | Client logic |
| --- | --- | --- | --- |
| [Health](health.md) | `GET /api/health` | kit-server-health | kit-rules `update` |
| [Changelog](changelog.md) | `GET /api/changelog`, `GET /api/changelog/new`, `POST /api/changelog/seen` | kit-server-changelog | kit-rules `changelog` |
| [Notices](notices.md) | `GET /api/notices`, `POST /api/notices/check` | kit-server-notices | kit-leptos/iced/compose banners |
| [Bug reports](bug-reports.md) | `/api/bug-reports/*`, `/api/crash-reports` | kit-server-bugreports | bug report sheets |
| [Diagnostics](diagnostics.md) | `GET /api/diagnostics`, `POST /api/diagnostics/{id}/test` | kit-server-diagnostics | diagnostics pages |
| [Settings schema](settings-schema.md) | `GET /api/settings/schema`, `GET`/`PUT /api/settings`, `GET /api/settings/secret/{key}` | kit-server-settings | schema forms |
| [Update feed](update-feed.md) | GitHub releases of the app; `GET`/`POST /api/update`, `POST /api/restart` | kit-server-updates | in-app updaters |

[`openapi.yaml`](openapi.yaml) has the HTTP shapes in OpenAPI 3.1; the prose files say what the fields mean
and the rules behind them.

**Rule:** a contract change is a kit release with a changelog entry. Servers and clients built against the
same kit version always match. A breaking change raises MAJOR (`docs/releases/<version>.md`).

Common to every endpoint: JSON bodies; errors answer `{"detail": "<plain sentence>"}` with the HTTP status
(more fields may follow), and every error text can be shown in full and copied. "Admin" means a
signed-in admin (cookie session, app token or API key); 401 without a session, 403 for a non-admin.
