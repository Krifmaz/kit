//! Toasts: which stay, how they stack, how long each kind stays.
//!
//! Every platform keeps the same rules: an error (`err`) stays until it is closed and carries Copy, so a long error can be read in
//! full and pasted into a bug report – except [`COPY_FAILED`], which says what to do instead and goes like any
//! other toast; a toast with an action ("Undo") stays its own time and goes when its button is pressed.
//! What suits a platform (how many errors stay, whether other toasts are capped, what a repeat does, how long a
//! toast stays) is a [`ToastPolicy`], with one preset per platform: [`ToastPolicy::web`],
//! [`ToastPolicy::desktop`] and [`ToastPolicy::android`]. The words are [`ToastWords`] (an app may pass its own).
//!
//!
//! [`ToastStack`] is the state machine: [`ToastStack::add`], [`ToastStack::close`], [`ToastStack::expire`]
//! (a timer that ends, Android's way), [`ToastStack::tick`] (a clock that runs, the desktop's way),
//! [`ToastStack::take_action`] and [`ToastStack::newest`]. It holds no timers itself: the UI arms one per toast
//! from [`Toast::expires_at`] (or ticks), so it fits Leptos signals, Iced state and a Kotlin port alike.

/// The words when the clipboard refuses: that toast says what to do instead, so it has no Copy of its own and
/// goes like any other toast.
pub const COPY_FAILED: &str = "Copy failed – select and copy manually";
/// The toast after a copy that worked.
pub const COPIED: &str = "Copied to clipboard";
/// The error toast's Copy button.
pub const COPY: &str = "Copy";
/// The error toast's Close button (its title and label).
pub const CLOSE: &str = "Close";
/// The usual action label.
pub const UNDO: &str = "Undo";
/// How long an Undo toast stays by default.
pub const UNDO_MS: u64 = 6000;

/// The web's cap on error toasts on screen; a new one pushes out the oldest.
pub const WEB_ERROR_TOASTS: usize = 3;
/// The web's and the desktop's time on screen for a toast that is not an error (the web fades it from
/// 3800 ms and removes it at 4200 ms).
pub const WEB_SHOW_MS: u64 = 4200;
/// How long the web's fade takes before a toast goes (3800 ms → 4200 ms; an action toast fades at `ms - 400`).
pub const WEB_FADE_MS: u64 = 400;
/// The desktop's cap on toasts of every kind on screen (the oldest goes).
pub const DESKTOP_MAX_TOASTS: usize = 5;
/// Android's time on screen for a toast that is not an error (the web's 3.8 s plus its fade).
pub const ANDROID_SHOW_MS: u64 = 4000;
/// Android's guard against a runaway burst of errors.
pub const ANDROID_MAX_ERRORS: usize = 10;
/// Android's cap on other toasts.
pub const ANDROID_MAX_OTHERS: usize = 3;

/// The words toasts and copies say. [`ToastWords::DEFAULT`] holds the kit's; an app passes its own to change any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToastWords {
    /// The error toast's Copy button ([`COPY`]).
    pub copy: &'static str,
    /// The error toast's Close button ([`CLOSE`]).
    pub close: &'static str,
    /// The toast after a copy that worked ([`COPIED`]).
    pub copied: &'static str,
    /// The toast when the clipboard refuses ([`COPY_FAILED`]); it goes by itself like any other toast.
    pub copy_failed: &'static str,
    /// The usual action label ([`UNDO`]).
    pub undo: &'static str,
}

impl ToastWords {
    /// The kit's words.
    pub const DEFAULT: ToastWords = ToastWords { copy: COPY, close: CLOSE, copied: COPIED, copy_failed: COPY_FAILED, undo: UNDO };
}

impl Default for ToastWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A toast's kind, as the web's `toast(msg, kind)` names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ToastKind {
    /// `""`: a plain toast (the web's drop zone uses it when some files were taken).
    Plain,
    /// `"info"`.
    #[default]
    Info,
    /// `"ok"`.
    Ok,
    /// `"warn"`.
    Warn,
    /// `"err"`: stays until closed, with Copy (except [`COPY_FAILED`]).
    Err,
}

