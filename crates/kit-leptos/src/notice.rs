//! The admin's notice banners: for each notice that fails, a plain sentence saying what is wrong and what to
//! check, with Check again.
//!
//! The server keeps a list of notices (`GET /api/notices`, `{"notices": [{id, ok, text, checking, severity}]}`,
//! `contracts/notices.md`); Check again posts `/api/notices/check`, which checks every notice and answers the same
//! list. The toast after a check says how that notice went in its own three sentences
//! ([`NoticeSource::words_for`]) or the configured ones ([`Words::notices`](crate::config::Words::notices)). An
//! answer that is one notice object (an older single-notice route) counts as a list of one.

use crate::api;
use crate::html::html;
use crate::icons::icon;
use crate::toasts::show;
use kit_rules::notices::{banners, check_toast, find, parse, CheckWords, Notice, NoticePoll, Severity, CHECKING, CHECK_AGAIN, TICK_MS};
use leptos::prelude::*;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Where the notices come from and what Check again says.
#[derive(Clone)]
pub struct NoticeSource {
    /// The path that answers them (`/notices`).
    pub path: String,
    /// The path Check again posts to (`/notices/check`).
    pub check_path: String,
    /// The part of the answer that holds the notices (a field name), or "" for the whole answer.
    pub part: String,
    /// Check again's three sentences for a notice without words of its own.
    pub words: CheckWords,
    /// Check again's sentences per notice id (`("storage", CheckWords { … })`).
    pub words_for: Vec<(String, CheckWords)>,
    /// Runs after a Check again that answered (to tell the app's own pages).
    pub after_check: Option<Rc<dyn Fn()>>,
}

impl Default for NoticeSource {
    /// The configured paths ([`notices`](crate::config::Paths::notices),
    /// [`notices_check`](crate::config::Paths::notices_check)) and words.
    fn default() -> Self {
        let c = crate::config::get();
        NoticeSource {
            path: c.paths.notices.clone(),
            check_path: c.paths.notices_check.clone(),
            part: String::new(),
            words: c.words.notices,
            words_for: Vec::new(),
            after_check: None,
        }
    }
}

impl NoticeSource {
    /// With Check again's sentences for notice `id`.
    pub fn words_for(mut self, id: &str, words: CheckWords) -> Self {
        self.words_for.push((id.to_string(), words));
        self
    }

    /// The words Check again's toast says for notice `id`.
    pub fn words_of(&self, id: &str) -> CheckWords {
        self.words_for.iter().find(|(i, _)| i == id).map(|(_, w)| *w).unwrap_or(self.words)
    }

    /// The notices of an answer.
    pub fn notices_of(&self, answer: &Value) -> Vec<Notice> {
        let v = if self.part.is_empty() { answer } else { answer.get(&self.part).unwrap_or(&Value::Null) };
        parse(v)
    }
}

thread_local! {
    static LAST: ArcRwSignal<Vec<Notice>> = ArcRwSignal::new(Vec::new());
    static ADMIN: ArcRwSignal<bool> = ArcRwSignal::new(false);
    static SOURCE: RefCell<Option<NoticeSource>> = const { RefCell::new(None) };
    /// The slot [`mount`] draws into, and the banners on it now (their sentences and view).
    static SLOT: RefCell<Option<Slot>> = const { RefCell::new(None) };
}

/// The banners' slot and what is drawn on it: the sentences shown and their view, none while no banner shows.
type Slot = (web_sys::Element, Option<(Vec<String>, crate::rt::Mounted)>);

/// The banners that show now: (id, sentence, severity) of each failing notice, for admins only.
fn shown() -> Vec<(String, String, Severity)> {
    let admin = ADMIN.with(|a| a.get_untracked());
    LAST.with(|l| {
        banners(&l.get_untracked(), admin).into_iter().map(|n| (n.id.clone(), n.banner().unwrap_or("").to_string(), n.severity)).collect()
    })
}

/// Draws the banners in [`mount`]'s slot again after every answer: new sentences are new banners (their buttons
/// ready again), none takes them away and leaves the slot empty (no placeholder).
fn repaint() {
    let now = shown();
    let texts: Vec<String> = now.iter().map(|(_, t, _)| t.clone()).collect();
    let gone = SLOT.with(|s| {
        let mut s = s.borrow_mut();
        let (el, drawn) = s.as_mut()?;
        if drawn.as_ref().map(|(t, _)| t) == Some(&texts) || (drawn.is_none() && texts.is_empty()) {
            return None;
        }
        let old = drawn.take();
        if !texts.is_empty() {
            let m = crate::rt::mount(el, move || seq_banners(now));
            if let Some(m) = m {
                *drawn = Some((texts, m));
            }
        }
        old
    });
    // the old banners leave the page outside the borrow
    drop(gone);
}

