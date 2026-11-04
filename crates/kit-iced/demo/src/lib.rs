//! # kit-iced-demo
//!
//! Every kit-iced component in one desktop window, for an imaginary app called Demo. Each page of the
//! sidebar shows one feature; the dialogs (details, confirm, What's new, New report) and the toasts are
//! drawn over the window.
//!
//! The components hold no network code: their update functions answer what the app should do next. This
//! demo answers those requests itself, with fake data, on its next [`Message::Tick`] (half a second
//! later), so the "Checking…", "Saving…" and "Sending…" states can be seen. Copies go to the real
//! clipboard and are read back to check them, as an app does.

use iced::widget::{button, column, container, row, scrollable, space, stack, tooltip};
use iced::{Alignment, Background, Border, Length, Task};
use kit_iced::details::{self, Act, Details, Lead, Links};
use kit_iced::diagnostics;
use kit_iced::drop_zone::{self, Accept, DropZone, Outcome};
use kit_iced::form::{self, Form, Schema};
use kit_iced::notice::{self, CheckWords, Notice};
use kit_iced::theme::{self, Badge, Btn, Palette};
use kit_iced::toast::{self, ToastKind, Toasts};
use kit_iced::update::{self, Feed, Phase, UpdateMsg};
use kit_iced::whats_new::{self, Changelog, ChangelogMsg, Popup, PopupMsg};
use kit_iced::widgets::{self as w, El, Icon, Logo};
use kit_iced::{bug_report, confirm, offline, settings};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// The imaginary app's name, in every sentence the components build.
pub const APP: &str = "Demo";
/// The version the (fake) server runs.
pub const SERVER_VERSION: &str = "0.2.0";
/// The version of this (fake) desktop build, older than the release the update check finds.
pub const BUILD: &str = "0.1.0";
/// Where the fake releases live.
pub const FEED: Feed =
    Feed { app: APP, page: "https://github.com/example/demo/releases", downloads: "https://github.com/example/demo/releases/download/" };

/// The files the drag and drop page takes: images, several at once.
pub const IMAGES: Accept = Accept::many(&["png", "jpg", "jpeg", "webp", "gif"]);
/// The line the drop place shows while files are held over the window.
pub const DROP_HINT: &str = "Drop images here to add them";

/// The error toast of the Toasts page (it stays, with Copy).
pub const ERROR_TEXT: &str = "Couldn't reach the server: connection refused (127.0.0.1:8080)";
/// The Undo toast of the Toasts page.
pub const UNDO_TEXT: &str = "3 items archived";
/// The toast after Undo.
pub const UNDONE_TEXT: &str = "3 items are back";

/// The details dialog's title.
pub const DETAILS_TITLE: &str = "Import failed";
/// The details dialog's text, in full.
pub const DETAILS_TEXT: &str = "Import of report.zip stopped at file 14 of 20.\n\nPOST /api/items/import → HTTP 422\n{\n  \"error\": \"archive_invalid\",\n  \"detail\": \"The archive ends 312 bytes before its table of contents. It was probably cut off while it was being copied, so every file after the 14th could not be read and nothing after it was touched.\",\n  \"file\": \"report.zip\",\n  \"size\": 48213345,\n  \"checked_at\": \"2026-11-04T09:12:44Z\"\n}\n\nEvery file before it was imported.";
/// The details dialog's lead: why, in a sentence.
pub const DETAILS_LEAD: &str = "report.zip is not a complete archive";

/// The notice the Notices page starts with.
pub const NOTICE_TEXT: &str = "The data folder is read-only – check the disk and its permissions, then press Check again.";
/// The toasts after Check again.
pub const NOTICE_WORDS: CheckWords = CheckWords {
    failing: "The data folder still can't be written",
    checking: "Still checking the data folder",
    fixed: "The data folder can be written",
};

/// The toast after the confirm dialog's Delete.
pub const DELETED: &str = "Report deleted";

/// The diagnostics page's help line.
pub const DIAG_HELP: &str = "Runs every check of the server and says what to fix.";

/// One page of the sidebar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    /// Themes, accents, buttons, badges, cards, skeletons, icons and the logo.
    #[default]
    Appearance,
    /// Toasts: ok, error (with Copy), a repeated error, Undo.
    Toasts,
    /// The details dialog.
    Details,
    /// The confirm dialog.
    Confirm,
    /// The What's new popup and the changelog page.
    WhatsNew,
    /// The Updates block and the words of every update phase.
    Updates,
    /// The notice banner with Check again.
    Notices,
    /// The offline banner.
    Offline,
    /// Drag and drop.
    Drop,
    /// The settings pieces and the settings form.
    Settings,
    /// The New report dialog and a report with the reply box.
    BugReport,
    /// The diagnostics checks.
    Diagnostics,
}

impl Page {
    /// Every page, in the sidebar's order.
    pub const ALL: [Page; 12] = [
        Page::Appearance,
        Page::Toasts,
        Page::Details,
        Page::Confirm,
        Page::WhatsNew,
        Page::Updates,
        Page::Notices,
        Page::Offline,
        Page::Drop,
        Page::Settings,
        Page::BugReport,
        Page::Diagnostics,
    ];

    /// The sidebar entry.
    pub fn label(self) -> &'static str {
        match self {
            Page::Appearance => "Appearance",
            Page::Toasts => "Toasts",
            Page::Details => "Details dialog",
            Page::Confirm => "Confirm dialog",
            Page::WhatsNew => "What's new",
            Page::Updates => "Updates",
            Page::Notices => "Notices",
            Page::Offline => "Offline",
            Page::Drop => "Drag and drop",
            Page::Settings => "Settings",
            Page::BugReport => "Bug report",
            Page::Diagnostics => "Diagnostics",
        }
    }

    /// The sidebar entry's icon.
    pub fn icon(self) -> Icon {
        match self {
            Page::Appearance => Icon::Sun,
            Page::Toasts => Icon::Inbox,
            Page::Details => Icon::List,
            Page::Confirm => Icon::Trash,
            Page::WhatsNew => Icon::Spark,
            Page::Updates => Icon::Download,
            Page::Notices => Icon::Shield,
            Page::Offline => Icon::Link,
            Page::Drop => Icon::Plus,
            Page::Settings => Icon::Settings,
            Page::BugReport => Icon::Bug,
            Page::Diagnostics => Icon::Activity,
        }
    }
}

/// What an Undo toast gives back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Undo {
    /// Bring the archived items back.
    Archive,
}

/// What a confirm dialog asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Delete a report (a red button).
    DeleteReport,
    /// Sign out on every device (a plain one).
    SignOut,
}

