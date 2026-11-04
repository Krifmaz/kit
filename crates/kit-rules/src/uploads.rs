//! Uploads: which files an upload takes, checked the same way for the file picker and drag & drop, and the
//! words for those it refuses.
//!
//! From Encore's `web-rs/src/dropzone.rs` (`accepts`, `accept_hint`, `choose`, `refused_message`, the drop's
//! words), the desktop's `clients/desktop/src/widgets/drop_zone.rs` (`Accept`, `DropZone`, `SAME_DROP`) and
//! the bug report's limits in `web-rs/src/pages/bugreports.rs` (`MAX_FILES`, `MAX_BYTES`, `add`) and Android's
//! `features/bugs/BugRules.kt` / `BugAttachments.kt`.
//!
//! The web reads an input's `accept` attribute (`.zip,application/zip`, `image/*`); the desktop's file dialogs
//! know only endings ([`Accept`]). Both refuse with the same words.

/// The class every drop zone on screen carries while files are dragged over the window. From `dropzone.rs`
/// `READY_CLASS`.
pub const READY_CLASS: &str = "drop-ready";
/// The class of the zone under the pointer while files are dragged over it. From `dropzone.rs` `OVER_CLASS`.
pub const OVER_CLASS: &str = "drop-over";
/// The toast for a drop on an upload that can't be used right now. From `dropzone.rs` `init`.
pub const NOT_POSSIBLE: &str = "Uploading isn't possible right now";
/// The toast when an upload of one file got several. From `dropzone.rs` `refused_message` and the desktop's
/// `DropZone::handle_at`.
pub const ONLY_ONE: &str = "Only one file can be uploaded here: the first was used";
/// Files dropped within this long of the one before belong to the same drop (the desktop gets one event per
/// file). From the desktop's `drop_zone.rs` `SAME_DROP`.
pub const SAME_DROP_MS: u64 = 800;

/// The words for one refused file: "notes.txt can't be uploaded here (it takes .zip)" (without the brackets
/// when `hint` is empty).
///
/// From `dropzone.rs` `refused_message` and the desktop's `DropZone::handle_at`.
pub fn refused_one(name: &str, hint: &str) -> String {
    if hint.is_empty() {
        format!("{name} can't be uploaded here")
    } else {
        format!("{name} can't be uploaded here (it takes {hint})")
    }
}

/// Whether a file with this `name` and MIME type `mime` passes an input's `accept` attribute, the way the
/// browser's file chooser reads it: a comma separated list of `.ext` (the name's ending, any case), `type/*`
/// (any subtype) and `type/subtype` (exactly). An empty `accept` takes every file; a file without a type (`""`)
/// only passes on its name.
///
/// From `dropzone.rs` `accepts`.
pub fn accepts(accept: &str, name: &str, mime: &str) -> bool {
    let tokens: Vec<String> = accept.split(',').map(|t| t.trim().to_ascii_lowercase()).filter(|t| !t.is_empty()).collect();
    if tokens.is_empty() {
        return true;
    }
    let (name, mime) = (name.to_ascii_lowercase(), mime.trim().to_ascii_lowercase());
    tokens.iter().any(|t| {
        if t.starts_with('.') {
            name.ends_with(t.as_str())
        } else if let Some(major) = t.strip_suffix("/*") {
            mime.split_once('/').is_some_and(|(m, _)| m == major)
        } else {
            !mime.is_empty() && mime == *t
        }
    })
}

/// The extensions an `accept` names, for the words about a refused file (".zip, .db"; "" when it names only
/// types such as `image/*`).
///
/// From `dropzone.rs` `accept_hint`.
pub fn accept_hint(accept: &str) -> String {
    let exts: Vec<&str> = accept.split(',').map(str::trim).filter(|t| t.starts_with('.')).collect();
    exts.join(", ")
}

