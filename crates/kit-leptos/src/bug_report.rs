//! The New report sheet: kind, title, description and files (picked or dropped), sent to the server with the
//! page, the app version and the browser (`contracts/bug-reports.md`).
//!
//! The words are the configured [`BugWords`]; the report goes to the configured
//! [`bug_reports`](crate::config::Paths::bug_reports) path, and `<app id>:bugs-changed` tells the app's pages.

use crate::api;
use crate::html::esc;
use crate::html::html;
use crate::icons::icon;
use crate::modal::{self, Modal, ModalOpts};
use crate::toasts::toast;
use kit_rules::bugs::{device, BugWords, ACCEPT, TEXT_MAX, TITLE_MAX};
use kit_rules::uploads::FileLimits;
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::File;

/// What goes with a report, and what happens once it went.
#[derive(Clone)]
pub struct SheetOpts {
    /// The app's id: the event `<id>:bugs-changed` after sending.
    pub app: String,
    /// The page the user came from (`#/projects`), "" for none.
    pub page: String,
    /// The app's version, "" when unknown.
    pub version: String,
    /// The path the report is posted to (`/bug-reports`).
    pub path: String,
    /// Runs with the new report's id once it went (the app may open it).
    pub on_sent: Option<Rc<dyn Fn(i64)>>,
}

impl SheetOpts {
    /// A report from `page` of the app `app` (its id or name: the event is `<app in lower case>:bugs-changed`) at
    /// `version`, posted to the configured path.
    pub fn new(app: &str, page: &str, version: &str) -> Self {
        let path = crate::config::get().paths.bug_reports.clone();
        SheetOpts { app: app.to_string(), page: page.to_string(), version: version.to_string(), path, on_sent: None }
    }

    /// A report from `page` of the configured app, with its id and version.
    pub fn from_config(page: &str) -> Self {
        let c = crate::config::get();
        SheetOpts::new(&c.app.id, page, &c.app.version)
    }
}

fn words() -> BugWords {
    crate::config::get().words.bugs
}

/// A picked file as its thumbnail shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct Thumb {
    /// The file's name.
    pub name: String,
    /// An object URL for an image, None for a text file.
    pub url: Option<String>,
}

/// The sheet's state while it is open.
#[derive(Clone, Copy)]
pub struct Sheet {
    /// The kind picked (`bug`, `not_bug`, `howto`).
    pub kind: RwSignal<String>,
    /// The thumbnails of the picked files.
    pub thumbs: RwSignal<Vec<Thumb>>,
    /// The picked files themselves.
    files: StoredValue<Vec<File>, LocalStorage>,
    /// The report is being sent.
    pub sending: RwSignal<bool>,
    /// The title input.
    title: NodeRef<leptos::html::Input>,
    /// The description.
    body: NodeRef<leptos::html::Textarea>,
    /// The hidden file input.
    input: NodeRef<leptos::html::Input>,
}

impl Sheet {
    /// A sheet with nothing picked (under the current reactive owner).
    pub fn new() -> Self {
        Sheet {
            kind: RwSignal::new(words().kinds[0].0.to_string()),
            thumbs: RwSignal::new(Vec::new()),
            files: StoredValue::new_local(Vec::new()),
            sending: RwSignal::new(false),
            title: NodeRef::new(),
            body: NodeRef::new(),
            input: NodeRef::new(),
        }
    }

    fn draw(&self) {
        for t in self.thumbs.get_untracked() {
            if let Some(u) = t.url {
                let _ = web_sys::Url::revoke_object_url(&u);
            }
        }
        let thumbs = self.files.with_value(|fs| {
            fs.iter()
                .map(|f| Thumb {
                    name: f.name(),
                    url: f.type_().starts_with("image/").then(|| web_sys::Url::create_object_url_with_blob(f).ok()).flatten(),
                })
                .collect()
        });
        self.thumbs.set(thumbs);
    }

    /// Adds picked or dropped files under the limits, saying what was left out.
    pub fn add(&self, list: web_sys::FileList) {
        let files: Vec<File> = (0..list.length()).filter_map(|i| list.get(i)).collect();
        let named: Vec<(String, u64)> = files.iter().map(|f| (f.name(), f.size() as u64)).collect();
        let refs: Vec<(&str, u64)> = named.iter().map(|(n, s)| (n.as_str(), *s)).collect();
        let have = self.files.with_value(Vec::len);
        let (taken, said) = FileLimits::BUG_REPORT.add(have, &refs);
        for msg in said {
            toast(&msg, "err");
        }
        self.files.update_value(|fs| fs.extend(taken.into_iter().map(|i| files[i].clone())));
        self.draw();
    }

    /// Takes file `i` out.
    pub fn remove(&self, i: usize) {
        self.files.update_value(|fs| {
            if i < fs.len() {
                fs.remove(i);
            }
        });
        self.draw();
    }
}

impl Default for Sheet {
    fn default() -> Self {
        Self::new()
    }
}

