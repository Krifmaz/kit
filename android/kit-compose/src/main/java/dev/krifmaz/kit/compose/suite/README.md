# suite

Linked apps on the kit ([contracts/suite.md](../../../../../../../../../../contracts/suite.md)): each person's
shared sidebar with every linked app's sections, and one search across them. The rules are kit-rules' `suite`,
the same on every client.

| Name | Use it |
| --- | --- |
| `SuiteRules` | parse the sidebar and search answers, `move`, `toggle`, `prefsOf` (the `PUT` body), `searchPath(q, scope)`, `fill` |
| `rememberSidebar(api)` / `SidebarState` | the sidebar, asked once (`refresh`) and saved after editing (`save`) |
| `SuiteNav(state, active, onLocal, onBrowse)` | the entries; a local one calls `onLocal(path)`, another app's `onBrowse(url)`; Edit sidebar: move up and down, hide, show, reset |
| `SuiteSearch(api, onOpen)` | the field and each app's hits (or why it failed) |
| `Related(api, query, onOpen)` | "Related in <app>" from the other apps; nothing while they have nothing |
| `PathIcon(d, tint)` / `pathIcon(d)` | a suite icon (SVG path data in a 24×24 box) as an `ImageVector` |
| `SuiteWords` | every word, to replace for another language |

```kotlin
val sidebar = rememberSidebar(app.api)
SuiteNav(sidebar, active = "/#/", onLocal = { path -> openPage(path) }, onBrowse = { url -> uriHandler.openUri(url) })
```