/// Splits a drop by an input's rule: (the files to hand over, the names of those refused). Without `multiple`
/// only the first acceptable file is kept and every other one counts as refused.
///
/// From `dropzone.rs` `choose`.
pub fn choose<T>(
    files: Vec<T>,
    accept: &str,
    multiple: bool,
    name: impl Fn(&T) -> String,
    mime: impl Fn(&T) -> String,
) -> (Vec<T>, Vec<String>) {
    let (mut keep, mut refused) = (vec![], vec![]);
    for f in files {
        if accepts(accept, &name(&f), &mime(&f)) && (multiple || keep.is_empty()) {
            keep.push(f);
        } else {
            refused.push(name(&f));
        }
    }
    (keep, refused)
}

/// The toast after a drop where some files were refused ("" when none were). Its kind on the web: `err` when
/// nothing was kept, else a plain toast ([`refused_kind`]).
///
/// From `dropzone.rs` `refused_message`.
pub fn refused_message(refused: &[String], accept: &str, multiple: bool, kept: usize) -> String {
    if refused.is_empty() {
        return String::new();
    }
    let hint = accept_hint(accept);
    let takes = if hint.is_empty() { String::new() } else { format!(" (it takes {hint})") };
    match (kept, refused) {
        // one file wanted and one used: the rest were extras
        (1, _) if !multiple => ONLY_ONE.to_string(),
        (_, [one]) => format!("{one} can't be uploaded here{takes}"),
        (_, many) => format!("{} files can't be uploaded here{takes}", many.len()),
    }
}

/// The kind of [`refused_message`]'s toast: an error when nothing was kept, else plain.
///
/// From `dropzone.rs` `drop_on`.
pub fn refused_kind(kept: usize) -> crate::toasts::ToastKind {
    if kept == 0 {
        crate::toasts::ToastKind::Err
    } else {
        crate::toasts::ToastKind::Plain
    }
}

/// The ending of a file name the way `std::path::Path::extension` reads it: after the last dot, unless the dot
/// starts the name (`.zip` has none, `a.tar.gz` has `gz`).
fn extension(name: &str) -> Option<&str> {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    if file == ".." {
        return None;
    }
    let i = file.rfind('.')?;
    (i > 0).then(|| &file[i + 1..])
}

/// Which files an upload takes when the file dialog knows only endings (the desktop): the endings (lower case,
/// without the dot; empty takes any file) and whether one drop may hand over several files.
///
/// From the desktop's `widgets/drop_zone.rs` `Accept`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accept {
    /// The file endings taken, e.g. `["zip"]`; empty takes any file.
    pub exts: &'static [&'static str],
    /// Whether one drop may hand over several files.
    pub multiple: bool,
}

impl Accept {
    /// One file with one of these endings.
    pub const fn one(exts: &'static [&'static str]) -> Self {
        Accept { exts, multiple: false }
    }

    /// Any number of files with these endings.
    pub const fn many(exts: &'static [&'static str]) -> Self {
        Accept { exts, multiple: true }
    }

    /// Whether a file called `name` (a name or a path) has an ending the upload takes (any case). Folders
    /// (`is_dir`) are never taken.
    pub fn takes(&self, name: &str, is_dir: bool) -> bool {
        if is_dir {
            return false;
        }
        if self.exts.is_empty() {
            return true;
        }
        let ext = extension(name).unwrap_or("").to_ascii_lowercase();
        self.exts.iter().any(|e| e.eq_ignore_ascii_case(&ext))
    }

    /// The endings for the words (".zip", ".png, .jpg"); "" when any file is taken.
    pub fn hint(&self) -> String {
        self.exts.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(", ")
    }

    /// The same rule as an input's `accept` attribute (".zip,.db"), so the web's picker and drop check it the
    /// same way.
    pub fn to_accept_attr(&self) -> String {
        self.exts.iter().map(|e| format!(".{e}")).collect::<Vec<_>>().join(",")
    }
}