fn seq_banners(list: Vec<(String, String, Severity)>) -> AnyView {
    crate::html::seq(list.into_iter().map(|(id, text, severity)| {
        let busy = ArcRwSignal::new(false);
        let b = busy.clone();
        view! { <NoticeBanner id=id.clone() text=text severity=severity busy=Signal::derive(move || b.get()) on_check=Callback::new(move |_| check_again(&id, busy.clone()))/> }
    }))
}

/// The last notices the server sent.
pub fn last() -> ArcRwSignal<Vec<Notice>> {
    LAST.with(Clone::clone)
}

/// Keeps a notices answer (the whole answer of the source's path) and draws the banners again (also after
/// another page's own check).
pub fn keep(answer: &Value) {
    let n = SOURCE.with(|s| s.borrow().as_ref().map(|s| s.notices_of(answer)));
    if let Some(n) = n {
        set_all(n);
    }
}

/// Sets one notice as the whole list.
pub fn set(n: Notice) {
    set_all(vec![n]);
}

/// Sets the notices themselves.
pub fn set_all(n: Vec<Notice>) {
    LAST.with(|l| l.set(n));
    repaint();
}

/// Whether the banners are for this user (admins only).
pub fn set_admin(admin: bool) {
    ADMIN.with(|a| a.set(admin));
    repaint();
}

async fn load() {
    let Some(path) = SOURCE.with(|s| s.borrow().as_ref().map(|s| s.path.clone())) else { return };
    // offline, or an older server: the banners keep what they last knew
    if let Ok(v) = api::get(&path).await {
        keep(&v);
    }
}

/// Starts asking (admins, once per page load): every minute, every 15 s while a notice fails.
pub fn start(source: NoticeSource, admin: bool) {
    set_admin(admin);
    let first = SOURCE.with(|s| s.borrow_mut().replace(source).is_none());
    if !admin || !first {
        return;
    }
    crate::rt::spawn(load());
    let mut poll = NoticePoll::default();
    crate::rt::every(TICK_MS, move || {
        if poll.tick(LAST.with(|l| l.get_untracked().iter().any(Notice::failing))) {
            crate::rt::spawn(load());
        }
    });
}

/// Check again for notice `id`: asks the server to check at once, keeps the answer and says how that notice
/// went (a notice the answer no longer lists is fixed). `busy` is the button's state.
pub fn check_again(id: &str, busy: ArcRwSignal<bool>) {
    let Some(src) = SOURCE.with(|s| s.borrow().clone()) else { return };
    let id = id.to_string();
    busy.set(true);
    crate::rt::spawn(async move {
        match api::post_empty(&src.check_path).await {
            Ok(v) => {
                keep(&v);
                let list = src.notices_of(&v);
                let n = find(&list, &id).cloned().unwrap_or_default();
                let (msg, kind) = check_toast(&n, &src.words_of(&id));
                show(msg, kind);
                if let Some(f) = &src.after_check {
                    f();
                }
                // the banner goes or is drawn again; a button that stays is ready again
                busy.set(false);
            }
            Err(err) => {
                crate::toasts::toast(&err.message, "err");
                busy.set(false);
            }
        }
    });
}

fn severity_class(s: Severity) -> &'static str {
    match s {
        Severity::Info => "info",
        Severity::Warn => "warn",
        Severity::Err => "err",
    }
}

/// The banner for one notice: its sentence and Check again (`on_check`). `busy` while the check runs.
#[component]
pub fn NoticeBanner(
    /// The sentence.
    #[prop(into)]
    text: String,
    /// The check runs (the button says "Checking…").
    #[prop(into)]
    busy: Signal<bool>,
    /// Check again was pressed.
    on_check: Callback<()>,
    /// The notice's id (`data-notice`), "" for none.
    #[prop(optional, into)]
    id: String,
    /// How serious it is (the banner's colour).
    #[prop(optional)]
    severity: Severity,
) -> impl IntoView {
    let label = move || {
        if busy.get() {
            format!(r#"<span class="spinner sm"></span> {CHECKING}"#)
        } else {
            format!("{}{CHECK_AGAIN}", icon("refresh"))
        }
    };
    view! {
        <div class=format!("notice-banner {}", severity_class(severity)) role="status" data-notice=id>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(crate::icons::paths("alert"))}>{""}</svg>
            <span>{text}</span>
            <button class="btn sm" data-notice-check disabled=move || busy.get() on:click=move |_| on_check.run(()) {..html(label)}>{""}</button>
        </div>
    }
}

