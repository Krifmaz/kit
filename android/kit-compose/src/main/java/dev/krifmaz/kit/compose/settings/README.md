# settings

Settings drawn from the server's schema, and the Diagnostics page (DESIGN.md §6, §11; contracts/settings-schema.md,
contracts/diagnostics.md). The paths come from `KitApp.routes` (`settingsSchema`, `settings`, `settingsSecret`,
`settingsTest`, `diagnostics`, `clientCrash`).

| File | What it is |
| --- | --- |
| `SchemaModels.kt` | The schema answer and `SchemaApi`, the calls behind the cards. |
| `SchemaRules.kt` | Pure rules: visibility (`when`), what Save sends, number checks, where a value comes from, keys never shown (`neverShown`), the layout of a framed block. |
| `SchemaForm.kt` | `SchemaFormState`, `SchemaPage`, `SchemaCard`, the fields, Test and Save, `CardSlots` for a card's own extras and `FieldBlock` for a framed block of fields (one key prefix). |
| `DiagnosticsRules.kt` | The check groups, Copy report's text, app crash kinds and text. |
| `DiagnosticsScreen.kt` | `DiagnosticsScreen` (checks, Server block, where settings come from, slots for the app's own parts) and `AppCrashesSection`. |
| `SettingsWidgets.kt` | The Settings page frame and the web's cards, section heads, muted/mono text, dots, links, `SecureWindow`, `serverCall`. |

Wiring it in: implement `dev.krifmaz.kit.compose.core.KitApi` over the app's client, then

```kotlin
val schemaApi = SchemaApi(api, http)
val state = withContext(Dispatchers.IO) { schemaApi.loadState(canReveal = isOwner) }
SchemaPage(state, schemaApi, state.schema.section("general")!!,
    slots = { g -> if (g.id == "tunnel") CardSlots(block = FieldBlock("tunnel_", "Connection")) else CardSlots.None })

DiagnosticsScreen(
    title = "Diagnostics", subtitle = "…", onBack = onBack,
    load = { api.get(KitApp.routes.diagnostics) as JsonObject },
    report = { DiagnosticsRules.report(it) },
    bottom = { AppCrashesSection(load = { … }, clear = { … }) },
)
```

Tests: `src/test/.../settings`, including a check that the demo app's schema (`android/demo/src/main/assets/
settings-schema.json`, every kind of field) decodes and holds together.
