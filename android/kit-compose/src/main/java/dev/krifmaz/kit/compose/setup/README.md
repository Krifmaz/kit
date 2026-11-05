# setup

First-run setup on Android (contracts/setup.md; the server is kit-server-setup): while a new server has no owner,
the app shows "Create your account" instead of its sign-in form. The path comes from `KitApp.routes.setup`
(`/api/setup`).

| File | What it is |
| --- | --- |
| `SetupRules.kt` | Pure rules, the same as kit-rules' `setup`: whether the screen shows, the form's checks, the request body, and `SetupWords`. |
| `SetupScreen.kt` | `setupNeeded(api)` (blocking: ask on Dispatchers.IO) and `SetupForm` (setup code, username, optional email, password twice). |

Wiring it in, with the app's `KitApi`, where the sign-in form goes:

```kotlin
val needed by produceState(false, api) { value = withContext(Dispatchers.IO) { setupNeeded(api) } }
if (needed) SetupForm(api, onDone = { user, pw -> signIn(user, pw) }) else SignInForm(…)
```
