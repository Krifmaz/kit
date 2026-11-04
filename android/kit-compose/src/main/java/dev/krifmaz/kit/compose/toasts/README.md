# toasts

`Toasts.show(message, kind)` from anywhere; `ToastHost(bottomPadding)` once in the app's root. Kinds: "info",
"ok", "warn", "err". Errors stay until closed and have **Copy**; duplicates merge and move to the newest; Undo
toasts; a cap on stacking; toasts show above every sheet and dialog (a small window of their own). The rules are
plain Kotlin in `ToastRules` (tested); they match kit-rules' toast rules on the web and desktop.

From Encore's `ui/components/Toasts.kt` and `ToastRules.kt`.
