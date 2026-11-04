# details

`DetailsDialog(title, text, onDismiss)`: the whole of a cut-off text (an error, a log line), wrapped and
selectable, with **Copy** and Close. `CopyIconButton` and `copyToClipboard` put text on the clipboard with the
same toasts ("Copied to clipboard", or "Copy failed – select and copy manually").

From Encore's `ui/components/Details.kt` (and `copyToClipboard` from its Settings widgets).
