# uploads

Drag and drop, and the file picker, with the same checks as the web's drop zones.

- `UploadRules` — whether a file matches an `accept` list (`image/` with a star, `.pdf`, `text/plain`), which files
  are kept and the toast for the rest ("Only one file can be uploaded here: the first was used"). Unit-tested.
- `FileLimits(maxFiles, maxBytes, per)` — how many and how large; `add` says which of the new files fit and the
  toasts for the others; `FileLimits.BUG_REPORT` is the bug report sheet's 6 pictures of 8 MB.
- `DropZone(accept, multiple, onFiles)` — a dashed box that takes files dragged from another app (split screen,
  desktop mode, ChromeOS) and has a **Choose** button for the system picker. Each file arrives as a `PickedFile`
  (uri, name, mime, size); `pickedFileOf(resolver, uri)` reads one.

```kotlin
DropZone(accept = "image/*", multiple = true, onFiles = { files -> attach(files) })
```