/// Everything the demo's window sends.
#[derive(Clone, Debug)]
pub enum Message {
    /// Open a page.
    Go(Page),
    /// The half-second tick: toasts expire and the fake server answers.
    Tick,
    /// A demo button was pressed (a toast says which).
    Pressed(&'static str),
    /// A theme was picked (`encore`, `dark`, `oled`, `light`).
    Theme(String),
    /// An accent was picked (an id of `theme::ACCENTS`, or `""` for the theme's own).
    Accent(String),
    /// A background was picked (`solid` or `fade`).
    Background(String),
    /// Push a toast of this kind.
    Toast(ToastKind),
    /// Push the same error toast again.
    ToastRepeat,
    /// Push the Undo toast.
    ToastUndo,
    /// A toast was clicked away.
    Dismiss(u64),
    /// A toast's Copy.
    CopyToast(u64),
    /// A toast's action (Undo).
    ToastAction(u64),
    /// The clipboard read back after a copy: what was copied, what the clipboard holds.
    ClipboardRead(String, Option<String>),
    /// Open the details dialog.
    OpenDetails,
    /// A message of the details dialog.
    Details(details::Msg<Page>),
    /// The fake answer to the details dialog's action: its toast.
    ActDone(String),
    /// Open a confirm dialog.
    Ask(Ask),
    /// A message of the confirm dialog.
    Confirm(confirm::Msg),
    /// Feed the popup the fake `GET /api/changelog/new` answer.
    ShowPopup,
    /// A message of the What's new popup.
    Popup(PopupMsg),
    /// A message of the changelog page.
    Changelog(ChangelogMsg),
    /// A message of the Updates block.
    Update(UpdateMsg),
    /// The fake GitHub answer to Check for updates came.
    UpdateChecked,
    /// Step the updater to its next phase.
    NextPhase,
    /// The notice banner's Check again.
    CheckNotice,
    /// The fake answer to Check again came.
    NoticeChecked,
    /// Bring the failing notice back.
    NoticeBack,
    /// Go offline (true) or online (false).
    Offline(bool),
    /// The offline banner's Retry.
    Retry,
    /// A drag and drop event of the window (or of the demo's buttons that stand for one).
    Drop(drop_zone::Event),
    /// The settings pieces' switch.
    Switch(bool),
    /// The settings pieces' select.
    Pick(String),
    /// The settings pieces' input.
    Typed(String),
    /// Copy this text (a failed test line's Copy).
    CopyText(String),
    /// A message of the settings form.
    Form(form::Msg),
    /// Open the New report dialog.
    NewReport,
    /// Close the New report dialog.
    CloseReport,
    /// A message of the New report dialog.
    New(bug_report::NewMsg),
    /// A message of the reply box.
    Reply(bug_report::ReplyMsg),
    /// A message of the diagnostics page.
    Diag(diagnostics::Msg),
    /// The fake answer to Run checks came.
    DiagChecked,
}

/// The demo's whole state.
pub struct Demo {
    page: Page,
    theme: String,
    accent: String,
    background: String,
    palette: Palette,
    toasts: Toasts<Undo>,
    /// The fake server's answers, given on the next tick.
    later: Vec<Message>,
    details: Option<Details<Page>>,
    confirm: Option<confirm::Confirm<Ask>>,
    popup: Popup,
    changelog: Changelog,
    phase: Phase,
    found: Option<String>,
    check_at_start: bool,
    notices: Vec<Notice>,
    notice_checking: bool,
    notice_checks: u32,
    offline: bool,
    zone: DropZone,
    dropped: Vec<PathBuf>,
    switch_on: bool,
    picked: String,
    typed: String,
    schema: Schema,
    form: Form,
    /// The fake server's settings (what `GET /api/admin/settings` answers).
    server: Value,
    /// The fake server's secrets, in clear.
    secrets: HashMap<String, String>,
    new_report: Option<bug_report::NewReport>,
    sending: Option<bug_report::Report>,
    report: Value,
    reply: bug_report::Reply,
    replying: Option<String>,
    diag: diagnostics::State,
}

impl Default for Demo {
    fn default() -> Self {
        Demo::new()
    }
}

/// The theme choices of the Appearance page (the default theme is called by its id elsewhere).
const THEMES: [(&str, &str); 4] = [("encore", "Default"), ("dark", "Dark"), ("oled", "OLED black"), ("light", "Light")];
/// The background choices of the Appearance page.
const BACKGROUNDS: [(&str, &str); 2] = [("solid", "Solid"), ("fade", "Fade")];
/// The select of the settings pieces.
const SIZES: [(&str, &str); 3] = [("small", "Small"), ("medium", "Medium"), ("large", "Large")];
/// Every button look, by name.
const BTN_KINDS: [(Btn, &str); 13] = [
    (Btn::Primary, "Primary"),
    (Btn::Default, "Default"),
    (Btn::Ghost, "Ghost"),
    (Btn::Danger, "Danger"),
    (Btn::Icon, "Icon"),
    (Btn::Nav, "Nav"),
    (Btn::NavOn, "Nav (open)"),
    (Btn::Tab, "Tab"),
    (Btn::TabOn, "Tab (chosen)"),
    (Btn::Seg, "Seg"),
    (Btn::SegOn, "Seg (chosen)"),
    (Btn::Row, "Row"),
    (Btn::Play, "Play"),
];
/// Every badge look, by name.
const BADGES: [(Badge, &str); 6] = [
    (Badge::Plain, "Plain"),
    (Badge::Ok, "Ok"),
    (Badge::Warn, "Warn"),
    (Badge::Err, "Err"),
    (Badge::Info, "Info"),
    (Badge::Accent, "Accent"),
];

/// An accent's name for the picker: `""` is the theme's own accent.
pub fn accent_name(id: &str) -> &'static str {
    match id {
        "" => "Theme",
        _ => theme::ACCENTS.iter().find(|a| a.0 == id).map(|a| a.1).unwrap_or("Custom"),
    }
}

/// The fake answer of `GET /api/changelog/new`: show the popup, with one new release.
pub fn fake_changelog_new() -> Value {
    json!({ "show": true, "version": SERVER_VERSION, "releases": [
        { "version": "0.2.0", "title": "Demo 0.2", "summary": "Faster lists and a new search.",
          "highlights": ["Search finds items by any word of their notes", "Lists load twice as fast", "Settings show where each value comes from"] }
    ] })
}

/// The fake answer of `GET /api/changelog`: two releases with every kind of block.
pub fn fake_changelog() -> Value {
    json!({ "version": SERVER_VERSION, "popup": true, "releases": [
        { "version": "0.2.0", "title": "Demo 0.2", "blocks": [
            { "k": "p", "text": "Faster lists and a new search." },
            { "k": "h", "text": "New" },
            { "k": "li", "lead": "Search:", "text": "finds items by any word of their notes." },
            { "k": "li", "lead": "Settings:", "text": "each field says where its value comes from." },
            { "k": "h", "text": "Limits" },
            { "k": "row", "head": true, "cells": ["Setting", "Before", "Now"] },
            { "k": "row", "cells": ["Items per page", "25", "50"] },
            { "k": "row", "cells": ["Upload size", "4 MB", "8 MB"] }
        ] },
        { "version": "0.1.0", "title": "Demo 0.1", "blocks": [
            { "k": "p", "text": "The first release." },
            { "k": "li", "text": "Lists, settings and bug reports." }
        ] }
    ] })
}

/// The fake answer of GitHub's release list: a hotfix (passed over) and the newest full release.
pub fn fake_releases() -> Value {
    let file = |tag: &str, name: &str| json!({ "name": name, "browser_download_url": format!("{}{tag}/{name}", FEED.downloads), "size": 48_000_000 });
    json!([
        { "tag_name": "hotfix-0.2.0.1", "assets": [] },
        { "tag_name": "v0.2.0", "html_url": "https://github.com/example/demo/releases/tag/v0.2.0", "body": "Faster lists and a new search.",
          "assets": [file("v0.2.0", "Demo-0.2.0-linux-x86_64.AppImage"), file("v0.2.0", "Demo-0.2.0-windows-x86_64.exe"), file("v0.2.0", "SHA256SUMS")] }
    ])
}