impl ToastKind {
    /// The kind as the CSS class and the Kotlin string name it (`""`, `"info"`, `"ok"`, `"warn"`, `"err"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ToastKind::Plain => "",
            ToastKind::Info => "info",
            ToastKind::Ok => "ok",
            ToastKind::Warn => "warn",
            ToastKind::Err => "err",
        }
    }

    /// The kind for one of those names; anything unknown is [`ToastKind::Plain`] (the web would draw it as
    /// a plain toast with that class).
    pub fn parse(s: &str) -> ToastKind {
        match s {
            "info" => ToastKind::Info,
            "ok" => ToastKind::Ok,
            "warn" => ToastKind::Warn,
            "err" => ToastKind::Err,
            _ => ToastKind::Plain,
        }
    }
}

/// Whether a toast with these words and kind stays until it is closed (and has Copy): an error, unless it is
/// [`COPY_FAILED`].
pub fn stays(text: &str, kind: ToastKind) -> bool {
    kind == ToastKind::Err && text != COPY_FAILED
}

/// The toast after a copy: [`COPIED`] (ok) when it worked, [`COPY_FAILED`] (err, which goes by itself) when not.
/// `copied` is the words for a copy that worked (a share link says what the link does instead).
pub fn copy_toast(ok: bool, copied: &str) -> (&str, ToastKind) {
    if ok {
        (copied, ToastKind::Ok)
    } else {
        (COPY_FAILED, ToastKind::Err)
    }
}

/// What a toast that arrives again does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repeats {
    /// The web and the desktop: the same error while it is on screen (a retry that failed the same way, a page
    /// that polls) is dropped, and the one on screen stays where it is. Other toasts are not merged.
    KeepStayingError,
    /// Android: any toast with the same words and kind moves to the newest place, keeps its id (its Close still
    /// works), takes the new action (an "Undo" undoes the latest) and starts its time again.
    MoveToNewest,
}

/// How a client stacks its toasts. Use a preset; the fields say exactly where the clients differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToastPolicy {
    /// At most this many staying errors; a new one pushes out the oldest.
    pub max_errors: usize,
    /// At most this many toasts that go by themselves (None: no cap).
    pub max_others: Option<usize>,
    /// At most this many toasts of every kind (None: no cap); the oldest goes, whatever its kind.
    pub max_total: Option<usize>,
    /// How long a toast that is not an error and has no action of its own stays, in ms.
    pub show_ms: u64,
    /// How long before it goes a toast starts to fade, in ms (0: no fade).
    pub fade_ms: u64,
    /// What a repeat does.
    pub repeats: Repeats,
    /// A toast with blank words is not shown at all.
    pub skip_blank: bool,
}

impl ToastPolicy {
    /// The web: three errors, no cap on other toasts, 4.2 s with a 0.4 s fade, a staying
    /// error that comes again is dropped, blank toasts are shown as they come.
    pub const fn web() -> Self {
        ToastPolicy {
            max_errors: WEB_ERROR_TOASTS,
            max_others: None,
            max_total: None,
            show_ms: WEB_SHOW_MS,
            fade_ms: WEB_FADE_MS,
            repeats: Repeats::KeepStayingError,
            skip_blank: false,
        }
    }

    /// The desktop: three errors, five
    /// toasts in all, 4.2 s without a fade, a staying error that comes again is dropped.
    pub const fn desktop() -> Self {
        ToastPolicy {
            max_errors: WEB_ERROR_TOASTS,
            max_others: None,
            max_total: Some(DESKTOP_MAX_TOASTS),
            show_ms: WEB_SHOW_MS,
            fade_ms: 0,
            repeats: Repeats::KeepStayingError,
            skip_blank: false,
        }
    }

    /// Android: ten errors, three other toasts, 4 s, any repeat moves to the newest place and
    /// starts its time again, blank toasts are not shown.
    pub const fn android() -> Self {
        ToastPolicy {
            max_errors: ANDROID_MAX_ERRORS,
            max_others: Some(ANDROID_MAX_OTHERS),
            max_total: None,
            show_ms: ANDROID_SHOW_MS,
            fade_ms: 0,
            repeats: Repeats::MoveToNewest,
            skip_blank: true,
        }
    }
}

