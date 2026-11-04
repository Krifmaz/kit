# core

Plain helpers the other packages share:

- `Json.kt`: the lenient JSON layer (`KitJson`, readers like `str` / `bool` / `long`, `L…` field types and
  `Items` for `@Serializable` models), so a missing or odd field never fails a whole answer.
- `Format.kt`: the web's `fmt.size`, `fmt.day` and `fmt.ago`.
- `DownloadsFolder.kt`: saves an export (bug report, crash log) straight into the phone's Downloads.

From Encore's `core/` and its formatting helpers.
