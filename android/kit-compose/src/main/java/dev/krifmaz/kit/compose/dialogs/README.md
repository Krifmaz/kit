# dialogs

Confirm dialogs, for anything that deletes, signs out or can't be undone.

- `Confirm(title, text, okLabel, danger, typeToConfirm, cancelLabel)` — what one dialog asks. With `typeToConfirm`
  set, OK waits until that word is typed (trimmed, exact): `Type "delete" to confirm`.
- `ConfirmRules` — `canConfirm`, `typePrompt`, the Cancel word (unit-tested).
- `ConfirmDialog(confirm, onDismiss, onConfirm)` — the dialog; a danger OK is red. The short form
  `ConfirmDialog(title, text, okLabel, danger, onDismiss, onConfirm)` stays for a plain question.
- `Confirms` / `rememberConfirms()` / `ConfirmHost(confirms)` — ask from any click handler:

```kotlin
val confirms = rememberConfirms()
SmallButton("Delete", danger = true) {
    confirms.ask(Confirm("Delete the account?", "This can't be undone.", "Delete", danger = true, typeToConfirm = "delete")) { delete() }
}
ConfirmHost(confirms) // once in the screen
```
