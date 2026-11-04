# images

A Coil 3 setup for any app:

- **`buildKitImageLoader(app, http)`** — images go through the app's OkHttp client (session, headers), ask for WebP,
  use 20 % of memory and a 250 MB disk cache (`cacheDir/images`), follow the server's `Cache-Control` / ETag
  headers, and de-duplicate concurrent requests for the same URL. Every size and folder is a parameter.
- **`ImagePreloader(context, loader)`** — fetches URLs ahead of need into the disk cache only, each once, and nothing
  on metered networks. The app maps its ids to URLs (a gallery's thumbnails, a list's pictures).
- **`PreloadAhead(state, count, keyAt, fetch)`** — for a `LazyGridState` or `LazyListState`: the keys 3 screens ahead
  in the scroll direction, and around the spot after a jump (**`PreloadRules.ahead`**, tested).

Smoothness rules this serves: images decoded at display size (ask the server for the size you draw) and preloaded
ahead of the scroll.