/// A toast's button ("Undo") and what it carries for the app (a message, a callback id, a closure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToastAction<A> {
    /// The button's words.
    pub label: String,
    /// What the app does when it is pressed; handed back by [`ToastStack::take_action`].
    pub payload: A,
}

/// One toast on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toast<A = ()> {
    /// Names it (its Close, its place in the list); a repeat keeps it.
    pub id: u64,
    /// Goes up each time its words arrive: a timer armed for an older `seq` does not end it.
    pub seq: u64,
    /// The words, whole.
    pub text: String,
    /// Its kind.
    pub kind: ToastKind,
    /// Its button, if any.
    pub action: Option<ToastAction<A>>,
    /// When it arrived (this `seq`), in ms.
    pub at_ms: u64,
    /// How long it stays when it is not an error, in ms.
    pub life_ms: u64,
    /// It stays until its Close and has Copy (an error that is not the copy-failed words, see [`stays`]).
    pub staying: bool,
}

impl<A> Toast<A> {
    /// It stays until its Close and has Copy ([`stays`]).
    pub fn stays(&self) -> bool {
        self.staying
    }

    /// When it goes by itself (ms), or None for a staying error.
    pub fn expires_at(&self) -> Option<u64> {
        (!self.stays()).then(|| self.at_ms.saturating_add(self.life_ms))
    }

    /// When it starts to fade (ms) under `policy`, or None for a staying error. The web fades a toast 400 ms
    /// before it goes (`toast_action`: `ms.saturating_sub(400)`).
    pub fn fades_at(&self, policy: &ToastPolicy) -> Option<u64> {
        (!self.stays()).then(|| self.at_ms.saturating_add(self.life_ms.saturating_sub(policy.fade_ms)))
    }

    /// Whether it is fading at `now_ms` under `policy` (draw it with `opacity: 0`, as the web does).
    pub fn fading(&self, policy: &ToastPolicy, now_ms: u64) -> bool {
        policy.fade_ms > 0 && self.fades_at(policy).is_some_and(|t| now_ms >= t)
    }
}

/// The toasts on screen, oldest first (the newest is drawn nearest the edge the toasts come from, see
/// [`draw_order`]).
#[derive(Clone, Debug)]
pub struct ToastStack<A = ()> {
    policy: ToastPolicy,
    items: Vec<Toast<A>>,
    next: u64,
    copy_failed: String,
}

impl<A> ToastStack<A> {
    /// An empty stack under `policy`.
    pub fn new(policy: ToastPolicy) -> Self {
        ToastStack { policy, items: Vec::new(), next: 1, copy_failed: COPY_FAILED.to_string() }
    }

    /// The same stack with the app's words for a copy that failed (the one error that goes by itself, as
    /// [`COPY_FAILED`] does by default).
    pub fn with_copy_failed(mut self, words: &str) -> Self {
        self.copy_failed = words.to_string();
        self
    }

    /// The policy it stacks by.
    pub fn policy(&self) -> &ToastPolicy {
        &self.policy
    }

    /// The toasts, oldest first.
    pub fn items(&self) -> &[Toast<A>] {
        &self.items
    }