/// This computer's file of a release.
fn this_computer(version: &str) -> Option<String> {
    Some(format!("Demo-{version}-linux-x86_64.AppImage"))
}

/// The fake answer of `GET /api/notices`.
pub fn fake_notices() -> Value {
    json!({ "notices": [{ "id": "storage", "ok": false, "severity": "warn", "text": NOTICE_TEXT }, { "id": "mail", "ok": true }] })
}

/// The fake settings schema (`GET /api/admin/settings/schema`): a General card and an Email card whose
/// port fields follow the Security select, with secret fields.
pub fn fake_schema() -> Value {
    json!({ "sections": [
        { "id": "server", "title": "Server", "aliases": ["general"], "groups": [
            { "id": "general", "title": "General", "part": "Server", "desc": "The name and address people see.", "test": null, "test_label": null, "fields": [
                { "key": "app_name", "label": "App name", "type": "text", "help": "Shown in the title bar and in emails" },
                { "key": "base_url", "label": "Public URL", "type": "url", "help": "e.g. https://demo.example.com – used in links" },
                { "key": "signup_mode", "label": "Sign-up mode", "type": "select", "options": [
                    { "value": "invite", "label": "Invite only" }, { "value": "open", "label": "Open to anyone" }, { "value": "closed", "label": "Closed" }] },
                { "key": "page_size", "label": "Items per page", "type": "number" },
                { "key": "auto_approve", "label": "Approve sign-ups by themselves", "type": "bool" },
                { "key": "api_token", "label": "API token (optional)", "type": "password", "secret": true, "full": true }
            ] },
            { "id": "email", "title": "Email", "part": "Services", "desc": "Lets Demo send password-reset emails.", "test": "email", "test_label": "Send test email", "fields": [
                { "key": "smtp_host", "label": "SMTP server", "type": "text", "help": "Leave it empty to see a failed test" },
                { "key": "smtp_security", "label": "Security", "type": "select", "options": [
                    { "value": "starttls", "label": "STARTTLS" }, { "value": "tls", "label": "TLS" }, { "value": "none", "label": "None" }] },
                { "key": "smtp_port", "label": "STARTTLS port", "type": "number", "when": "smtp_security=starttls" },
                { "key": "tls_port", "label": "TLS port", "type": "number", "when": "smtp_security=tls" },
                { "key": "smtp_user", "label": "User name", "type": "text", "when": "smtp_security!=none" },
                { "key": "smtp_password", "label": "SMTP password", "type": "password", "secret": true, "when": "smtp_security!=none" },
                { "key": "notify_events", "label": "Mail about", "type": "checks", "options": [
                    { "value": "Sign-up", "label": "Sign-up" }, { "value": "Login", "label": "Login" }, { "value": "Error", "label": "Error" }] }
            ] }
        ] }
    ] })
}

/// The fake settings (`GET /api/admin/settings`), with `_meta` saying where values come from.
pub fn fake_settings() -> Value {
    json!({
        "app_name": "Demo", "base_url": "https://demo.example.com", "signup_mode": "invite", "page_size": 50, "auto_approve": false,
        "api_token": "", "smtp_host": "smtp.example.com", "smtp_security": "starttls", "smtp_port": 587, "tls_port": 465,
        "smtp_user": "demo", "smtp_password": "•••• (saved)", "notify_events": "Sign-up,Error",
        "_meta": {
            "base_url": { "source": "env", "env": "DEMO_BASE_URL" },
            "page_size": { "source": "default", "env": "DEMO_PAGE_SIZE" },
            "api_token": { "env": "DEMO_API_TOKEN" },
            "app_name": { "source": "ui" }
        }
    })
}

/// The fake diagnostics answer: one check passed, one needs a look, one failed.
pub fn fake_checks() -> Value {
    json!({ "checks": [
        { "name": "Database", "kind": "service", "ok": true, "status": "ok", "request": "SELECT 1", "detail": "Answered in 2 ms", "ms": 2 },
        { "name": "Mail server", "kind": "service", "ok": false, "request": "SMTP smtp.example.com:587", "detail": "Connection timed out after 10 s",
          "status": 504, "ms": 10000, "body": "connect ETIMEDOUT 203.0.113.7:587", "fix": "Check the SMTP server and port under Settings → Email." },
        { "name": "Data folder", "kind": "path", "ok": true, "status": "warn", "detail": "Writable, 92% full", "fix": "Free some space on the disk." },
        { "name": "Image tools", "kind": "tool", "ok": true, "status": "ok", "detail": "Found, version 7.1" }
    ] })
}

/// The fake report the Bug report page shows.
pub fn fake_report() -> Value {
    json!({ "id": 7, "kind": "bug", "status": "looking", "title": "The list jumps to the top after a save",
        "username": "sam", "created_at": 1_792_311_210.0, "page": "#/items", "app_version": SERVER_VERSION,
        "device": "Demo desktop · 1280x800", "files": 1,
        "messages": [
            { "username": "sam", "body": "After I save an item, the list scrolls back to the top. Expected it to stay where I was." },
            { "username": "admin", "body": "Thanks – I can see it too. Looking into it." }
        ] })
}

/// The device line a new report carries.
fn device() -> String {
    bug_report::device_line("Demo-Desktop/0.1 (Linux)", 1280.0, 800.0)
}

/// Copy `text` to the clipboard, then read it back to say whether it took.
fn copy(text: String) -> Task<Message> {
    iced::clipboard::write::<Message>(text.clone()).chain(iced::clipboard::read().map(move |got| Message::ClipboardRead(text.clone(), got)))
}

impl Demo {
    /// The demo on its first page, with every component fed its fake data.
    pub fn new() -> Demo {
        let schema = Schema::from_json(&fake_schema()).unwrap_or_default();
        let mut form = Form::new();
        form.open(schema.cards().map(|c| c.id.to_string()), None);
        let server = fake_settings();
        form.update(form::Msg::Values(Ok(server.clone())), schema.cards());
        let mut changelog = Changelog::default();
        changelog.reset();
        changelog.loaded(Ok(fake_changelog()));
        let mut diag = diagnostics::State::default();
        diag.checked(Ok(fake_checks()));
        let palette = theme::palette("encore", "", "", "solid", true);
        theme::set(palette);
        Demo {
            page: Page::default(),
            theme: "encore".into(),
            accent: String::new(),
            background: "solid".into(),
            palette,
            toasts: Toasts::default(),
            later: Vec::new(),
            details: None,
            confirm: None,
            popup: Popup::default(),
            changelog,
            phase: Phase::Idle,
            found: None,
            check_at_start: true,
            notices: notice::list(&fake_notices()),
            notice_checking: false,
            notice_checks: 0,
            offline: false,
            zone: DropZone::default(),
            dropped: Vec::new(),
            switch_on: true,
            picked: "medium".into(),
            typed: String::new(),
            schema,
            form,
            server,
            secrets: HashMap::from([("smtp_password".to_string(), "demo-password".to_string())]),
            new_report: None,
            sending: None,
            report: fake_report(),
            reply: bug_report::Reply::default(),
            replying: None,
            diag,
        }
    }

    /// The page open.
    pub fn page(&self) -> Page {
        self.page
    }

