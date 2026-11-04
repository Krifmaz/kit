//! The admin's notice banner: a plain sentence saying what is wrong and what to check, with Check again.
//!
//! From Encore's `web-rs/src/notice.rs` (`banner_html`, `check_toast`, `paint`, `keep`, `start`): asked when the
//! app starts and every minute (every 15 s while it fails, so the banner goes soon after the fix); Check again
//! runs the check at once and says how it went. Nothing for someone who is not an admin. The rules are
//! kit-rules' [`kit_rules::notices`]; Encore's own notice is the `downloads` part of
//! `GET /api/requests/storage` ([`NoticeSource::encore_downloads`]).

use crate::api;
use crate::html::html;
use crate::icons::icon;
use crate::toasts::show;
use kit_rules::notices::{check_toast, CheckWords, Notice, NoticePoll, CHECKING, CHECK_AGAIN, TICK_MS};
use leptos::prelude::*;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// Where one notice comes from and what Check again says.
#[derive(Clone)]
pub struct NoticeSource {
    /// The path that answers it (`/requests/storage`).
    pub path: String,
    /// The path Check again posts to (`/requests/storage/check`).
    pub check_path: String,
    /// The part of the answer that is the notice (`downloads`), or "" for the whole answer.
    pub part: String,
    /// Check again's three sentences.
    pub words: CheckWords,
    /// Runs after a Check again that answered (Encore tells its Requests and Downloader pages).
    pub after_check: Option<Rc<dyn Fn()>>,
}

impl NoticeSource {
    /// Encore's storage notice: the `downloads` part of `GET /api/requests/storage`, checked by
    /// `POST /api/requests/storage/check`, with [`CheckWords::DOWNLOADS`]. After a check it sends
    /// `encore:requests-changed` and `encore:storage-checked`, as Encore's `notice.rs` does.
    pub fn encore_downloads() -> Self {
        NoticeSource {
            path: "/requests/storage".into(),
            check_path: "/requests/storage/check".into(),
            part: "downloads".into(),
            words: CheckWords::DOWNLOADS,
            after_check: Some(Rc::new(|| {
                crate::rt::dispatch("encore:requests-changed");
                crate::rt::dispatch("encore:storage-checked");
            })),
        }
    }

    fn notice_of(&self, answer: &Value) -> Notice {
        let v = if self.part.is_empty() { answer } else { answer.get(&self.part).unwrap_or(&Value::Null) };
        if !v.is_object() {
            return Notice::default();
        }
        serde_json::from_value(v.clone()).unwrap_or_default()
    }
}

thread_local! {
    static LAST: ArcRwSignal<Notice> = ArcRwSignal::new(Notice::default());
    static ADMIN: ArcRwSignal<bool> = ArcRwSignal::new(false);
    static SOURCE: RefCell<Option<NoticeSource>> = const { RefCell::new(None) };
    /// The slot [`mount`] draws into, and the banner on it now (its sentence and view).
    static SLOT: RefCell<Option<Slot>> = const { RefCell::new(None) };
}

/// The banner's slot and what is drawn on it: the sentence shown and its view, none while no banner shows.
type Slot = (web_sys::Element, Option<(String, crate::rt::Mounted)>);

/// Draws the banner in [`mount`]'s slot again, as Encore's `paint` does after every answer: a new sentence is a
/// new banner (its button ready again), none takes it away and leaves the slot empty (no placeholder).
fn repaint() {
    let text = LAST.with(|l| l.get_untracked().banner().map(str::to_string)).filter(|_| ADMIN.with(|a| a.get_untracked()));
    let gone = SLOT.with(|s| {
        let mut s = s.borrow_mut();
        let (el, now) = s.as_mut()?;
        if now.as_ref().map(|(t, _)| t) == text.as_ref() {
            return None;
        }
        let old = now.take();
        if let Some(t) = text {
            let busy = ArcRwSignal::new(false);
            let shown = t.clone();
            let m = crate::rt::mount(el, move || {
                let b = busy.clone();
                view! { <NoticeBanner text=shown busy=Signal::derive(move || b.get()) on_check=Callback::new(move |_| check_again(busy.clone()))/> }
            });
            if let Some(m) = m {
                *now = Some((t, m));
            }
        }
        old
    });
    // the old banner leaves the page outside the borrow
    drop(gone);
}

/// The last notice the server sent.
pub fn last() -> ArcRwSignal<Notice> {
    LAST.with(Clone::clone)
}

/// Keeps a notice answer (the whole answer of the source's path) and draws the banner again (also after another
/// page's own check). From `notice.rs` `keep`.
pub fn keep(answer: &Value) {
    let n = SOURCE.with(|s| s.borrow().as_ref().map(|s| s.notice_of(answer)));
    if let Some(n) = n {
        LAST.with(|l| l.set(n));
        repaint();
    }
}

/// Sets the notice itself.
pub fn set(n: Notice) {
    LAST.with(|l| l.set(n));
    repaint();
}

/// Whether the banner is for this user (admins only).
pub fn set_admin(admin: bool) {
    ADMIN.with(|a| a.set(admin));
    repaint();
}