/// What to do with one dropped file.
///
/// From the desktop's `widgets/drop_zone.rs` `Outcome`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome<P> {
    /// Nothing (only the highlight changed).
    Nothing,
    /// Upload this file, as if it had been picked in the file dialog.
    Take(P),
    /// The file was not taken: show these words.
    Refused(String),
}

/// A drop place that gets one event per file (the desktop's window): whether files are held over it, and how
/// many files the current drop already handed over (an upload of one file takes only the first).
///
/// From the desktop's `widgets/drop_zone.rs` `DropZone`.
#[derive(Debug, Default, Clone)]
pub struct DropZone {
    /// Files are held over the window: draw the highlight.
    pub hovering: bool,
    taken: usize,
    last_drop: Option<u64>,
}

impl DropZone {
    /// Files are held over the window (once per file); a new drag starts counting again.
    pub fn hovered(&mut self) {
        if !self.hovering {
            self.taken = 0;
            self.last_drop = None;
        }
        self.hovering = true;
    }

    /// They left the window without being dropped.
    pub fn left(&mut self) {
        self.hovering = false;
    }

    /// One file was dropped at `now_ms`: `item` (a path), its file `name` and whether it is a folder. Some
    /// systems send no "hovered" first, so a pause of more than [`SAME_DROP_MS`] also starts a new drop.
    pub fn dropped<P>(&mut self, item: P, name: &str, is_dir: bool, accept: &Accept, now_ms: u64) -> Outcome<P> {
        self.hovering = false;
        if self.last_drop.is_none_or(|t| now_ms.saturating_sub(t) > SAME_DROP_MS) {
            self.taken = 0;
        }
        self.last_drop = Some(now_ms);
        let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
        if !accept.takes(name, is_dir) {
            return Outcome::Refused(refused_one(file, &accept.hint()));
        }
        if !accept.multiple && self.taken > 0 {
            return Outcome::Refused(ONLY_ONE.into());
        }
        self.taken += 1;
        Outcome::Take(item)
    }
}

/// How many files an upload holds and how big each may be (the bug report's attachments).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileLimits {
    /// Files per report or message.
    pub max_files: usize,
    /// Bytes per file.
    pub max_bytes: u64,
}

impl FileLimits {
    /// The bug report's: 6 files, 8 MiB each. From `web-rs/src/pages/bugreports.rs` `MAX_FILES` / `MAX_BYTES`,
    /// `BugRules.MAX_FILES` / `MAX_FILE_BYTES` and `contracts/bug-reports.md`.
    pub const BUG_REPORT: FileLimits = FileLimits { max_files: 6, max_bytes: 8 * 1024 * 1024 };

    /// The size in the words, in whole MB ("8 MB" for 8 MiB).
    fn mb(&self) -> u64 {
        self.max_bytes / (1024 * 1024)
    }

    /// The toast when there are already `max_files`: "At most 6 files per report". From `bugreports.rs` `add`.
    pub fn too_many(&self) -> String {
        format!("At most {} files per report", self.max_files)
    }

    /// The toast for a file over the size: "photo.png is over 8 MB". From `bugreports.rs` `add` and
    /// `BugAttachments.kt`.
    pub fn too_big(&self, name: &str) -> String {
        format!("{name} is over {} MB", self.mb())
    }

    /// The line under the picker: "Up to 6 files, 8 MB each.". From `bugreports.rs` (the form) and
    /// `NewBugReportSheet.kt`.
    pub fn summary(&self) -> String {
        format!("Up to {} files, {} MB each.", self.max_files, self.mb())
    }