    /// The look the demo set (the theme, accent and background picked).
    pub fn palette(&self) -> Palette {
        self.palette
    }

    /// The toasts on screen.
    pub fn toasts(&self) -> &Toasts<Undo> {
        &self.toasts
    }

    /// The open details dialog.
    pub fn details(&self) -> Option<&Details<Page>> {
        self.details.as_ref()
    }

    /// Whether a confirm dialog is open.
    pub fn confirm_open(&self) -> bool {
        self.confirm.is_some()
    }

    /// Whether the What's new popup is open.
    pub fn popup_open(&self) -> bool {
        self.popup.is_open()
    }

    /// Whether the New report dialog is open.
    pub fn new_report_open(&self) -> bool {
        self.new_report.is_some()
    }

    /// The updater's phase.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// The files dropped on the drag and drop page.
    pub fn dropped(&self) -> &[PathBuf] {
        &self.dropped
    }

    /// The report the Bug report page shows.
    pub fn report(&self) -> &Value {
        &self.report
    }

    fn apply_look(&mut self) {
        self.palette = theme::palette(&self.theme, &self.accent, "", &self.background, true);
        theme::set(self.palette);
    }

    fn ok(&mut self, words: impl Into<String>) {
        self.toasts.push(words, ToastKind::Ok);
    }

    fn err(&mut self, words: impl Into<String>) {
        self.toasts.push(words, ToastKind::Err);
    }

    /// Whether a setting is a secret the fake server keeps masked.
    fn is_secret(&self, key: &str) -> bool {
        self.schema.cards().flat_map(|c| c.fields.iter()).any(|f| f.key == key && f.is_secret())
    }

    /// The fake server saves `body`: secrets are kept in clear and answered masked.
    fn save(&mut self, body: Map<String, Value>) -> Value {
        for (k, v) in body {
            if self.is_secret(&k) {
                let clear = form::vs(&v);
                let masked = if clear.is_empty() { String::new() } else { "•••• (saved)".to_string() };
                if clear.is_empty() {
                    self.secrets.remove(&k);
                } else {
                    self.secrets.insert(k.clone(), clear);
                }
                self.server[&k] = Value::String(masked);
            } else {
                self.server[&k] = v;
            }
        }
        self.server.clone()
    }

    /// The fake test of a card: the email test passes while an SMTP server is set.
    fn test(&self, test: &str) -> Result<Value, String> {
        match test {
            "email" if form::vs(&self.server["smtp_host"]).trim().is_empty() => {
                Ok(json!({ "ok": false, "message": "No SMTP server is set, so no email could be sent" }))
            }
            "email" => Ok(json!({ "ok": true, "message": format!("Test email sent through {}", form::vs(&self.server["smtp_host"])) })),
            other => Err(format!("No test called {other}")),
        }
    }