    /// No toast on screen.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// A toast with these words and kind (and its own `life_ms` with an action: an Undo toast stays as long
    /// as Undo can be pressed). Returns the toast's id (a repeat on Android keeps the old one), or None when
    /// nothing was added (a blank toast where those are skipped, or the same staying error again on the web
    /// and the desktop).
    pub fn add(
        &mut self,
        text: impl Into<String>,
        kind: ToastKind,
        action: Option<ToastAction<A>>,
        life_ms: Option<u64>,
        now_ms: u64,
    ) -> Option<u64> {
        let text = text.into();
        if self.policy.skip_blank && text.trim().is_empty() {
            return None;
        }
        let staying = kind == ToastKind::Err && text != self.copy_failed;
        let seq = self.next;
        self.next += 1;
        let life_ms = match (&action, life_ms) {
            (Some(_), Some(ms)) => ms,
            _ => self.policy.show_ms,
        };
        let id = match self.policy.repeats {
            Repeats::KeepStayingError => {
                if staying && self.items.iter().any(|t| t.stays() && t.text == text) {
                    return None;
                }
                seq
            }
            Repeats::MoveToNewest => match self.items.iter().position(|t| t.text == text && t.kind == kind) {
                Some(i) => self.items.remove(i).id,
                None => seq,
            },
        };
        self.items.push(Toast { id, seq, text, kind, action, at_ms: now_ms, life_ms, staying });
        // the cap of the new toast's own group: staying errors, or the toasts that go by themselves
        let cap = if staying { Some(self.policy.max_errors) } else { self.policy.max_others };
        if let Some(cap) = cap {
            while self.items.iter().filter(|t| t.stays() == staying).count() > cap {
                if let Some(i) = self.items.iter().position(|t| t.stays() == staying) {
                    self.items.remove(i);
                }
            }
        }
        if let Some(total) = self.policy.max_total {
            while self.items.len() > total {
                self.items.remove(0);
            }
        }
        // the toast just added may have been pushed out by a total cap of 0
        self.items.iter().any(|t| t.id == id).then_some(id)
    }

    /// A plain toast of `kind` with the policy's time (`toast(msg, kind)`).
    pub fn show(&mut self, text: impl Into<String>, kind: ToastKind, now_ms: u64) -> Option<u64> {
        self.add(text, kind, None, None, now_ms)
    }

    /// A toast with one button that stays `life_ms` (`toast_action(msg, label, ms, f)`), of kind ok.
    pub fn show_action(&mut self, text: impl Into<String>, label: impl Into<String>, payload: A, life_ms: u64, now_ms: u64) -> Option<u64> {
        self.add(text, ToastKind::Ok, Some(ToastAction { label: label.into(), payload }), Some(life_ms), now_ms)
    }

    /// Its Close (or a click on it): toast `id` goes, whatever its kind.
    pub fn close(&mut self, id: u64) {
        self.items.retain(|t| t.id != id);
    }

    /// The time of toast `id` as it arrived the `seq`th time is up: it goes, unless it is a staying error or
    /// came again since (a new time started then).
    pub fn expire(&mut self, id: u64, seq: u64) {
        self.items.retain(|t| t.id != id || t.seq != seq || t.stays());
    }

    /// The clock moved to `now_ms`: every toast whose time is up goes ([`Toast::expires_at`]). Returns whether
    /// anything went.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.expires_at().is_none_or(|end| now_ms < end));
        self.items.len() != before
    }

    /// The earliest time a toast goes by itself (to arm one timer), or None when every toast stays.
    pub fn next_deadline(&self) -> Option<u64> {
        self.items.iter().filter_map(Toast::expires_at).min()
    }

    /// Its button was pressed: the toast goes, and its action comes back to be run (once).
    pub fn take_action(&mut self, id: u64) -> Option<A> {
        let i = self.items.iter().position(|t| t.id == id)?;
        self.items.remove(i).action.map(|a| a.payload)
    }

    /// The newest arrival on screen (0 when none): when it goes up, a platform that draws toasts in a window of
    /// their own puts it back on top.
    pub fn newest(&self) -> u64 {
        self.items.iter().map(|t| t.seq).max().unwrap_or(0)
    }
}

