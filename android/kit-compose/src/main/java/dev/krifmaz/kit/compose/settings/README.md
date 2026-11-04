# settings

Settings drawn from the server's schema, and the Diagnostics page (DESIGN.md §6, §11), extracted from Encore's
`clients/android/app/src/main/java/app/encore/android/features/settings/`.

| File | What it is | From Encore |
| --- | --- | --- |
| `SchemaModels.kt` | The schema answer (`GET /api/settings/schema`, contracts/settings-schema.md) and `SchemaApi`, the calls behind the cards. | `SchemaModels.kt` |
| `SchemaRules.kt` | Pure rules: visibility (`when`), what Save sends, number checks, where a value comes from. | `SchemaRules.kt` |
| `SchemaForm.kt` | `SchemaFormState`, `SchemaPage`, `SchemaCard`, the fields, Test and Save, `CardSlots` for a card's own extras. | `SchemaForm.kt` |
| `DiagnosticsRules.kt` | The check groups, Copy report's text, app crash kinds and text. | `SettingsRules.kt` (diagnostics part) |
| `DiagnosticsScreen.kt` | `DiagnosticsScreen` (checks, Server block, where settings come from, slots for the app's own parts) and `AppCrashesSection`. | `DiagnosticsScreen.kt` |
| `SettingsWidgets.kt` | The Settings page frame and the web's cards, section heads, muted/mono text, dots, links, `SecureWindow`, `serverCall`. | `SettingsWidgets.kt` |

Wiring it in: implement `dev.krifmaz.kit.compose.core.KitApi` over the app's client, then

```kotlin
val schemaApi = SchemaApi(api, http)
val state = withContext(Dispatchers.IO) { schemaApi.loadState(canReveal = isOwner) }
SchemaPage(state, schemaApi, state.schema.section("general")!!)

DiagnosticsScreen(
    title = "Diagnostics", subtitle = "…", onBack = onBack,
    load = { api.get("/api/diagnostics") as JsonObject },
    report = { DiagnosticsRules.report(it) },
    bottom = { AppCrashesSection(load = { … }, clear = { … }) },
)
```

Tests: `src/test/.../settings` (Encore's `SchemaRulesTest`, with the web's recorded schema answer as a fixture, and
the diagnostics part of `SettingsRulesTest`).
