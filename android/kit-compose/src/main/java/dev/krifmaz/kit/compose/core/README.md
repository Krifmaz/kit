# core

Plain helpers the other packages share:

- `Json.kt`: the lenient JSON layer (`KitJson`, readers like `str` / `bool` / `long`, `L…` field types and
  `Items` for `@Serializable` models), so a missing or odd field never fails a whole answer.
- `Format.kt`: the web's `fmt.size`, `fmt.day` and `fmt.ago`.
- `DownloadsFolder.kt`: saves an export (bug report, crash log) straight into the phone's Downloads.
- `Api.kt`: `KitApi`, the app's server calls as the kit's parts need them (`url`, `get`, `post`, `put`,
  `rawGet`), and `ApiException` with the server's `detail`. `put` has a default that refuses, for apps whose kit
  parts never PUT.