    /// Apply a message; the task is a clipboard copy when there is one.
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Go(p) => self.page = p,
            Message::Tick => {
                self.toasts.expire();
                let answers = std::mem::take(&mut self.later);
                return Task::batch(answers.into_iter().map(|m| self.update(m)));
            }
            Message::Pressed(what) => self.ok(format!("{what} pressed")),
            Message::Theme(t) => {
                self.theme = t;
                self.apply_look();
            }
            Message::Accent(a) => {
                self.accent = a;
                self.apply_look();
            }
            Message::Background(b) => {
                self.background = b;
                self.apply_look();
            }
            Message::Toast(kind) => {
                let words = match kind {
                    ToastKind::Err => ERROR_TEXT,
                    ToastKind::Warn => "The disk is 92% full",
                    ToastKind::Info => "A new version is being prepared",
                    _ => "Saved",
                };
                self.toasts.push(words, kind);
            }
            Message::ToastRepeat => self.err(ERROR_TEXT),
            Message::ToastUndo => self.toasts.push_action(UNDO_TEXT, "Undo", Undo::Archive, Duration::from_secs(6)),
            Message::Dismiss(id) => self.toasts.dismiss(id),
            Message::CopyToast(id) => {
                if let Some(text) = self.toasts.text(id) {
                    return copy(text);
                }
            }
            Message::ToastAction(id) => {
                if let Some(Undo::Archive) = self.toasts.take_action(id) {
                    self.ok(UNDONE_TEXT);
                }
            }
            Message::ClipboardRead(want, got) => {
                let (words, kind) = kit_rules::toasts::copy_toast(details::copied(&want, got.as_deref()), details::COPIED);
                self.toasts.push(words, kind);
            }
            Message::OpenDetails => {
                let lead = Lead { text: DETAILS_LEAD.into(), from: "Checked by the import step".into() };
                let act = Act {
                    label: "Keep anyway".into(),
                    path: "/api/items/import/keep".into(),
                    body: json!({ "file": "report.zip" }),
                    done: "The first 14 files are kept".into(),
                };
                let links = Links {
                    title: Some(("report.zip".into(), Page::Drop)),
                    subtitle: Some(("Uploads".into(), Page::Drop)),
                    button: Some(("Open diagnostics".into(), Page::Diagnostics)),
                };
                self.details = Some(Details::new(DETAILS_TITLE.into(), DETAILS_TEXT.into(), Some(lead), Some(act)).with_links(links));
            }
            Message::Details(m) => {
                let close = matches!(m, details::Msg::Close);
                let out = self.details.as_mut().and_then(|d| d.update(m));
                if close {
                    self.details = None;
                }
                match out {
                    Some(details::Out::Copy(text)) => return copy(text),
                    Some(details::Out::Act(a)) => {
                        // the app posts `a.body` to `a.path`; the fake server says yes
                        self.details = None;
                        self.later.push(Message::ActDone(a.done));
                    }
                    Some(details::Out::Open(p)) => {
                        self.details = None;
                        self.page = p;
                    }
                    None => {}
                }
            }
            Message::ActDone(words) => self.ok(words),
            Message::Ask(Ask::DeleteReport) => {
                self.confirm = Some(
                    confirm::Confirm::new("Delete report?", "It's gone for everyone. This can't be undone.", "Delete", Ask::DeleteReport)
                        .danger(),
                )
            }
            Message::Ask(Ask::SignOut) => {
                self.confirm = Some(confirm::Confirm::new("Sign out everywhere?", "Every device signs in again.", "Sign out", Ask::SignOut))
            }
            Message::Confirm(m) => {
                let action = self.confirm.take().and_then(|c| c.update(m));
                match action {
                    Some(Ask::DeleteReport) => self.ok(DELETED),
                    Some(Ask::SignOut) => self.ok("Signed out on every device"),
                    None => {}
                }
            }
            Message::ShowPopup => {
                // the app would now send POST /api/changelog/seen
                self.popup.loaded(&fake_changelog_new());
            }
            Message::Popup(m) => match m {
                PopupMsg::Close => self.popup.close(),
                PopupMsg::Full => {
                    self.popup.close();
                    self.page = Page::WhatsNew;
                }
                PopupMsg::Popup(on) => {
                    self.popup.set_on(on);
                    self.ok(whats_new::saved_words(on));
                }
            },
            Message::Changelog(m) => {
                if let Some(on) = self.changelog.update(m) {
                    self.ok(whats_new::saved_words(on));
                }
            }
            Message::Update(m) => match m {
                UpdateMsg::CheckAtStart(on) => self.check_at_start = on,
                UpdateMsg::Check => {
                    self.phase = Phase::Checking;
                    self.later.push(Message::UpdateChecked);
                }
                UpdateMsg::Install => {
                    if let Some(version) = self.found.clone() {
                        self.phase = Phase::Downloading { version, got: 0, total: 48_000_000 };
                    }
                }
                UpdateMsg::OpenReleases => self.ok(format!("This opens {}", FEED.page)),
            },
            Message::UpdateChecked => {
                let newest = update::full_release(&fake_releases())
                    .map(|r| update::newest(&r, BUILD, &FEED, this_computer))
                    .unwrap_or(Err("Couldn't check for updates: no release found".into()));
                match newest {
                    Ok(Some(r)) => {
                        self.ok(update::ready_words(APP, &r.version, "Settings → Updates"));
                        self.found = Some(r.version.clone());
                        self.phase = Phase::Available(r);
                    }
                    Ok(None) => self.phase = Phase::UpToDate,
                    Err(e) => self.phase = Phase::Failed(e),
                }
            }
            Message::NextPhase => {
                self.phase = next_phase(&self.phase);
                match &self.phase {
                    Phase::Available(r) => self.found = Some(r.version.clone()),
                    Phase::Idle => self.found = None,
                    _ => {}
                }
            }
            Message::CheckNotice => {
                // the app posts POST /api/notices/check; the answer comes on the next tick
                self.notice_checking = true;
                self.later.push(Message::NoticeChecked);
            }
            Message::NoticeChecked => {
                self.notice_checking = false;
                self.notice_checks += 1;
                if let Some(n) = self.notices.iter_mut().find(|n| n.id == "storage") {
                    // first still failing, then still checking, then fixed
                    match self.notice_checks {
                        1 => {}
                        2 => {
                            n.ok = true;
                            n.checking = true;
                        }
                        _ => {
                            n.ok = true;
                            n.checking = false;
                        }
                    }
                    let (words, kind) = notice::check_toast(n, &NOTICE_WORDS);
                    self.toasts.push(words, kind);
                    if self.notice_checks == 2 {
                        // still checking: the banner stays until the check ends
                        n.ok = false;
                    }
                }
            }
            Message::NoticeBack => {
                self.notices = notice::list(&fake_notices());
                self.notice_checks = 0;
            }
            Message::Offline(on) => self.offline = on,
            Message::Retry => {
                self.offline = false;
                self.ok("The server answers again");
            }
            Message::Drop(ev) => {
                if self.new_report.is_some() {
                    return self.update(Message::New(bug_report::NewMsg::Drop(ev)));
                }
                if self.page == Page::BugReport {
                    return self.update(Message::Reply(bug_report::ReplyMsg::Drop(ev)));
                }
                if self.page != Page::Drop {
                    return Task::none();
                }
                match self.zone.handle(ev, &IMAGES) {
                    Outcome::Take(p) => {
                        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        self.ok(format!("{name} added"));
                        self.dropped.push(p);
                    }
                    Outcome::Refused(m) => self.err(m),
                    Outcome::Nothing => {}
                }
            }
            Message::Switch(on) => self.switch_on = on,
            Message::Pick(k) => self.picked = k,
            Message::Typed(s) => self.typed = s,
            Message::CopyText(text) => return copy(text),
            Message::Form(m) => {
                let outs = self.form.update(m, self.schema.cards());
                let mut tasks = Vec::new();
                for out in outs {
                    match out {
                        form::Out::Save { card, body } => {
                            let saved = self.save(body);
                            self.later.push(Message::Form(form::Msg::Saved(card, Ok(saved))));
                        }
                        form::Out::Test { card, test, body } => {
                            let saved = self.save(body);
                            let tested = self.test(&test);
                            self.later.push(Message::Form(form::Msg::Tested(card, Ok((saved, tested)))));
                        }
                        form::Out::Clear { key: _, body } => {
                            let saved = self.save(body);
                            self.later.push(Message::Form(form::Msg::Cleared(Ok(saved))));
                        }
                        form::Out::Reveal { key } => {
                            let value = self.secrets.get(&key).cloned().unwrap_or_default();
                            self.later.push(Message::Form(form::Msg::Revealed(key, Ok(json!({ "value": value })))));
                        }
                        form::Out::CopySecret { key } => {
                            let value = self.secrets.get(&key).cloned().unwrap_or_default();
                            self.later.push(Message::Form(form::Msg::Copied(Ok(json!({ "value": value })))));
                        }
                        form::Out::Copy(text) => tasks.push(iced::clipboard::write(text)),
                        form::Out::CopyText(text) => tasks.push(copy(text)),
                        form::Out::ToastOk(words) => self.ok(words),
                        form::Out::ToastErr(words) => self.err(words),
                    }
                }
                return Task::batch(tasks);
            }
            Message::NewReport => {
                self.new_report = Some(bug_report::NewReport { page: "#/items".into(), ..bug_report::NewReport::default() });
            }
            Message::CloseReport => self.new_report = None,
            Message::New(m) => {
                let Some(d) = self.new_report.as_mut() else { return Task::none() };
                for out in bug_report::new_update(d, m, SERVER_VERSION, &device()) {
                    match out {
                        // no file dialog in the demo: it answers with two made-up files, one of them too big
                        bug_report::NewOut::PickFiles => self.later.push(Message::New(bug_report::NewMsg::Picked(fake_picked()))),
                        bug_report::NewOut::Error(e) => self.err(e),
                        bug_report::NewOut::Done(words) => self.ok(words),
                        bug_report::NewOut::Send(r) => {
                            self.sending = Some(r);
                            self.later.push(Message::New(bug_report::NewMsg::Sent(Ok(8))));
                        }
                        bug_report::NewOut::Show(id) => {
                            self.new_report = None;
                            if let Some(r) = self.sending.take() {
                                self.report = json!({ "id": id, "kind": r.kind, "status": "new", "title": r.title, "username": "you",
                                    "created_at": 1_793_000_000.0, "page": r.page, "app_version": r.app_version, "device": r.device,
                                    "files": r.files.len(), "messages": [{ "username": "you", "body": r.body }] });
                            }
                            self.page = Page::BugReport;
                        }
                    }
                }
            }
            Message::Reply(m) => {
                for out in bug_report::reply_update(&mut self.reply, m) {
                    match out {
                        bug_report::ReplyOut::PickFiles => self.later.push(Message::Reply(bug_report::ReplyMsg::Picked(fake_picked()))),
                        bug_report::ReplyOut::Error(e) => self.err(e),
                        bug_report::ReplyOut::Done(words) => self.ok(words),
                        bug_report::ReplyOut::Send(post) => {
                            self.replying = Some(post.body);
                            self.later.push(Message::Reply(bug_report::ReplyMsg::Sent(Ok(()))));
                        }
                        bug_report::ReplyOut::Reload => {
                            if let (Some(body), Some(list)) = (self.replying.take(), self.report["messages"].as_array_mut()) {
                                list.push(json!({ "username": "you", "body": body }));
                            }
                        }
                    }
                }
            }
            Message::Diag(m) => match self.diag.update(m) {
                Some(diagnostics::Action::Run) => self.later.push(Message::DiagChecked),
                Some(diagnostics::Action::CopyReport) => {
                    let server = diagnostics::Section { title: "Server".into(), lines: diagnostics::fact_lines(&facts()) };
                    let first = format!("{APP} {SERVER_VERSION} diagnostics (uid 1000)");
                    return copy(diagnostics::report(&first, self.diag.all(), &[server]));
                }
                Some(diagnostics::Action::Copy(text)) => return copy(text),
                None => {}
            },
            Message::DiagChecked => self.diag.checked(Ok(fake_checks())),
        }
        Task::none()
    }

    /// The window: the sidebar, the open page (under the offline banner while offline), the dialogs and the toasts.
    pub fn view(&self) -> El<'_, Message> {
        let mut main = column![].spacing(0).width(Length::Fill).height(Length::Fill);
        if self.offline {
            main = main.push(
                container(offline::banner(
                    offline::TEXT,
                    vec![("Diagnostics", false, Message::Go(Page::Diagnostics)), (offline::RETRY, true, Message::Retry)],
                ))
                .padding([8, 16]),
            );
        }
        main = main
            .push(scrollable(container(self.page_view()).padding([24, 28]).width(Length::Fill)).style(theme::scroll).height(Length::Fill));
        let shell = row![self.sidebar(), main].width(Length::Fill).height(Length::Fill);
        let mut layers = stack![container(shell).width(Length::Fill).height(Length::Fill).style(theme::app_bg)];
        if let Some(p) = whats_new::popup(&self.popup) {
            layers = layers.push(p.map(Message::Popup));
        }
        if let Some(d) = &self.details {
            layers = layers.push(details::view(d).map(Message::Details));
        }
        if let Some(c) = &self.confirm {
            layers = layers.push(confirm::view(c).map(Message::Confirm));
        }
        if let Some(d) = &self.new_report {
            layers = layers.push(bug_report::new_dialog(d, SERVER_VERSION, Message::New, Message::CloseReport));
        }
        layers = layers.push(toast::view(self.toasts.list(), 18.0, Message::Dismiss, Message::CopyToast, Message::ToastAction));
        layers.width(Length::Fill).height(Length::Fill).into()
    }

    fn sidebar(&self) -> El<'_, Message> {
        let p = theme::look();
        let mut nav =
            column![container(row![w::logo(&Logo::KIT, 28.0), w::h3(APP)].spacing(10).align_y(Alignment::Center)).padding([8, 10])]
                .spacing(2)
                .width(Length::Fill);
        for page in Page::ALL {
            let on = page == self.page;
            nav = nav.push(
                button(
                    row![w::icon(page.icon(), 16.0, if on { p.text } else { p.muted }), w::t(page.label()).size(14)]
                        .spacing(10)
                        .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .style(theme::btn(if on { Btn::NavOn } else { Btn::Nav }))
                .on_press(Message::Go(page)),
            );
        }
        container(nav).padding(12).width(theme::SIDEBAR).height(Length::Fill).style(theme::sidebar).into()
    }

    fn page_view(&self) -> El<'_, Message> {
        match self.page {
            Page::Appearance => self.appearance(),
            Page::Toasts => self.toasts_page(),
            Page::Details => self.details_page(),
            Page::Confirm => self.confirm_page(),
            Page::WhatsNew => self.whats_new_page(),
            Page::Updates => self.updates_page(),
            Page::Notices => self.notices_page(),
            Page::Offline => self.offline_page(),
            Page::Drop => self.drop_page(),
            Page::Settings => self.settings_page(),
            Page::BugReport => self.bug_report_page(),
            Page::Diagnostics => self.diagnostics_page(),
        }
    }

    fn appearance(&self) -> El<'_, Message> {
        let mut accents = row![button(w::t("Theme").size(13).font(theme::semibold()))
            .padding([4, 10])
            .style(theme::btn(if self.accent.is_empty() { Btn::Primary } else { Btn::Default }))
            .on_press(Message::Accent(String::new()))]
        .spacing(8)
        .align_y(Alignment::Center);
        for (id, _, a, b) in theme::ACCENTS {
            let (a, b) = (theme::hex(a), theme::hex(b));
            let chosen = self.accent == id;
            let ring = self.palette.text;
            let swatch = container(space()).width(26).height(26).style(move |_| container::Style {
                background: Some(Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::FRAC_PI_4).add_stop(0.0, a).add_stop(1.0, b),
                ))),
                border: Border { color: if chosen { ring } else { iced::Color::TRANSPARENT }, width: 2.0, radius: 99.0.into() },
                ..container::Style::default()
            });
            let b = button(swatch).padding(0).style(theme::btn(Btn::Row)).on_press(Message::Accent(id.to_string()));
            accents = accents.push(tooltip(b, w::tip_box(accent_name(id)), tooltip::Position::Bottom).gap(4));
        }
        let mut kinds = row![].spacing(8).align_y(Alignment::Center);
        for (kind, name) in BTN_KINDS {
            kinds = kinds.push(
                button(w::t(name).size(13).font(theme::semibold()))
                    .padding([8, 14])
                    .style(theme::btn(kind))
                    .on_press(Message::Pressed(name)),
            );
        }
        let helpers = row![
            w::btn("Primary button", Btn::Primary, Some(Message::Pressed("Primary button"))),
            w::btn("Disabled", Btn::Default, None),
            w::btn_sm("Small", Btn::Default, Some(Message::Pressed("Small"))),
            w::btn_icon(Icon::Plus, "With an icon", Btn::Default, Some(Message::Pressed("With an icon"))),
            w::icon_btn(Icon::Settings, "An icon button", Some(Message::Pressed("Icon button"))),
            w::link("A link", 13.0, Message::Pressed("A link")),
            w::muted_link("A muted link", 13.0, Message::Pressed("A muted link")),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let mut badges = row![].spacing(8);
        for (kind, name) in BADGES {
            badges = badges.push(w::badge(name, kind));
        }
        let mut icons = row![].spacing(10);
        for i in Icon::ALL {
            icons = icons.push(
                column![w::icon(i, 20.0, self.palette.text), w::small_muted(i.name())].spacing(4).align_x(Alignment::Center).width(64),
            );
        }
        let card = w::card(
            column![
                row![w::avatar("sam", 36.0), column![w::h3("A card"), w::muted("A bordered panel with 18px padding.")].spacing(2)]
                    .spacing(12)
                    .align_y(Alignment::Center),
                w::divider(),
                w::field("A field", settings::input("Type here", &self.typed, Message::Typed)),
                w::error_box("An error box says what went wrong."),
            ]
            .spacing(12),
        )
        .width(Length::Fill);
        column![
            w::page_head("Appearance", Some(w::muted("Every theme and accent, and the pieces every screen is built from.").into()), vec![]),
            section("Theme", w::tabs(&THEMES, &self.theme, Message::Theme)),
            section("Background", w::seg(&BACKGROUNDS, &self.background, Message::Background)),
            section("Accent", column![accents.wrap(), w::small_muted(format!("Accent: {}", accent_name(&self.accent)))].spacing(8)),
            section(
                "Text",
                column![
                    w::h1("Heading 1"),
                    w::h2("Heading 2"),
                    w::h3("Heading 3"),
                    w::t("Body text"),
                    w::muted("Muted text"),
                    w::faint("Faint text"),
                    w::small("Small text")
                ]
                .spacing(4)
            ),
            section("Button looks", kinds.wrap()),
            section("Buttons", helpers.wrap()),
            section("Badges", badges.wrap()),
            section("Card", card),
            section("Skeletons", column![w::skeleton_rows(2), w::skeleton_tiles(3, 90.0)].spacing(16)),
            section("Icons", icons.wrap()),
            section(
                "Logo",
                row![w::logo(&Logo::KIT, 48.0), w::muted("Logo::KIT in the accent gradient")].spacing(12).align_y(Alignment::Center)
            ),
        ]
        .spacing(22)
        .into()
    }

    fn toasts_page(&self) -> El<'_, Message> {
        column![
            w::page_head("Toasts", Some(w::muted("Bottom right. Errors stay with Copy; the rest go after four seconds.").into()), vec![]),
            row![
                w::btn("Show a success toast", Btn::Primary, Some(Message::Toast(ToastKind::Ok))),
                w::btn("Show an error", Btn::Danger, Some(Message::Toast(ToastKind::Err))),
                w::btn("Show the same error again", Btn::Default, Some(Message::ToastRepeat)),
                w::btn("Show an Undo toast", Btn::Default, Some(Message::ToastUndo)),
            ]
            .spacing(8)
            .wrap(),
            row![
                w::btn_sm("Show a warning", Btn::Default, Some(Message::Toast(ToastKind::Warn))),
                w::btn_sm("Show an info", Btn::Default, Some(Message::Toast(ToastKind::Info))),
            ]
            .spacing(8),
            w::small_muted(format!(
                "{} on screen. At most {} errors stay and {} toasts show; the same error again does not pile up.",
                self.toasts.list().len(),
                toast::STAYING,
                Toasts::<Undo>::MAX
            )),
        ]
        .spacing(16)
        .into()
    }

    fn details_page(&self) -> El<'_, Message> {
        column![
            w::page_head("Details dialog", Some(w::muted("Any text in full, selectable, with Copy, an action and links.").into()), vec![]),
            row![w::btn("Open details", Btn::Primary, Some(Message::OpenDetails))].spacing(8),
            w::error_box("Import failed – report.zip is not a complete archive."),
        ]
        .spacing(16)
        .into()
    }

    fn confirm_page(&self) -> El<'_, Message> {
        column![
            w::page_head(
                "Confirm dialog",
                Some(w::muted("A question before an action that matters; red when it destroys something.").into()),
                vec![]
            ),
            row![
                w::btn("Delete report…", Btn::Danger, Some(Message::Ask(Ask::DeleteReport))),
                w::btn("Sign out everywhere…", Btn::Default, Some(Message::Ask(Ask::SignOut))),
            ]
            .spacing(8),
        ]
        .spacing(16)
        .into()
    }

    fn whats_new_page(&self) -> El<'_, Message> {
        column![
            w::page_head(
                "What's new",
                Some(w::muted("The popup after an update, and the changelog page.").into()),
                vec![w::btn("Show the popup", Btn::Primary, Some(Message::ShowPopup)).into()]
            ),
            whats_new::changelog(&self.changelog, APP).map(Message::Changelog),
        ]
        .spacing(16)
        .into()
    }

    fn updates_page(&self) -> El<'_, Message> {
        let block: El<'_, UpdateMsg> =
            settings::card("Updates", update::lines(APP, BUILD, self.check_at_start, &self.phase, self.found.as_deref()));
        let mut words = column![].spacing(6);
        for (name, p) in sample_phases() {
            words = words.push(
                row![container(w::badge(name, Badge::Plain)).width(120), settings::help(update::describe(&p, APP))]
                    .spacing(10)
                    .align_y(Alignment::Center),
            );
        }
        column![
            w::page_head("Updates", Some(w::muted("The newest full release, checked against its SHA256SUMS.").into()), vec![]),
            block.map(Message::Update),
            row![
                w::btn_sm("Next phase", Btn::Default, Some(Message::NextPhase)),
                w::small_muted(format!("Now: {}", phase_name(&self.phase)))
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            w::h2("The words of every phase"),
            words,
        ]
        .spacing(16)
        .into()
    }

    fn notices_page(&self) -> El<'_, Message> {
        let lines: Vec<String> = notice::banners(&self.notices, true).iter().filter_map(|n| n.banner()).map(String::from).collect();
        let banner = notice::banner(Icon::Shield, lines, Some((self.notice_checking, Message::CheckNotice)));
        let mut page = column![w::page_head(
            "Notices",
            Some(w::muted("A plain sentence when something on the server is wrong, with Check again.").into()),
            vec![]
        )]
        .spacing(16);
        match banner {
            Some(b) => page = page.push(b),
            None => {
                page = page.push(w::muted("No notices – everything is fine."));
                page = page.push(row![w::btn("Bring the notice back", Btn::Default, Some(Message::NoticeBack))]);
            }
        }
        page.into()
    }

    fn offline_page(&self) -> El<'_, Message> {
        let toggle = if self.offline {
            w::btn("Go online", Btn::Default, Some(Message::Offline(false)))
        } else {
            w::btn("Go offline", Btn::Primary, Some(Message::Offline(true)))
        };
        column![
            w::page_head("Offline", Some(w::muted("While the server can't be reached, a line over every page says so.").into()), vec![]),
            row![toggle],
        ]
        .spacing(16)
        .into()
    }

    fn drop_page(&self) -> El<'_, Message> {
        let mut files = column![].spacing(6);
        for f in &self.dropped {
            files = files.push(
                row![
                    w::icon(Icon::Check, 14.0, self.palette.ok),
                    w::t(f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()).size(13)
                ]
                .spacing(8),
            );
        }
        if self.dropped.is_empty() {
            files = files.push(w::small_muted("Nothing dropped yet."));
        }
        let zone = drop_zone::view(
            w::card(
                column![
                    w::h3("Images"),
                    w::muted(format!("Drag files from your computer onto this window. Takes {}.", IMAGES.hint())),
                    files
                ]
                .spacing(10),
            )
            .width(Length::Fill),
            &self.zone,
            DROP_HINT,
        );
        column![
            w::page_head(
                "Drag and drop",
                Some(w::muted("Files dropped on the window go to the open page's upload, checked like the file dialog.").into()),
                vec![]
            ),
            zone,
            w::small_muted("No files at hand? These buttons send the same events as the window:"),
            row![
                w::btn_sm("Hold a file over the window", Btn::Default, Some(Message::Drop(drop_zone::Event::Hovered))),
                w::btn_sm("Take it away", Btn::Default, Some(Message::Drop(drop_zone::Event::Left))),
                w::btn_sm(
                    "Drop photo.png",
                    Btn::Default,
                    Some(Message::Drop(drop_zone::Event::Dropped(PathBuf::from("/home/demo/photo.png"))))
                ),
                w::btn_sm(
                    "Drop notes.txt",
                    Btn::Default,
                    Some(Message::Drop(drop_zone::Event::Dropped(PathBuf::from("/home/demo/notes.txt"))))
                ),
            ]
            .spacing(8)
            .wrap(),
        ]
        .spacing(16)
        .into()
    }

    fn settings_page(&self) -> El<'_, Message> {
        let pieces = settings::card(
            "Pieces",
            vec![
                settings::help("A help line under a heading.").into(),
                settings::switch(self.switch_on, "A switch", Message::Switch),
                settings::switch_maybe::<Message, fn(bool) -> Message>(true, "A switch set by an environment variable", None),
                settings::two(
                    w::field("A select", settings::select_table(&SIZES, &self.picked, Message::Pick)),
                    Some(w::field("An input", settings::input("Type here", &self.typed, Message::Typed))),
                ),
                settings::two(w::field("A locked input", settings::input_locked("set by DEMO_REGION")), None),
                settings::between(w::t("A row with a value on the right").size(14).into(), w::badge("on", Badge::Ok)),
                settings::result_line(true, "A test that passed"),
                settings::result_line_copy(false, "A test that failed: connection timed out", Message::CopyText),
                settings::warn_text("A line in the warning colour").into(),
                settings::err_text("A line in the error colour").into(),
                settings::block("A block", vec![settings::help("Lines under a small heading.").into()]),
                row![
                    settings::link("A link", Message::Pressed("A link")),
                    settings::link_muted("a muted link", Message::Pressed("A muted link"))
                ]
                .spacing(8)
                .into(),
            ],
        );
        let mut page = column![w::page_head(
            "Settings",
            Some(w::muted("A form drawn from the server's schema, and the pieces of a settings screen.").into()),
            vec![]
        ),]
        .spacing(14);
        for s in &self.schema.sections {
            page = page.push(w::h2(s.title.as_str()));
            for (part, cards) in s.parts() {
                if !part.is_empty() {
                    page = page.push(form::part_heading(part));
                }
                for c in cards {
                    page = page.push(self.form.card_view(c, true, Message::Form, form::Slots::default()));
                }
            }
        }
        page.push(pieces).into()
    }

    fn bug_report_page(&self) -> El<'_, Message> {
        let r = &self.report;
        let text = |k: &str| r[k].as_str().unwrap_or_default().to_string();
        let messages = r["messages"].as_array().cloned().unwrap_or_default();
        let mut thread = column![].spacing(12);
        for m in &messages {
            let who = m["username"].as_str().unwrap_or("?").to_string();
            thread = thread.push(
                row![
                    w::avatar(&who, 30.0),
                    column![
                        w::small_muted(who.clone()).font(theme::semibold()),
                        w::t(m["body"].as_str().unwrap_or_default().to_string()).size(14)
                    ]
                    .spacing(2)
                ]
                .spacing(10),
            );
        }
        let mut statuses = row![].spacing(6);
        for (k, _, _) in bug_report::STATUSES {
            statuses = statuses.push(bug_report::status_badge(k));
        }
        let mut kinds = row![].spacing(6);
        for (k, _) in bug_report::KINDS {
            kinds = kinds.push(bug_report::kind_badge(k));
        }
        let files = r["files"].as_i64().unwrap_or(0);
        column![
            w::page_head(
                "Bug report",
                Some(w::muted("The New report dialog, a report's thread and its reply box.").into()),
                vec![w::btn_icon(Icon::Plus, "Write a report", Btn::Primary, Some(Message::NewReport)).into()]
            ),
            w::card(
                column![
                    row![w::h2(text("title")), bug_report::kind_badge(&text("kind")), bug_report::status_badge(&text("status"))]
                        .spacing(10)
                        .align_y(Alignment::Center)
                        .wrap(),
                    w::small_muted(bug_report::meta_line(r)),
                    w::small_muted(bug_report::count_words(messages.len().saturating_sub(1) as i64, files)),
                    w::divider(),
                    thread,
                ]
                .spacing(10)
            )
            .width(Length::Fill),
            bug_report::reply_view(&self.reply).map(Message::Reply),
            w::h3("Every kind and status"),
            kinds.wrap(),
            statuses.wrap(),
        ]
        .spacing(16)
        .into()
    }

    fn diagnostics_page(&self) -> El<'_, Message> {
        let about = format!("{APP} {SERVER_VERSION} · running as uid 1000");
        column![
            diagnostics::view(&self.diag, DIAG_HELP, about).map(Message::Diag),
            w::section_head("Server", None),
            diagnostics::facts(facts().into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()),
        ]
        .spacing(16)
        .into()
    }
}

