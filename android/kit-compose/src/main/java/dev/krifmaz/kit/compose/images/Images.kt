package dev.krifmaz.kit.compose.images

import android.app.Application
import android.content.Context
import android.net.ConnectivityManager
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.grid.LazyGridState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import coil3.ImageLoader
import coil3.annotation.ExperimentalCoilApi
import coil3.request.CachePolicy
import coil3.request.ImageRequest
import kotlinx.coroutines.flow.distinctUntilChanged
import okhttp3.OkHttpClient
import java.io.File
import kotlin.math.abs

/**
 * An image loader built the way Encore builds its cover loader: images go through the app's [http] client (so they
 * carry the session and the app's headers), ask for WebP, keep [memoryPercent] of the app's memory for decoded images
 * and [diskBytes] on disk in [diskDir], follow the server's cache headers (immutable versioned URLs, ETag revalidation
 * for the rest), and share one download when a preload and the view on screen ask for the same image.
 *
 * @param app the application.
 * @param http the app's OkHttp client; its own HTTP cache is turned off here (the disk cache holds the images).
 * @param diskDir the disk cache's folder (Encore: cacheDir/covers/images).
 * @param diskBytes the disk cache's size (Encore: 250 MB).
 * @param memoryPercent the share of the app's memory for decoded images (Encore: 0.20).
 * @param crossfadeMs the fade-in of a loaded image (Encore: 120 ms).
 */
@OptIn(ExperimentalCoilApi::class)
fun buildKitImageLoader(
    app: Application,
    http: OkHttpClient,
    diskDir: File = File(app.cacheDir, "covers/images"),
    diskBytes: Long = 250L * 1024 * 1024,
    memoryPercent: Double = 0.20,
    crossfadeMs: Int = 120,
): ImageLoader {
    // no OkHttp cache underneath: Coil's disk cache holds the images
    val images =
        http
            .newBuilder()
            .cache(null)
            .addInterceptor { chain ->
                val r = chain.request()
                chain.proceed(if (r.header("Accept") == null) r.newBuilder().header("Accept", "image/webp,image/*;q=0.8").build() else r)
            }.build()
    return ImageLoader
        .Builder(app)
        .components {
            add(
                OkHttpNetworkFetcherFactory(
                    callFactory = { images },
                    cacheStrategy = { CacheControlCacheStrategy() },
                    // a preload and the image on screen asking for the same URL share one download
                    concurrentRequestStrategy = { DeDupeConcurrentRequestStrategy() },
                ),
            )
        }.memoryCache { MemoryCache.Builder().maxSizePercent(app, memoryPercent).build() }
        .diskCache { DiskCache.Builder().directory(diskDir).maxSizeBytes(diskBytes).build() }
        .crossfade(crossfadeMs)
        .build()
}

/**
 * Fetches images ahead of need, to the disk cache only (so images on screen keep the memory cache), each URL once
 * (the last 2000 remembered), and nothing on metered networks. The app turns its own ids into URLs and calls [urls]
 * or [url] (Encore's CoverPreloader does this for album covers).
 *
 * @param context any context (the application's is used).
 * @param loader the image loader the screens use, so the preloaded images are in its caches.
 */
class ImagePreloader(context: Context, private val loader: ImageLoader) {
    private val context: Context = context.applicationContext
    private val done = java.util.Collections.synchronizedSet(LinkedHashSet<String>())

    /** Whether the active network is metered (or unknown): then nothing is preloaded. */
    fun metered(): Boolean = try {
        (context.getSystemService(Context.CONNECTIVITY_SERVICE) as? ConnectivityManager)?.isActiveNetworkMetered ?: true
    } catch (_: Exception) {
        true
    }

    /** Preloads up to [max] of [urls] (none on a metered network). */
    fun urls(urls: List<String>, max: Int = MAX_AHEAD) {
        if (urls.isEmpty() || metered()) return
        urls.take(max).forEach(::url)
    }

    /** Preloads one http(s) URL, once; does not check the network (callers that loop check [metered] once). */
    fun url(url: String) {
        if (!url.startsWith("http") || !done.add(url)) return
        if (done.size > 2000) {
            synchronized(done) {
                done.iterator().let {
                    it.next()
                    it.remove()
                }
            }
        }
        loader.enqueue(
            ImageRequest
                .Builder(context)
                .data(url)
                .memoryCachePolicy(CachePolicy.DISABLED)
                .build(),
        )
    }

    /** The default caps. */
    companion object {
        /** Images of one loaded page (Encore: a page of albums). */
        const val MAX_PER_PAGE = 60

        /** Images ahead of a scroll. */
        const val MAX_AHEAD = 120
    }
}

/**
 * Calls [fetch] with the keys of the items 3 screens ahead of a grid in the direction it scrolls, and 2 screens both
 * ways after a jump (a scrubber). [keyAt] gives an item's key (null or "" while the item isn't loaded: it is
 * fetched once it loads).
 */
@Composable
fun PreloadAhead(state: LazyGridState, count: Int, keyAt: (Int) -> String?, fetch: (List<String>) -> Unit) =
    PreloadAhead({ state.firstVisibleItemIndex }, { state.layoutInfo.visibleItemsInfo.size }, count, keyAt, fetch)

/** [PreloadAhead] for a list. */
@Composable
fun PreloadAhead(state: LazyListState, count: Int, keyAt: (Int) -> String?, fetch: (List<String>) -> Unit) =
    PreloadAhead({ state.firstVisibleItemIndex }, { state.layoutInfo.visibleItemsInfo.size }, count, keyAt, fetch)

@Composable
private fun PreloadAhead(first: () -> Int, visible: () -> Int, count: Int, keyAt: (Int) -> String?, fetch: (List<String>) -> Unit) {
    val total by rememberUpdatedState(count)
    val idAt by rememberUpdatedState(keyAt)
    val onAhead by rememberUpdatedState(fetch)
    LaunchedEffect(Unit) {
        var prev = 0
        var dir = 1 // 1 down, -1 up, 0 just jumped
        snapshotFlow {
            val f = first()
            val v = visible().coerceAtLeast(1)
            if (f != prev) {
                dir = if (abs(f - prev) > v * 3) {
                    0
                } else if (f > prev) {
                    1
                } else {
                    -1
                }
                prev = f
            }
            PreloadRules.ahead(f, v, total, dir).mapNotNull { i -> idAt(i)?.takeIf { it.isNotEmpty() } }.distinct()
        }.distinctUntilChanged()
            .collect { ids -> onAhead(ids) }
    }
}

/** Which items to preload: pure, so it is tested on its own. */
object PreloadRules {
    /**
     * The indexes to preload when [first] is the first visible item, [visible] items show, the list has [count] and
     * it last moved in [dir] (1 down, -1 up, 0 a jump): 3 screens ahead, or 2 behind and 3 ahead after a jump.
     */
    fun ahead(first: Int, visible: Int, count: Int, dir: Int): IntRange = when (dir) {
        1 -> (first + visible) until minOf(count, first + visible + visible * 3)
        -1 -> maxOf(0, first - visible * 3) until first
        else -> maxOf(0, first - visible * 2) until minOf(count, first + visible * 3)
    }
}
