package dev.krifmaz.kit.compose.uploads

import android.app.Activity
import android.content.ContentResolver
import android.content.Context
import android.content.ContextWrapper
import android.net.Uri
import android.provider.OpenableColumns
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.draganddrop.dragAndDropTarget
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTarget
import androidx.compose.ui.draganddrop.mimeTypes
import androidx.compose.ui.draganddrop.toAndroidDragEvent
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.R
import dev.krifmaz.kit.compose.theme.palette
import dev.krifmaz.kit.compose.toasts.Toasts
import dev.krifmaz.kit.compose.widgets.SmallButton

/**
 * A file picked or dropped on a [DropZone].
 *
 * @property uri where to read it (a content URI; a drop's stays readable while the activity lives).
 * @property name its display name ("" when the provider gives none).
 * @property mime its MIME type ("" when unknown).
 * @property size its size in bytes (-1 when unknown).
 */
data class PickedFile(val uri: Uri, val name: String, val mime: String, val size: Long)

/** Reads a content URI's name, type and size from its provider. */
fun pickedFileOf(resolver: ContentResolver, uri: Uri): PickedFile {
    var name = ""
    var size = -1L
    try {
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                val s = c.getColumnIndex(OpenableColumns.SIZE)
                if (n >= 0 && !c.isNull(n)) name = c.getString(n)
                if (s >= 0 && !c.isNull(s)) size = c.getLong(s)
            }
        }
    } catch (_: Exception) {
        // a provider that refuses the query: the name falls back to the URI's last part
    }
    if (name.isEmpty()) name = uri.lastPathSegment?.substringAfterLast('/') ?: ""
    return PickedFile(uri, name, resolver.getType(uri) ?: "", size)
}

private fun Context.activity(): Activity? = when (this) {
    is Activity -> this
    is ContextWrapper -> baseContext.activity()
    else -> null
}

/**
 * An upload area: files dragged from another app (split screen, a desktop window, ChromeOS) can be dropped on it, and
 * its button opens the system picker. Both go through the same [accept] rule ([UploadRules]); refused files get a
 * toast saying why, and [onFiles] gets the rest. While files are dragged over it the zone lights up.
 *
 * @param accept the upload's rule (`image/ *`, `.zip,application/zip`; "" for any file).
 * @param multiple whether it takes several files (else the first acceptable one).
 * @param onFiles the accepted files.
 * @param modifier the zone's modifier.
 * @param enabled false: a drop says [UploadRules.NOT_POSSIBLE] and the button waits.
 * @param label the words in the zone.
 * @param button the picker button's words.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun DropZone(
    accept: String,
    multiple: Boolean,
    onFiles: (List<PickedFile>) -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    label: String = if (multiple) "Drop files here" else "Drop a file here",
    button: String = if (multiple) "Choose files" else "Choose a file",
) {
    val p = MaterialTheme.palette
    val context = LocalContext.current
    var over by remember { mutableStateOf(false) }

    fun take(files: List<PickedFile>) {
        val (keep, refused) = UploadRules.choose(files, accept, multiple, { it.name }, { it.mime })
        val msg = UploadRules.refusedMessage(refused, accept, multiple, keep.size)
        if (msg.isNotEmpty()) Toasts.show(msg, UploadRules.refusedKind(keep.size))
        if (keep.isNotEmpty()) onFiles(keep)
    }
    val types = accept.split(',').map { it.trim() }.filter { it.contains('/') }.ifEmpty { listOf("*/*") }.toTypedArray()
    val pickMany = rememberLauncherForActivityResult(ActivityResultContracts.OpenMultipleDocuments()) { uris ->
        take(uris.map { pickedFileOf(context.contentResolver, it) })
    }
    val pickOne = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) take(listOf(pickedFileOf(context.contentResolver, uri)))
    }
    val target = remember(accept, multiple, enabled) {
        object : DragAndDropTarget {
            override fun onEntered(event: DragAndDropEvent) {
                over = true
            }

            override fun onExited(event: DragAndDropEvent) {
                over = false
            }

            override fun onEnded(event: DragAndDropEvent) {
                over = false
            }

            override fun onDrop(event: DragAndDropEvent): Boolean {
                over = false
                if (!enabled) {
                    Toasts.show(UploadRules.NOT_POSSIBLE, "err")
                    return false
                }
                val drag = event.toAndroidDragEvent()
                // another app's content URIs are readable only with the drop's permission
                context.activity()?.requestDragAndDropPermissions(drag)
                val clip = drag.clipData ?: return false
                val files = (0 until clip.itemCount).mapNotNull { clip.getItemAt(it).uri }.map { pickedFileOf(context.contentResolver, it) }
                take(files)
                return files.isNotEmpty()
            }
        }
    }
    val shape = RoundedCornerShape(12.dp)
    Column(
        modifier
            .fillMaxWidth()
            .testTag("kit-drop-zone")
            .dragAndDropTarget(shouldStartDragAndDrop = { e -> e.mimeTypes().isNotEmpty() }, target = target)
            .background(if (over) p.accent.copy(alpha = 0.10f) else p.bg2, shape)
            .border(if (over) 2.dp else 1.dp, if (over) p.accent else p.border, shape)
            .padding(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(label, style = MaterialTheme.typography.bodyMedium, color = if (over) p.accent else p.muted, textAlign = TextAlign.Center)
        SmallButton(button, iconRes = R.drawable.kit_ic_upload, enabled = enabled) {
            if (multiple) pickMany.launch(types) else pickOne.launch(types)
        }
    }
}