/// A titled block of a page.
fn section<'a>(title: &'a str, body: impl Into<El<'a, Message>>) -> El<'a, Message> {
    column![w::h3(title), body.into()].spacing(10).width(Length::Fill).into()
}

/// The server facts of the diagnostics page and its report.
fn facts() -> Vec<(&'static str, &'static str)> {
    vec![("Version", SERVER_VERSION), ("Running as", "uid 1000"), ("CPUs", "4"), ("Memory", "8.0 GB (container limit)")]
}

/// The files the demo's "file dialog" answers with: a picture and one over the size limit.
fn fake_picked() -> Vec<bug_report::Picked> {
    vec![(PathBuf::from("/home/demo/screenshot.png"), 240_000), (PathBuf::from("/home/demo/recording.gif"), 9_000_000)]
}

/// The release the update check finds, for the phases that need one.
fn fake_release() -> update::Release {
    update::full_release(&fake_releases())
        .and_then(|r| update::newest(&r, BUILD, &FEED, this_computer).ok().flatten())
        .unwrap_or(update::Release { version: "0.2.0".into(), page: FEED.page.into(), notes: String::new(), asset: None, sums: None })
}

/// A phase's name.
fn phase_name(p: &Phase) -> &'static str {
    match p {
        Phase::Idle => "Idle",
        Phase::Checking => "Checking",
        Phase::UpToDate => "UpToDate",
        Phase::Available(_) => "Available",
        Phase::Downloading { .. } => "Downloading",
        Phase::Verifying(_) => "Verifying",
        Phase::Installing(_) => "Installing",
        Phase::Failed(_) => "Failed",
    }
}