/// The sheet's body.
pub fn sheet_body(s: Sheet, page: &str, version: &str) -> impl IntoView {
    let w = words();
    let seg: Vec<_> = w
        .kinds
        .iter()
        .map(|(k, l)| {
            let k = *k;
            view! { <button {..()} type="button" data-v=k class=move || if s.kind.get() == k { "on" } else { "" } on:click=move |_| s.kind.set(k.to_string())>{*l}</button> }
        })
        .collect();
    let thumbs = move || {
        s.thumbs
            .get()
            .into_iter()
            .enumerate()
            .map(|(i, t)| {
                let inner = match &t.url {
                    Some(u) => view! { <img src=u.clone() alt=t.name.clone()/> }.into_any(),
                    None => view! { <span class="small ellipsis" title=t.name.clone()>{t.name.clone()}</span> }.into_any(),
                };
                view! {
                    <div class="bug-thumb" data-file=i title=t.name.clone()>{inner}
                        <button {..()} type="button" class="bug-thumb-x" data-rm=i aria-label=format!("{} {}", w.remove, t.name)
                            on:click=move |e| { e.prevent_default(); s.remove(i); }>"×"</button>
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };
    view! {
        <div {..()} class="stack" style="gap:12px" data-dropzone="br-files">
            <div class="seg" id="br-kind">{seg}</div>
            <label class="field"><span>{w.title}</span><input class="input" id="br-title" maxlength=TITLE_MAX placeholder=w.title_placeholder node_ref=s.title/></label>
            <label class="field"><span>{w.description}</span><textarea class="input" id="br-body" rows="6" maxlength=TEXT_MAX placeholder=w.description_placeholder node_ref=s.body></textarea></label>
            <div {..()} class="row wrap" style="gap:10px">
                <button {..()} type="button" class="btn sm" id="br-add" on:click=move |e| { e.prevent_default(); if let Some(i) = s.input.get_untracked() { i.click(); } }
                    {..html(format!("{}{}", icon("plus"), w.add_images))}>{""}</button>
                <input type="file" id="br-files" multiple accept=ACCEPT hidden node_ref=s.input
                    on:change=move |e| {
                        let Some(i) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) else { return };
                        if let Some(list) = i.files() { s.add(list); }
                        i.set_value("");
                    }/>
                <span class="small muted">{w.files_line(&FileLimits::BUG_REPORT)}</span>
            </div>
            <div class="bug-thumbs" id="br-thumbs">{thumbs}</div>
            <div class="small muted" id="br-meta">{w.sent_along(page, version)}</div>
        </div>
    }
}

/// The sheet's buttons: Cancel and Send ("Sending…" while it goes).
pub fn sheet_footer(s: Sheet, on_send: impl Fn() + 'static) -> impl IntoView {
    let w = words();
    let cancel = crate::config::get().words.dialogs.cancel;
    view! {
        <button class="btn" data-close>{cancel}</button>
        <button class="btn primary" id="br-send" disabled=move || s.sending.get() on:click=move |_| on_send()>{move || if s.sending.get() { w.sending } else { w.send }}</button>
    }
}

fn focus(el: Option<web_sys::HtmlElement>) {
    if let Some(e) = el {
        let _ = e.focus();
    }
}

/// Opens the New report sheet.
pub fn open(o: SheetOpts) -> Modal {
    let owner = Owner::new();
    let s = owner.with(Sheet::new);
    let (page, version) = (o.page.clone(), o.version.clone());
    let handle: Rc<std::cell::RefCell<Option<Modal>>> = Rc::default();
    let h = handle.clone();
    let w = words();
    let send = move || {
        let title = s.title.get_untracked().map(|i| i.value()).unwrap_or_default().trim().to_string();
        let text = s.body.get_untracked().map(|t| t.value()).unwrap_or_default().trim().to_string();
        if let Some(msg) = w.missing(&title, &text) {
            toast(msg, "err");
            if msg == w.need_title {
                focus(s.title.get_untracked().map(Into::into));
            } else {
                focus(s.body.get_untracked().map(Into::into));
            }
            return;
        }
        let Ok(fd) = web_sys::FormData::new() else { return };
        let ua = crate::rt::window().and_then(|w| w.navigator().user_agent().ok()).unwrap_or_default();
        let screen = crate::rt::window().and_then(|w| w.screen().ok()).and_then(|sc| Some((sc.width().ok()?, sc.height().ok()?)));
        let _ = fd.append_with_str("kind", &s.kind.get_untracked());
        let _ = fd.append_with_str("title", &title);
        let _ = fd.append_with_str("body", &text);
        let _ = fd.append_with_str("page", &o.page);
        let _ = fd.append_with_str("app_version", &o.version);
        let _ = fd.append_with_str("device", &device(&ua, screen));
        s.files.with_value(|fs| {
            for f in fs {
                let _ = fd.append_with_blob_and_filename("file", f, &f.name());
            }
        });
        s.sending.set(true);
        let (o, h) = (o.clone(), h.clone());
        crate::rt::spawn(async move {
            match api::post_form(&o.path, fd).await {
                Ok(r) => {
                    if let Some(m) = h.borrow().as_ref() {
                        m.close();
                    }
                    toast(w.sent, "ok");
                    crate::rt::dispatch(&format!("{}:bugs-changed", o.app.to_ascii_lowercase()));
                    if let Some(f) = &o.on_sent {
                        f(r.get("id").and_then(serde_json::Value::as_i64).unwrap_or(0));
                    }
                }
                Err(e) => {
                    toast(&e.message, "err");
                    s.sending.set(false);
                }
            }
        });
    };
    let m = modal::open(
        ModalOpts::new(&esc(w.new_report), "").body_view(move || sheet_body(s, &page, &version)).footer_view(move || sheet_footer(s, send)),
    );
    *handle.borrow_mut() = Some(m.clone());
    focus(s.title.get_untracked().map(Into::into));
    m.on_close(move || {
        s.thumbs.with_untracked(|ts| {
            for t in ts {
                if let Some(u) = &t.url {
                    let _ = web_sys::Url::revoke_object_url(u);
                }
            }
        });
        drop(owner);
    });
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};

    /// The kit's sheet body for `page` and `version` (both as the meta line says them, escaped).
    fn body(meta: &str) -> String {
        format!(
            r#"<div class="stack" style="gap:12px" data-dropzone="br-files">
              <div class="seg" id="br-kind"><button type="button" data-v="bug" class="on">Bug</button><button type="button" data-v="not_bug" class="">Not a bug</button><button type="button" data-v="howto" class="">How do I…</button></div>
              <label class="field"><span>Title</span><input class="input" id="br-title" maxlength="200" placeholder="A short name for it"></label>
              <label class="field"><span>Description</span><textarea class="input" id="br-body" rows="6" maxlength="20000" placeholder="What happened, what you expected, how to repeat it"></textarea></label>
              <div class="row wrap" style="gap:10px">
                <button type="button" class="btn sm" id="br-add">{plus}Add images</button>
                <input type="file" id="br-files" multiple accept="image/*,.txt,.log" hidden>
                <span class="small muted">Up to 6 files, 8 MB each. Images, .txt and .log files (pick or drop them here).</span>
              </div>
              <div class="bug-thumbs" id="br-thumbs"></div>
              <div class="small muted" id="br-meta">{meta}</div>
            </div>"#,
            plus = icon("plus")
        )
    }

    #[test]
    fn the_sheet_asks_for_kind_title_text_and_files() {
        assert_same_html(
            &render(|| sheet_body(Sheet::new(), "#/projects?q=<x>", "1.4.0")),
            &body("Sent along: the page you came from (#/projects?q=&lt;x&gt;), the app version (1.4.0) and your browser."),
        );
        assert_same_html(&render(|| sheet_body(Sheet::new(), "", "")), &body("Sent along: the app version (unknown) and your browser."));
        assert_same_html(
            &render(|| sheet_footer(Sheet::new(), || {})),
            r#"<button class="btn" data-close>Cancel</button><button class="btn primary" id="br-send">Send</button>"#,
        );
    }

    #[test]
    fn each_file_has_a_thumbnail_and_remove() {
        let html = render(|| {
            let s = Sheet::new();
            s.thumbs
                .set(vec![Thumb { name: "shot.png".into(), url: Some("blob:x".into()) }, Thumb { name: "server.log".into(), url: None }]);
            sheet_body(s, "", "")
        });
        let want = r#"<div class="bug-thumbs" id="br-thumbs"><div class="bug-thumb" data-file="0" title="shot.png"><img src="blob:x" alt="shot.png"><button type="button" class="bug-thumb-x" data-rm="0" aria-label="Remove shot.png">×</button></div><div class="bug-thumb" data-file="1" title="server.log"><span class="small ellipsis" title="server.log">server.log</span><button type="button" class="bug-thumb-x" data-rm="1" aria-label="Remove server.log">×</button></div></div>"#;
        let start = html[..html.find(r#"id="br-thumbs""#).unwrap()].rfind("<div").unwrap();
        let end = html[..html.find(r#"id="br-meta""#).unwrap()].rfind("<div").unwrap();
        assert_same_html(&html[start..end], want);
    }

    #[test]
    fn the_sheet_goes_where_the_app_says_in_its_words() {
        let mut cfg = crate::config::AppConfig::new(kit_rules::app::App::named("Kit Demo").with_version("1.4.0"));
        cfg.paths.bug_reports = "/feedback".into();
        cfg.words.bugs.new_report = "Feedback";
        cfg.words.bugs.title = "Betreff";
        crate::config::set(cfg);
        let o = SheetOpts::from_config("#/");
        assert_eq!((o.app.as_str(), o.version.as_str(), o.path.as_str()), ("kit-demo", "1.4.0", "/feedback"));
        assert!(render(|| sheet_body(Sheet::new(), "", "")).contains("<span>Betreff</span>"));
        crate::config::set(crate::config::AppConfig::default());
    }
}
