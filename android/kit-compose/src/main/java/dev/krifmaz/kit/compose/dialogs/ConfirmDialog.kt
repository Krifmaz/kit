package dev.krifmaz.kit.compose.dialogs

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.krifmaz.kit.compose.theme.KitStatusColors
import dev.krifmaz.kit.compose.theme.palette

/**
 * A confirm dialog: the question, what happens, Cancel and the confirm button ([Confirm.okLabel], red when
 * [Confirm.danger]). With [Confirm.typeToConfirm] the button works only once that word is typed, for steps that
 * can't be undone. Confirming closes it ([onDismiss]) and then runs [onConfirm].
 *
 * @param confirm what it asks.
 * @param onDismiss closes it (Cancel, outside, back).
 * @param onConfirm runs after the person confirmed.
 */
@Composable
fun ConfirmDialog(confirm: Confirm, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    val p = MaterialTheme.palette
    var typed by remember(confirm) { mutableStateOf("") }
    val ok = ConfirmRules.canConfirm(confirm, typed)
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = p.panel,
        titleContentColor = p.text,
        textContentColor = p.muted,
        title = { Text(confirm.title) },
        text = {
            Column {
                if (confirm.text.isNotEmpty()) Text(confirm.text)
                if (confirm.typeToConfirm.isNotEmpty()) {
                    if (confirm.text.isNotEmpty()) Spacer(Modifier.height(12.dp))
                    Text(ConfirmRules.typePrompt(confirm.typeToConfirm), style = MaterialTheme.typography.bodySmall)
                    OutlinedTextField(
                        typed,
                        { typed = it },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth().testTag("kit-confirm-typed"),
                    )
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    onDismiss()
                    onConfirm()
                },
                enabled = ok,
                modifier = Modifier.testTag("kit-confirm-ok"),
            ) {
                Text(
                    confirm.okLabel,
                    color = if (!ok) {
                        p.faint
                    } else if (confirm.danger) {
                        KitStatusColors.Err
                    } else {
                        p.text
                    },
                    fontWeight = FontWeight.SemiBold,
                )
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(confirm.cancelLabel, color = p.muted) } },
    )
}

/**
 * A confirm dialog by its words: [title], [text], the [okLabel] button (red when [danger]) and Cancel.
 *
 * @param title the question.
 * @param text what happens; "" for none.
 * @param okLabel the confirm button's words.
 * @param danger draws the confirm button in red.
 * @param onDismiss closes it.
 * @param onConfirm runs after the person confirmed.
 */
@Composable
fun ConfirmDialog(title: String, text: String, okLabel: String, danger: Boolean = false, onDismiss: () -> Unit, onConfirm: () -> Unit) =
    ConfirmDialog(Confirm(title, text, okLabel, danger), onDismiss, onConfirm)

/**
 * Confirm dialogs asked from anywhere in a screen: `confirms.ask(Confirm("Delete?", okLabel = "Delete")) { delete() }`
 * with one [ConfirmHost] drawing the open one. Make it with [rememberConfirms].
 */
@Stable
class Confirms {
    private var open by mutableStateOf<Pair<Confirm, () -> Unit>?>(null)

    /** The dialog showing now, or null. */
    val showing: Confirm? get() = open?.first

    /** Shows [confirm]; [then] runs if the person confirms. A dialog already open is replaced. */
    fun ask(confirm: Confirm, then: () -> Unit) {
        open = confirm to then
    }

    /** Closes the open dialog without running its action. */
    fun dismiss() {
        open = null
    }

    /** The open dialog's action. */
    internal val action: (() -> Unit)? get() = open?.second
}

/** A [Confirms] that lasts while the screen is composed. */
@Composable
fun rememberConfirms(): Confirms = remember { Confirms() }

/** Draws the dialog [confirms] has open, if any; put it once in the screen. */
@Composable
fun ConfirmHost(confirms: Confirms) {
    val c = confirms.showing ?: return
    // taken now: confirming closes the dialog (which forgets it) before the action runs
    val action = confirms.action
    ConfirmDialog(c, onDismiss = { confirms.dismiss() }, onConfirm = { action?.invoke() })
}