/// One of each phase, for the words table.
fn sample_phases() -> Vec<(&'static str, Phase)> {
    let v = "0.2.0".to_string();
    let phases = vec![
        Phase::Checking,
        Phase::UpToDate,
        Phase::Available(fake_release()),
        Phase::Downloading { version: v.clone(), got: 12_000_000, total: 48_000_000 },
        Phase::Downloading { version: v.clone(), got: 12_000_000, total: 0 },
        Phase::Verifying(v.clone()),
        Phase::Installing(v),
        Phase::Failed(failed_words()),
    ];
    phases.into_iter().map(|p| (phase_name(&p), p)).collect()
}

/// The words of a download that does not match its SHA256SUMS.
fn failed_words() -> String {
    let name = "Demo-0.2.0-linux-x86_64.AppImage";
    let sums = format!("{}  {name}\n", "a".repeat(64));
    update::verify(Some(&sums), name, &"b".repeat(64)).err().unwrap_or_default()
}

/// The phase after `p`, to step through them all.
fn next_phase(p: &Phase) -> Phase {
    let v = "0.2.0".to_string();
    match p {
        Phase::Idle => Phase::Checking,
        Phase::Checking => Phase::UpToDate,
        Phase::UpToDate => Phase::Available(fake_release()),
        Phase::Available(r) => Phase::Downloading { version: r.version.clone(), got: 12_000_000, total: 48_000_000 },
        Phase::Downloading { version, got, total } if got < total => {
            Phase::Downloading { version: version.clone(), got: (got + 12_000_000).min(*total), total: *total }
        }
        Phase::Downloading { version, .. } => Phase::Verifying(version.clone()),
        Phase::Verifying(_) => Phase::Installing(v),
        Phase::Installing(_) => Phase::Failed(failed_words()),
        Phase::Failed(_) => Phase::Idle,
    }
}