async fn load() {
    let Some(path) = SOURCE.with(|s| s.borrow().as_ref().map(|s| s.path.clone())) else { return };
    // offline, or an older server: the banner keeps what it last knew
    if let Ok(v) = api::get(&path).await {
        keep(&v);
    }
}

/// Starts asking (admins, once per page load). From `notice.rs` `start`.
pub fn start(source: NoticeSource, admin: bool) {
    set_admin(admin);
    let first = SOURCE.with(|s| s.borrow_mut().replace(source).is_none());
    if !admin || !first {
        return;
    }
    crate::rt::spawn(load());
    let mut poll = NoticePoll::default();
    crate::rt::every(TICK_MS, move || {
        if poll.tick(LAST.with(|l| l.get_untracked().failing())) {
            crate::rt::spawn(load());
        }
    });
}

/// Check again: asks the server to check at once, keeps the answer and says how it went. `busy` is the button's
/// state. From `notice.rs` `start` (the `[data-notice-check]` click).
pub fn check_again(busy: ArcRwSignal<bool>) {
    let Some(src) = SOURCE.with(|s| s.borrow().clone()) else { return };
    busy.set(true);
    crate::rt::spawn(async move {
        match api::post_empty(&src.check_path).await {
            Ok(v) => {
                keep(&v);
                let (msg, kind) = check_toast(&src.notice_of(&v), &src.words);
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

/// The banner for one notice: its sentence and Check again (`on_check`). `busy` while the check runs. From
/// `notice.rs` `banner_html`.
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
) -> impl IntoView {
    let label = move || {
        if busy.get() {
            format!(r#"<span class="spinner" style="width:12px;height:12px"></span> {CHECKING}"#)
        } else {
            format!("{}{CHECK_AGAIN}", icon("refresh"))
        }
    };
    view! {
        <div class="storage-banner" role="status">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(crate::icons::paths("alert"))}>{""}</svg>
            <span>{text}</span>
            <button class="btn sm" data-notice-check disabled=move || busy.get() on:click=move |_| on_check.run(()) {..html(label)}>{""}</button>
        </div>
    }
}

/// The banner of the notice [`start`] watches, for admins, while it fails (nothing otherwise): Encore's
/// `#server-notice` slot. From `notice.rs` `paint`.
#[component]
pub fn ServerNotice() -> impl IntoView {
    let last = last();
    let admin = ADMIN.with(Clone::clone);
    let busy = ArcRwSignal::new(false);
    move || {
        let n = last.get();
        let text = n.banner().map(str::to_string).filter(|_| admin.get())?;
        let b = busy.clone();
        let b2 = busy.clone();
        Some(
            view! { <NoticeBanner text=text busy=Signal::derive(move || b.get()) on_check=Callback::new(move |_| check_again(b2.clone()))/> },
        )
    }
}

/// Draws the banner into the page's existing `#server-notice` (Encore's shell has one), and again there after
/// every answer. The slot holds only the banner: nothing at all while there is none, as in Encore.
pub fn mount() {
    let Some(el) = crate::rt::by_id("server-notice") else { return };
    let old = SLOT.with(|s| s.borrow_mut().replace((el, None)));
    drop(old);
    repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::esc;
    use crate::testing::{assert_same_html, render};
    use serde_json::json;

    /// Encore's `banner_html` (`notice.rs`).
    fn banner_html(text: &str) -> String {
        format!(
            r#"<div class="storage-banner" role="status">{}<span>{}</span><button class="btn sm" data-notice-check>{}Check again</button></div>"#,
            icon("alert"),
            esc(text),
            icon("refresh")
        )
    }

    const TEXT: &str = "The downloads folder /downloads is read-only – check the disk (dmesg, fsck) or the mount";

    #[test]
    fn the_banner_is_encores() {
        let html = render(|| view! { <NoticeBanner text=TEXT busy=false on_check=Callback::new(|_| {})/> });
        assert_same_html(&html, &banner_html(TEXT));
        let busy = render(|| view! { <NoticeBanner text=TEXT busy=true on_check=Callback::new(|_| {})/> });
        assert!(
            busy.contains(r#"<span class="spinner" style="width:12px;height:12px"></span> Checking…"#) && busy.contains("disabled"),
            "{busy}"
        );
    }

    #[test]
    fn the_banner_shows_for_admins_while_the_notice_fails() {
        SOURCE.with(|s| *s.borrow_mut() = Some(NoticeSource::encore_downloads()));
        set_admin(true);
        keep(&json!({ "downloads": { "ok": false, "text": TEXT, "waiting": 3 } }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), &banner_html(TEXT));
        // a member sees nothing
        set_admin(false);
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        // every folder writable, an older server: nothing
        set_admin(true);
        keep(&json!({ "downloads": { "ok": true, "text": null } }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        keep(&json!({}));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
        keep(&json!({ "downloads": { "ok": false } }));
        assert_same_html(&render(|| view! { <ServerNotice/> }), "");
    }
}