    /// Adds picked or dropped files (`(name, size)` each) to the `have` already there: the indexes of those
    /// taken, and the toasts (errors) for the rest. Once full it says [`FileLimits::too_many`] and stops; a file
    /// over the size is left out with [`FileLimits::too_big`].
    ///
    /// From `web-rs/src/pages/bugreports.rs` `add`.
    pub fn add(&self, have: usize, files: &[(&str, u64)]) -> (Vec<usize>, Vec<String>) {
        let (mut taken, mut said) = (vec![], vec![]);
        for (i, (name, size)) in files.iter().enumerate() {
            if have + taken.len() >= self.max_files {
                said.push(self.too_many());
                break;
            }
            if *size > self.max_bytes {
                said.push(self.too_big(name));
                continue;
            }
            taken.push(i);
        }
        (taken, said)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the web (web-rs/src/dropzone.rs tests) --------------------------------------------------------------

    #[test]
    fn empty_accept_takes_anything() {
        assert!(accepts("", "song.flac", "audio/flac"));
        assert!(accepts(" , ", "x", ""));
    }

    #[test]
    fn extensions_match_the_end_of_the_name_in_any_case() {
        assert!(accepts(".zip,application/zip", "Plugin.ZIP", ""));
        assert!(accepts(".db,.sqlite,.sqlite3", "encore-2026.sqlite3", "application/octet-stream"));
        assert!(!accepts(".db", "encore.db.bak", ""));
        assert!(accepts(".apk,.apkm", "AppleMusic.apkm", ""));
    }

    #[test]
    fn wildcard_and_exact_types() {
        assert!(accepts("image/*", "me.webp", "image/webp"));
        assert!(!accepts("image/*", "me.txt", "text/plain"));
        assert!(accepts("image/jpeg,image/png", "c.png", "image/png"));
        assert!(!accepts("image/jpeg,image/png", "c.gif", "image/gif"));
        assert!(!accepts("image/*", "c.heic", ""));
        assert!(accepts("image/*,.txt,.log", "server.log", ""));
    }

    #[test]
    fn single_inputs_keep_only_the_first_good_file() {
        let files = vec![("a.txt", "text/plain"), ("b.zip", "application/zip"), ("c.zip", "application/zip")];
        let (keep, refused) = choose(files, ".zip", false, |f| f.0.to_string(), |f| f.1.to_string());
        assert_eq!(keep, vec![("b.zip", "application/zip")]);
        assert_eq!(refused, vec!["a.txt", "c.zip"]);
    }

    #[test]
    fn multiple_inputs_keep_every_good_file() {
        let files = vec!["a.png", "b.exe", "c.log"];
        let (keep, refused) = choose(
            files,
            "image/*,.txt,.log",
            true,
            |f| f.to_string(),
            |f| if f.ends_with("png") { "image/png".into() } else { String::new() },
        );
        assert_eq!(keep, vec!["a.png", "c.log"]);
        assert_eq!(refused, vec!["b.exe"]);
    }

    #[test]
    fn messages_about_refused_files() {
        assert_eq!(refused_message(&[], ".zip", false, 1), "");
        assert_eq!(refused_message(&["a.txt".into()], ".zip,application/zip", false, 0), "a.txt can't be uploaded here (it takes .zip)");
        assert_eq!(refused_message(&["a".into(), "b".into()], "image/*", true, 0), "2 files can't be uploaded here");
        assert_eq!(refused_message(&["b.zip".into()], ".zip", false, 1), "Only one file can be uploaded here: the first was used");
        assert_eq!(refused_kind(0), crate::toasts::ToastKind::Err);
        assert_eq!(refused_kind(1), crate::toasts::ToastKind::Plain);
    }

    // ---- the desktop (clients/desktop/src/widgets/drop_zone.rs tests) -------------------------------------------

    const ZIP: Accept = Accept::one(&["zip"]);
    const PICS: Accept = Accept::many(&["png", "jpg", "txt", "log"]);

    #[test]
    fn endings_are_checked_in_any_case() {
        assert!(ZIP.takes("/tmp/Plugin.ZIP", false));
        assert!(!ZIP.takes("/tmp/plugin.tar.gz", false));
        assert!(!ZIP.takes("/tmp/zip", false));
        assert!(!ZIP.takes("/tmp/.zip", false), "a dot that starts the name is no ending");
        assert!(Accept::one(&[]).takes("/tmp/anything.bin", false));
        assert_eq!(PICS.hint(), ".png, .jpg, .txt, .log");
        assert_eq!(PICS.to_accept_attr(), ".png,.jpg,.txt,.log");
    }

    #[test]
    fn folders_are_never_taken() {
        assert!(!Accept::one(&[]).takes("/tmp", true));
    }

    #[test]
    fn hover_lights_up_and_leaving_turns_it_off() {
        let mut z = DropZone::default();
        z.hovered();
        assert!(z.hovering);
        z.left();
        assert!(!z.hovering);
    }

    #[test]
    fn a_single_upload_takes_the_first_good_file_of_a_drop() {
        let mut z = DropZone::default();
        z.hovered();
        assert_eq!(
            z.dropped("/a/notes.txt", "/a/notes.txt", false, &ZIP, 0),
            Outcome::Refused("notes.txt can't be uploaded here (it takes .zip)".into())
        );
        assert_eq!(z.dropped("/a/one.zip", "/a/one.zip", false, &ZIP, 0), Outcome::Take("/a/one.zip"));
        assert_eq!(z.dropped("/a/two.zip", "/a/two.zip", false, &ZIP, 0), Outcome::Refused(ONLY_ONE.into()));
        assert!(!z.hovering);
        z.hovered();
        assert_eq!(z.dropped("/a/two.zip", "two.zip", false, &ZIP, 0), Outcome::Take("/a/two.zip"));
    }

    #[test]
    fn a_pause_starts_a_new_drop_without_a_hover() {
        let mut z = DropZone::default();
        assert_eq!(z.dropped(1, "one.zip", false, &ZIP, 0), Outcome::Take(1));
        assert_eq!(z.dropped(2, "two.zip", false, &ZIP, SAME_DROP_MS), Outcome::Refused(ONLY_ONE.into()), "within the same drop");
        assert_eq!(z.dropped(3, "three.zip", false, &ZIP, 2 * SAME_DROP_MS + 1), Outcome::Take(3));
    }

    #[test]
    fn a_multiple_upload_takes_every_good_file() {
        let mut z = DropZone::default();
        z.hovered();
        for f in ["/a/1.png", "/a/2.JPG", "/a/log.log"] {
            assert_eq!(z.dropped(f, f, false, &PICS, 0), Outcome::Take(f));
        }
        assert!(matches!(z.dropped("x", "/a/x.exe", false, &PICS, 0), Outcome::Refused(_)));
    }

    #[test]
    fn the_desktop_and_the_web_refuse_in_the_same_words() {
        let mut z = DropZone::default();
        let Outcome::Refused(desktop) = z.dropped((), "notes.txt", false, &ZIP, 0) else { panic!() };
        assert_eq!(desktop, refused_message(&["notes.txt".into()], &ZIP.to_accept_attr(), false, 0));
        assert_eq!(refused_one("x.bin", ""), "x.bin can't be uploaded here");
    }

    // ---- the bug report's limits -----------------------------------------------------------------------------

    #[test]
    fn a_report_takes_six_files_of_up_to_8_mb() {
        let l = FileLimits::BUG_REPORT;
        assert_eq!(l.summary(), "Up to 6 files, 8 MB each.");
        let big = 8 * 1024 * 1024 + 1;
        let (taken, said) = l.add(4, &[("a.png", 10), ("huge.png", big), ("b.png", 10), ("c.png", 10)]);
        assert_eq!(taken, vec![0, 2]);
        assert_eq!(said, vec!["huge.png is over 8 MB".to_string(), "At most 6 files per report".to_string()]);
        let (taken, said) = l.add(0, &[("x.png", 8 * 1024 * 1024)]);
        assert_eq!((taken, said.len()), (vec![0], 0), "exactly 8 MB is fine");
    }
}