/// The toasts in the order they are drawn, top to bottom. At the bottom the newest is nearest the bottom (the
/// stack's own order); at the `top` the newest is nearest the top, so a new toast always appears at the edge
/// the toasts come from.
pub fn draw_order<A>(items: &[Toast<A>], top: bool) -> Vec<&Toast<A>> {
    if top {
        items.iter().rev().collect()
    } else {
        items.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Android {
        s: ToastStack<u32>,
        n: u64,
    }
    impl Android {
        fn new() -> Self {
            Android { s: ToastStack::new(ToastPolicy::android()), n: 0 }
        }
        fn add(&mut self, m: &str, kind: ToastKind) -> Option<u64> {
            self.n += 1;
            self.s.show(m, kind, self.n)
        }
        fn messages(&self) -> Vec<&str> {
            self.s.items().iter().map(|t| t.text.as_str()).collect()
        }
    }

    #[test]
    fn errors_stay_and_other_toasts_go() {
        assert!(stays("Storage: os error 28 writing /data/report.pdf", ToastKind::Err));
        assert!(!stays("Added to the list", ToastKind::Ok));
        assert!(!stays("Saved", ToastKind::Info));
        assert!(!stays("Server is slow", ToastKind::Warn));
        // the clipboard refused: that toast goes like any other and has no Copy (as on the web)
        assert!(!stays(COPY_FAILED, ToastKind::Err));
        assert_eq!(COPY_FAILED, "Copy failed – select and copy manually");
    }

    #[test]
    fn a_new_error_stacks_without_hiding_the_old_one() {
        let mut a = Android::new();
        a.add("First failed", ToastKind::Err);
        a.add("Second failed", ToastKind::Err);
        assert_eq!(a.messages(), ["First failed", "Second failed"]);
        assert!(a.s.items().iter().all(Toast::stays));
        a.add("Copied to clipboard", ToastKind::Ok);
        assert_eq!(a.messages(), ["First failed", "Second failed", "Copied to clipboard"]);
    }

    #[test]
    fn the_time_runs_out_only_for_toasts_that_are_not_errors() {
        let mut a = Android::new();
        a.add("Failed", ToastKind::Err);
        let err = a.s.items()[0].clone();
        a.add("Added to the list", ToastKind::Ok);
        let ok = a.s.items()[1].clone();
        a.s.expire(ok.id, ok.seq);
        assert_eq!(a.messages(), ["Failed"]);
        // an error stays when its time would be up
        a.s.expire(err.id, err.seq);
        assert_eq!(a.messages(), ["Failed"]);
        // until its Close
        a.s.close(err.id);
        assert!(a.s.is_empty());
    }

    #[test]
    fn the_same_message_again_does_not_pile_up() {
        let mut a = Android::new();
        a.add("Failed", ToastKind::Err);
        let first = a.s.items()[0].clone();
        a.add("Other", ToastKind::Err);
        a.add("Failed", ToastKind::Err);
        // one "Failed", moved to the newest place, same id (its Close still works), a new seq (on top again)
        assert_eq!(a.messages(), ["Other", "Failed"]);
        let last = a.s.items().last().unwrap();
        assert_eq!(last.id, first.id);
        assert!(last.seq > first.seq);
        assert_eq!(last.seq, a.s.newest());
    }

    #[test]
    fn a_repeated_toast_starts_its_time_again() {
        let mut a = Android::new();
        a.add("Moved to the top", ToastKind::Ok);
        let old = a.s.items()[0].clone();
        a.add("Moved to the top", ToastKind::Ok);
        // the first time's timer ends: the toast stays, it came again since
        a.s.expire(old.id, old.seq);
        assert_eq!(a.messages(), ["Moved to the top"]);
        let again = a.s.items()[0].clone();
        assert_eq!(again.at_ms, 2, "its time starts again");
        a.s.expire(again.id, again.seq);
        assert!(a.s.is_empty());
    }

    #[test]
    fn errors_stay_in_a_burst_and_others_are_capped() {
        let mut a = Android::new();
        for i in 1..=4 {
            a.add(&format!("Error {i}"), ToastKind::Err);
        }
        // a fourth error doesn't take the first away: errors stay until their Close
        assert_eq!(a.messages(), ["Error 1", "Error 2", "Error 3", "Error 4"]);
        for i in 1..=5 {
            a.add(&format!("Note {i}"), ToastKind::Info);
        }
        // other toasts never take an error's place, and keep to their own cap
        assert_eq!(a.messages(), ["Error 1", "Error 2", "Error 3", "Error 4", "Note 3", "Note 4", "Note 5"]);
        assert_eq!(a.s.items().iter().filter(|t| !t.stays()).count(), ANDROID_MAX_OTHERS);
    }

    #[test]
    fn only_a_runaway_burst_of_errors_takes_the_oldest_away() {
        let mut a = Android::new();
        for i in 1..=ANDROID_MAX_ERRORS + 1 {
            a.add(&format!("Error {i}"), ToastKind::Err);
        }
        assert_eq!(a.s.items().len(), ANDROID_MAX_ERRORS);
        assert_eq!(a.s.items()[0].text, "Error 2");
        assert_eq!(a.s.items().last().unwrap().text, format!("Error {}", ANDROID_MAX_ERRORS + 1));
    }

    #[test]
    fn blank_messages_are_not_shown_on_android() {
        let mut a = Android::new();
        assert_eq!(a.add("  ", ToastKind::Err), None);
        assert!(a.s.is_empty());
        // the web draws what it is given
        let mut w: ToastStack = ToastStack::new(ToastPolicy::web());
        assert!(w.show("", ToastKind::Ok, 0).is_some());
    }

    #[test]
    fn newest_is_zero_with_no_toasts() {
        assert_eq!(ToastStack::<()>::new(ToastPolicy::android()).newest(), 0);
    }

    #[test]
    fn the_newest_toast_is_nearest_the_edge_the_toasts_come_from() {
        let mut a = Android::new();
        a.add("Added to the list", ToastKind::Info);
        a.add("Saved", ToastKind::Info);
        let words = |v: Vec<&Toast<u32>>| v.iter().map(|t| t.text.clone()).collect::<Vec<_>>();
        assert_eq!(words(draw_order(a.s.items(), false)), ["Added to the list", "Saved"]);
        assert_eq!(words(draw_order(a.s.items(), true)), ["Saved", "Added to the list"]);
    }

    #[test]
    fn an_undo_toast_has_its_button_and_its_own_time() {
        let mut s: ToastStack<u32> = ToastStack::new(ToastPolicy::android());
        let id = s.show_action("List cleared", UNDO, 1, UNDO_MS, 0).unwrap();
        let t = s.items()[0].clone();
        assert_eq!(t.action.as_ref().map(|a| a.label.as_str()), Some("Undo"));
        assert_eq!(t.life_ms, 6000);
        assert!(!t.stays());
        // cleared again before it went: one toast, whose Undo undoes the latest
        assert_eq!(s.show_action("List cleared", UNDO, 10, UNDO_MS, 1), Some(id));
        assert_eq!(s.items().len(), 1);
        assert_eq!(s.take_action(id), Some(10));
        assert!(s.is_empty(), "the toast goes with its button");
        assert_eq!(s.take_action(id), None, "once");
        // a toast without one: no button, the usual time
        s.show("Saved", ToastKind::Info, 3);
        assert!(s.items()[0].action.is_none());
        assert_eq!(s.items()[0].life_ms, ANDROID_SHOW_MS);
        // a time without an action is ignored: only an action toast keeps its own time
        s.add("Plain", ToastKind::Info, None, Some(9000), 4);
        assert_eq!(s.items()[1].life_ms, ANDROID_SHOW_MS);
    }

    // ---- the web preset --------------------------------------------------------------------------------------

    #[test]
    fn the_web_keeps_three_errors_and_drops_the_same_error_again() {
        let mut w: ToastStack = ToastStack::new(ToastPolicy::web());
        let a = w.show("A failed", ToastKind::Err, 0);
        w.show("B failed", ToastKind::Err, 1);
        // the same error again (a retry that failed the same way): the one on screen stays, nothing piles up
        assert_eq!(w.show("A failed", ToastKind::Err, 2), None);
        assert_eq!(w.items()[0].id, a.unwrap(), "it stays where it was");
        w.show("C failed", ToastKind::Err, 3);
        w.show("D failed", ToastKind::Err, 4);
        let texts: Vec<_> = w.items().iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["B failed", "C failed", "D failed"], "a new one pushes out the oldest");
        // other toasts are neither capped nor merged
        for _ in 0..6 {
            w.show("Saved", ToastKind::Ok, 5);
        }
        assert_eq!(w.items().len(), 9);
        // a failed copy goes like any other toast, and is not merged
        w.show(COPY_FAILED, ToastKind::Err, 6);
        w.show(COPY_FAILED, ToastKind::Err, 6);
        assert_eq!(w.items().iter().filter(|t| t.text == COPY_FAILED).count(), 2);
    }

    #[test]
    fn the_web_fades_a_toast_at_3800_and_removes_it_at_4200() {
        let p = ToastPolicy::web();
        let mut w: ToastStack = ToastStack::new(p);
        w.show("Saved", ToastKind::Ok, 1000);
        let t = w.items()[0].clone();
        assert_eq!(t.fades_at(&p), Some(4800));
        assert_eq!(t.expires_at(), Some(5200));
        assert!(!t.fading(&p, 4799) && t.fading(&p, 4800));
        assert_eq!(w.next_deadline(), Some(5200));
        assert!(!w.tick(5199));
        assert!(w.tick(5200));
        // an action toast fades 400 ms before its own time
        w.show_action("List cleared", UNDO, (), UNDO_MS, 0);
        assert_eq!(w.items()[0].fades_at(&p), Some(5600));
        assert_eq!(w.items()[0].expires_at(), Some(6000));
        // an error never goes by itself
        w.show("Failed", ToastKind::Err, 0);
        assert_eq!(w.items()[1].expires_at(), None);
        assert!(w.tick(1_000_000));
        assert_eq!(w.items().len(), 1);
        assert_eq!(w.next_deadline(), None);
    }

    // ---- the desktop preset ----------------------------------------------------------------------------------

    #[test]
    fn desktop_errors_stay_until_closed_and_the_rest_go() {
        let mut d: ToastStack<&str> = ToastStack::new(ToastPolicy::desktop());
        d.show("x", ToastKind::Err, 0);
        d.show("Copied to clipboard", ToastKind::Ok, 0);
        // the failed copy says what to do instead: it goes like any other toast
        d.show(COPY_FAILED, ToastKind::Err, 0);
        // a toast with Undo stays as long as Undo can be pressed
        d.show_action("List cleared", UNDO, "undo-clear", 6000, 55_000);
        d.tick(60_000);
        let texts: Vec<_> = d.items().iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["x", "List cleared"]);
        d.tick(61_000);
        assert_eq!(d.items().len(), 1);
    }

    #[test]
    fn desktop_keeps_three_errors_and_five_toasts() {
        let mut d: ToastStack = ToastStack::new(ToastPolicy::desktop());
        for i in 1..=4 {
            d.show(format!("Error {i}"), ToastKind::Err, 0);
        }
        assert_eq!(d.show("Error 4", ToastKind::Err, 0), None, "a page that polls does not pile up a copy");
        let texts: Vec<_> = d.items().iter().map(|t| t.text.clone()).collect();
        assert_eq!(texts, ["Error 2", "Error 3", "Error 4"]);
        for i in 1..=3 {
            d.show(format!("Note {i}"), ToastKind::Ok, 0);
        }
        // five in all: the oldest goes, whatever its kind
        let texts: Vec<_> = d.items().iter().map(|t| t.text.clone()).collect();
        assert_eq!(texts, ["Error 3", "Error 4", "Note 1", "Note 2", "Note 3"]);
    }

    #[test]
    fn kinds_read_as_their_names() {
        for k in [ToastKind::Plain, ToastKind::Info, ToastKind::Ok, ToastKind::Warn, ToastKind::Err] {
            assert_eq!(ToastKind::parse(k.as_str()), k);
        }
        assert_eq!(copy_toast(true, COPIED), ("Copied to clipboard", ToastKind::Ok));
        assert_eq!(copy_toast(false, COPIED), (COPY_FAILED, ToastKind::Err));
    }

    #[test]
    fn an_app_may_word_the_failed_copy_its_own_way() {
        let mut s: ToastStack = ToastStack::new(ToastPolicy::web()).with_copy_failed("Kopieren fehlgeschlagen");
        s.show("Kopieren fehlgeschlagen", ToastKind::Err, 0);
        s.show(COPY_FAILED, ToastKind::Err, 0);
        assert!(!s.items()[0].stays(), "the app's words go by themselves");
        assert!(s.items()[1].stays(), "the kit's words are an ordinary error for this app");
        assert_eq!(ToastWords::default(), ToastWords::DEFAULT);
        assert_eq!(ToastWords::DEFAULT.undo, "Undo");
    }
}
