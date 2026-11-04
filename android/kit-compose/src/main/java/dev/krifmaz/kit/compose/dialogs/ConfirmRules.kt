package dev.krifmaz.kit.compose.dialogs

/**
 * What a confirm dialog asks, as plain data: the question, what happens, the button's words, whether it is
 * dangerous, and (for steps that can't be undone) a word the person types first.
 *
 * @property title the question ("Delete this note?").
 * @property text what happens, in a sentence ("It can't be undone."); "" for none.
 * @property okLabel the confirm button's words ("Delete").
 * @property danger draws the confirm button in red.
 * @property typeToConfirm a word the person must type before the button works ("delete", a name); "" for none.
 * @property cancelLabel the dismiss button's words.
 */
data class Confirm(
    val title: String,
    val text: String = "",
    val okLabel: String = "OK",
    val danger: Boolean = false,
    val typeToConfirm: String = "",
    val cancelLabel: String = ConfirmRules.CANCEL,
)

/** The rules and words of a confirm dialog, without Android (the unit tests run them on the JVM). */
object ConfirmRules {
    /** The dismiss button's words. */
    const val CANCEL = "Cancel"

    /** The line over the field of a type-to-confirm dialog: `Type "delete" to confirm`. */
    fun typePrompt(word: String): String = "Type \"$word\" to confirm"

    /**
     * Whether the confirm button works: always without a word to type; otherwise once [typed] (spaces around it
     * left out) is the word exactly, case included.
     */
    fun canConfirm(c: Confirm, typed: String): Boolean = c.typeToConfirm.isEmpty() || typed.trim() == c.typeToConfirm
}