/// The banners of the notices [`start`] watches, for admins, while they fail (nothing otherwise).
#[component]
pub fn ServerNotice() -> impl IntoView {
    let last = last();
    let admin = ADMIN.with(Clone::clone);
    move || {
        let list: Vec<(String, String, Severity)> = banners(&last.get(), admin.get())
            .into_iter()
            .map(|n| (n.id.clone(), n.banner().unwrap_or("").to_string(), n.severity))
            .collect();
        seq_banners(list)
    }
}

/// Draws the banners into the page's existing `#server-notice`, and again there after every answer. The slot
/// holds only the banners: nothing at all while there is none.
pub fn mount() {
    let Some(el) = crate::rt::by_id("server-notice") else { return };
    let old = SLOT.with(|s| s.borrow_mut().replace((el, None)));
    drop(old);
    repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};
    use serde_json::json;

    const ALERT_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>"#;
    const REFRESH_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/></svg>"#;

    /// The kit's banner markup for one notice.
    fn banner(id: &str, severity: &str, text: &str) -> String {
        format!(
            r#"<div class="notice-banner {severity}" role="status" data-notice="{id}">{ALERT_SVG}<span>{text}</span><button class="btn sm" data-notice-check>{REFRESH_SVG}Check again</button></div>"#
        )
    }

    const TEXT: &str = "The data folder /data is read-only – check the disk or the mount";

    #[test]
    fn a_banner_says_its_sentence_with_check_again() {
        let html = render(|| view! { <NoticeBanner id="storage" text=TEXT busy=false on_check=Callback::new(|_| {})/> });
        assert_same_html(&html, &banner("storage", "warn", TEXT));
        let busy = render(|| view! { <NoticeBanner text=TEXT severity=Severity::Err busy=true on_check=Callback::new(|_| {})/> });
        assert!(busy.contains(r#"<span class="spinner sm"></span> Checking…"#) && busy.contains("disabled"), "{busy}");
        assert!(busy.contains(r#"class="notice-banner err""#), "{busy}");
    }

    #[test]
    fn the_banners_show_for_admins_while_notices_fail() {
        SOURCE.with(|s| *s.borrow_mut() = Some(NoticeSource::default()));
        set_admin(true);
        keep(&json!({ "notices": [
            { "id": "storage", "ok": false, "text": TEXT },
            { "id": "mail", "ok": true, "text": null },
            { "id": "queue", "ok": false, "text": "The job queue is stuck", "severity": "err" }
        ] }));
        let want = format!("{}{}", banner("storage", "warn", TEXT), banner("queue", "err", "The job queue is stuck"));
        assert_same_html(&render(|| view! { <ServerNotice/> }), &want);
        // a member sees nothing
        set_admin(false);
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        // every notice fine, an older server, one notice object: as the server says
        set_admin(true);
        keep(&json!({ "notices": [{ "id": "storage", "ok": true }] }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        keep(&json!({}));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        keep(&json!({ "id": "storage", "ok": false, "text": TEXT }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), &banner("storage", "warn", TEXT));
        keep(&json!({ "notices": [{ "ok": false }] }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
    }

    #[test]
    fn check_again_says_each_notice_its_own_way() {
        let storage = CheckWords { failing: "Still read-only", checking: "Still checking the disk", fixed: "Writable again" };
        let src = NoticeSource::default().words_for("storage", storage);
        assert_eq!(src.words_of("storage"), storage);
        assert_eq!(src.words_of("mail"), CheckWords::DEFAULT);
        assert_eq!((src.path.as_str(), src.check_path.as_str()), ("/notices", "/notices/check"));
        let nested = NoticeSource { part: "data".into(), ..NoticeSource::default() };
        assert_eq!(nested.notices_of(&json!({ "data": { "notices": [{ "id": "a", "ok": false, "text": "x" }] } })).len(), 1);
    }
}
