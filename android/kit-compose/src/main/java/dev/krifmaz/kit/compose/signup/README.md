# signup

Who can sign up, invite links and open sign-up on Android (contracts/signup.md; the server is kit-server-signup).
The path comes from `KitApp.routes.signup` (`/api/signup`).

| File | What it is |
| --- | --- |
| `SignupRules.kt` | Pure rules, the same as kit-rules' `signup`: the policy and the sign-in line, the form's checks, invite tokens and links, the admin's setting, invite and waiting rows, and `SignupWords`. |
| `SignupScreens.kt` | `SignInLine` (under the sign-in form), `SignUpScreen` (an invite link or open sign-up), `SignupAdminScreen` (who can sign up, waiting sign-ups with Approve and Reject, invites with the link shown once, Revoke and who used each; read-only when another app keeps the accounts). |

Wiring it in, with the app's `KitApi`:

```kotlin
SignInLine(api, onSignUp = { screen = "sign-up" }, openUrl = { uriHandler.openUri(it) })
SignUpScreen(api, invite = linkFromIntent, onBack = { screen = "sign-in" })
SignupAdminScreen(api, server = serverAddress, onBack = onBack) // admins only
```
