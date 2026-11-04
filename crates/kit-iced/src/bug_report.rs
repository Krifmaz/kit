//! The bug report sheet and the report labels, from Encore's desktop (`pages/bug_reports.rs` and
//! `pages/bug_report.rs`).
//!
//! - The words of a report: [`KINDS`], [`STATUSES`], [`kind_label`], [`kind_badge`], [`status_label`],
//!   [`status_badge`], [`status_table`], [`count_words`] and [`meta_line`].
//! - The file [`Picker`] behind *Add images*: pictures, `.txt` and `.log` files, picked in the file dialog or
//!   dropped on the window ([`drop_zone`]), at most [`MAX_FILES`] of at most [`MAX_BYTES`] each.
//! - The New report dialog: [`NewReport`], [`NewMsg`], [`new_update`] and [`new_dialog`] (kind, title,
//!   description, files, and a line saying what is sent along).
//! - The reply box of a report's thread: [`Reply`], [`ReplyMsg`], [`reply_update`] and [`reply_view`].
//!
//! The sheet holds no network code and opens no file dialog. Its update keeps the state, checks it and
//! answers what the app must do ([`NewOut`], [`ReplyOut`]): open the file dialog, show a toast, send this
//! report or reply. The app builds the multipart request (`POST /api/bug-reports` with the text fields of
//! [`Report::fields`] and one [`FILE_FIELD`] part per file; a reply posts [`ReplyPost::fields`] the same
//! way to `/api/bug-reports/{id}/messages`), sends it with its own client and hands the answer back
//! ([`NewMsg::Sent`], [`ReplyMsg::Sent`]). The app version and the device line are the app's to give.

use crate::drop_zone::{self, DropZone, Outcome};
use crate::theme::{self, Badge, Btn};
use crate::widgets::{self, badge, btn, btn_icon, card, field, modal, seg, small, small_muted, text_area, El, Icon};
use iced::widget::{button, column, container, image, row, text_editor, text_input};
use iced::{Alignment, Length};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Every kind, as the server stores it and as the user reads it: the list's chips and the labels;
/// `crash` is a crash log the phone app sends (the Crash logs section).
pub const KINDS: [(&str, &str); 4] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…"), ("crash", "Crash log")];
/// The kinds a new report can pick (a crash log comes from the app).
pub const PICK_KINDS: [(&str, &str); 3] = [("bug", "Bug"), ("not_bug", "Not a bug"), ("howto", "How do I…")];
/// The five statuses: key, label, badge.
pub const STATUSES: [(&str, &str, Badge); 5] = [
    ("new", "New", Badge::Warn),
    ("looking", "Looking into it", Badge::Info),
    ("fixed", "Fixed", Badge::Ok),
    ("wontfix", "Won't fix", Badge::Plain),
    ("answered", "Answered", Badge::Accent),
];
/// The most files a report or a reply takes.
pub const MAX_FILES: usize = 6;
/// The biggest file a report or a reply takes (8 MiB).
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The files *Add images* takes, picked or dropped (images, .txt and .log files, several at once).
pub const ATTACH_FILES: drop_zone::Accept = drop_zone::Accept::many(&["png", "jpg", "jpeg", "webp", "gif", "txt", "log"]);
/// The name of the file dialog's filter for [`ATTACH_FILES`] (Encore: `rfd`'s `add_filter`).
pub const FILE_FILTER: &str = "Images and text files";
/// The multipart field name of each attached file.
pub const FILE_FIELD: &str = "file";

/// The word for a kind key ("Bug" for one it doesn't know).
pub fn kind_label(k: &str) -> &'static str {
    KINDS.iter().find(|(key, _)| *key == k).map(|(_, l)| *l).unwrap_or("Bug")
}

/// A kind as a badge: a bug red, a how-to blue, a crash log amber, the rest plain.
pub fn kind_badge<'a, M: 'a>(k: &str) -> El<'a, M> {
    let kind = match k {
        "bug" => Badge::Err,
        "howto" => Badge::Info,
        "crash" => Badge::Warn,
        _ => Badge::Plain,
    };
    badge(kind_label(k), kind)
}

/// The word for a status key ("New" for one it doesn't know).
pub fn status_label(s: &str) -> &'static str {
    STATUSES.iter().find(|(key, _, _)| *key == s).map(|(_, l, _)| *l).unwrap_or("New")
}

/// A status as a badge in its colour.
pub fn status_badge<'a, M: 'a>(s: &str) -> El<'a, M> {
    let kind = STATUSES.iter().find(|(key, _, _)| *key == s).map(|(_, _, b)| *b).unwrap_or(Badge::Warn);
    badge(status_label(s), kind)
}

/// `(key, label)` pairs for a status select.
pub fn status_table() -> Vec<(&'static str, &'static str)> {
    STATUSES.iter().map(|(k, l, _)| (*k, *l)).collect()
}

/// What stands for the browser line on the web: the app's user agent and the window size
/// (`"<agent> · 1280x800"`), for [`new_update`]'s `device`.
pub fn device_line(agent: &str, width: f32, height: f32) -> String {
    format!("{} · {}x{}", agent, width.round() as u32, height.round() as u32)
}

/// "No replies", "1 reply", "3 replies" + " · 2 files".
pub fn count_words(messages: i64, files: i64) -> String {
    let mut s = match messages {
        0 => "No replies".to_string(),
        1 => "1 reply".to_string(),
        n => format!("{n} replies"),
    };
    if files > 0 {
        s += &format!(" · {files} file{}", if files == 1 { "" } else { "s" });
    }
    s
}

/// "by user · date · page X · app Y · device" for a report (`{username, created_at, page, app_version,
/// device}`); the parts it doesn't have are left out.
pub fn meta_line(r: &Value) -> String {
    let mut s = format!("by {} · {}", json_s(r, "username"), date(r.get("created_at").and_then(Value::as_f64).unwrap_or(0.0)));
    let page = json_s(r, "page");
    if !page.is_empty() {
        s += &format!(" · page {page}");
    }
    if truthy(r, "app_version") {
        s += &format!(" · app {}", json_s(r, "app_version"));
    }
    if truthy(r, "device") {
        s += &format!(" · {}", json_s(r, "device"));
    }
    s
}

/// A JSON field as text: a string, a number or a bool; "" otherwise (Encore's `fmt::s`).
fn json_s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// JavaScript truthiness of a field (Encore's `fmt::truthy`).
fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(_)) => true,
    }
}

/// `18 Oct 2026` for a unix timestamp in seconds (Encore's `fmt::date`, in UTC).
fn date(ts: f64) -> String {
    let days = (ts / 86_400.0).floor() as i64;
    let (y, m, d) = civil_from_days(days);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{d} {} {y}", MONTHS[(m - 1) as usize])
}

/// The civil date (year, month, day) of a day count since 1970-01-01.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// ---- the file picker behind "Add images" (the New report dialog and the reply box) ---------

/// A picked file: its path and size.
pub type Picked = (PathBuf, u64);

/// The files picked (or dropped) for a report or a reply, and the window's drop place.
#[derive(Debug, Default)]
pub struct Picker {
    /// The picked files, in the order they came, each once.
    pub files: Vec<PathBuf>,
    /// Files dragged onto the window: they join the picked ones.
    pub drop: DropZone,
}

impl Picker {
    /// Add picked files within the limits; the words of each rule that stopped one come back, for an
    /// error toast each.
    pub fn add(&mut self, picked: Vec<Picked>) -> Vec<String> {
        let mut errors = Vec::new();
        for (path, size) in picked {
            if self.files.len() >= MAX_FILES {
                errors.push(format!("At most {MAX_FILES} files per report"));
                break;
            }
            if size > MAX_BYTES {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                errors.push(format!("{name} is over 8 MB"));
                continue;
            }
            if !self.files.contains(&path) {
                self.files.push(path);
            }
        }
        errors
    }

    /// A file dragged over or dropped on the window: a dropped one is added like a picked one
    /// (same types, same limits); the words for one that isn't taken come back, for an error toast.
    pub fn dropped(&mut self, ev: drop_zone::Event) -> Vec<String> {
        match self.drop.handle(ev, &ATTACH_FILES) {
            Outcome::Take(path) => {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                self.add(vec![(path, size)])
            }
            Outcome::Refused(m) => vec![m],
            Outcome::Nothing => Vec::new(),
        }
    }

    /// Take the file at `i` off the list (nothing when there is none).
    pub fn remove(&mut self, i: usize) {
        if i < self.files.len() {
            self.files.remove(i);
        }
    }

    /// The thumbnails (pictures) or names (text files), each with its × button.
    pub fn view<'a, M: Clone + 'a>(&'a self, on_remove: impl Fn(usize) -> M + 'a) -> El<'a, M> {
        let p = theme::look();
        let mut r = row![].spacing(8);
        for (i, f) in self.files.iter().enumerate() {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let inner: El<'a, M> = if is_picture(f) {
                image(image::Handle::from_path(f)).width(56).height(56).content_fit(iced::ContentFit::Cover).border_radius(6.0).into()
            } else {
                container(small(name.clone()).color(p.muted))
                    .padding([6, 10])
                    .height(56)
                    .max_width(160)
                    .center_y(56)
                    .style(theme::panel2)
                    .into()
            };
            r = r.push(
                row![inner, button(widgets::icon(Icon::X, 12.0, p.muted)).padding(4).style(theme::btn(Btn::Icon)).on_press(on_remove(i)),]
                    .spacing(2)
                    .align_y(Alignment::Start),
            );
        }
        r.wrap().into()
    }
}

/// Whether the file is a picture by its extension.
pub fn is_picture(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif")
    )
}

/// The files the file dialog answered with, with their sizes (0 for one that can't be read), for
/// [`NewMsg::Picked`] and [`ReplyMsg::Picked`].
pub fn with_sizes(paths: impl IntoIterator<Item = PathBuf>) -> Vec<Picked> {
    paths
        .into_iter()
        .map(|path| {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            (path, size)
        })
        .collect()
}

// ---- the New report dialog -------------------------------------------------------------

/// The New report dialog's state.
#[derive(Debug)]
pub struct NewReport {
    /// The kind key, one of [`PICK_KINDS`] (`bug` at first).
    pub kind: String,
    /// The page the person came from (`#/library/album/…`), sent along; "" for none.
    pub page: String,
    /// The title as typed.
    pub title: String,
    /// The description.
    pub body: text_editor::Content,
    /// The attached files.
    pub picker: Picker,
    /// The report is on its way: Send is off and reads "Sending…".
    pub sending: bool,
}

impl Default for NewReport {
    fn default() -> Self {
        Self {
            kind: "bug".into(),
            page: String::new(),
            title: String::new(),
            body: text_editor::Content::new(),
            picker: Picker::default(),
            sending: false,
        }
    }
}

/// The New report dialog's messages.
#[derive(Clone, Debug)]
pub enum NewMsg {
    /// A kind was picked.
    Kind(String),
    /// The title changed.
    Title(String),
    /// An edit in the description.
    Body(text_editor::Action),
    /// *Add images*: the app opens the file dialog ([`NewOut::PickFiles`]).
    AddFiles,
    /// The files the file dialog answered with.
    Picked(Vec<Picked>),
    /// A file dragged over or dropped on the window: it joins the attachments.
    Drop(drop_zone::Event),
    /// The × of the attached file at this place.
    RemoveFile(usize),
    /// Send.
    Send,
    /// The server's answer to [`NewOut::Send`]: the new report's id, or the words of the error.
    Sent(Result<i64, String>),
}

/// A new report to send: `POST /api/bug-reports`, multipart, with [`Report::fields`] as text fields
/// and one [`FILE_FIELD`] part per file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The kind key.
    pub kind: String,
    /// The title, trimmed.
    pub title: String,
    /// The description, trimmed.
    pub body: String,
    /// The page the person came from; "" for none.
    pub page: String,
    /// The app version the app gave; "" when it has none.
    pub app_version: String,
    /// The device line the app gave ([`device_line`]).
    pub device: String,
    /// The attached files, in order.
    pub files: Vec<PathBuf>,
}

impl Report {
    /// The text fields of the form, by name, in Encore's order: `kind`, `title`, `body`, `page`,
    /// `app_version`, `device`.
    pub fn fields(&self) -> [(&'static str, &str); 6] {
        [
            ("kind", &self.kind),
            ("title", &self.title),
            ("body", &self.body),
            ("page", &self.page),
            ("app_version", &self.app_version),
            ("device", &self.device),
        ]
    }
}

/// What the app does after a New report message, in this order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NewOut {
    /// Open the file dialog (several files, filter [`FILE_FILTER`] with [`ATTACH_FILES`]' endings) and
    /// answer with [`NewMsg::Picked`] (see [`with_sizes`]); nothing picked answers nothing or an empty list.
    PickFiles,
    /// Show an error toast with these words.
    Error(String),
    /// Show a toast with these words.
    Done(String),
    /// Send this report, then answer with [`NewMsg::Sent`].
    Send(Report),
    /// The report was sent: close the dialog, refresh what counts reports (the sidebar badge) and open
    /// the report with this id.
    Show(i64),
}

/// The title and the description, trimmed, when a report can be sent; else the words saying what is
/// missing.
pub fn validate(d: &NewReport) -> Result<(String, String), &'static str> {
    let title = d.title.trim().to_string();
    let text = d.body.text().trim().to_string();
    if title.is_empty() {
        return Err("Give the report a title");
    }
    if text.is_empty() {
        return Err("Say what happened");
    }
    Ok((title, text))
}

/// Drive the dialog. `app_version` (the server's version as the app knows it, "" for none) and
/// `device` ([`device_line`]) go into the report it asks to send.
pub fn new_update(d: &mut NewReport, msg: NewMsg, app_version: &str, device: &str) -> Vec<NewOut> {
    match msg {
        NewMsg::Kind(k) => d.kind = k,
        NewMsg::Title(s) => d.title = s,
        NewMsg::Body(a) => d.body.perform(a),
        NewMsg::AddFiles => return vec![NewOut::PickFiles],
        NewMsg::Picked(files) => return d.picker.add(files).into_iter().map(NewOut::Error).collect(),
        NewMsg::Drop(ev) => return d.picker.dropped(ev).into_iter().map(NewOut::Error).collect(),
        NewMsg::RemoveFile(i) => d.picker.remove(i),
        NewMsg::Send => {
            if d.sending {
                return Vec::new();
            }
            let (title, body) = match validate(d) {
                Ok(v) => v,
                Err(e) => return vec![NewOut::Error(e.into())],
            };
            d.sending = true;
            return vec![NewOut::Send(Report {
                kind: d.kind.clone(),
                title,
                body,
                page: d.page.clone(),
                app_version: app_version.to_string(),
                device: device.to_string(),
                files: d.picker.files.clone(),
            })];
        }
        NewMsg::Sent(Ok(id)) => {
            d.sending = false;
            return vec![NewOut::Done("Report sent. Thanks.".into()), NewOut::Show(id)];
        }
        NewMsg::Sent(Err(e)) => {
            d.sending = false;
            return vec![NewOut::Error(e)];
        }
    }
    Vec::new()
}

/// The whole modal in the page's message type. `app_version` is the one [`new_update`] gets.
pub fn new_dialog<'a, M: Clone + 'a>(d: &'a NewReport, app_version: &str, wrap: impl Fn(NewMsg) -> M + 'a, on_close: M) -> El<'a, M> {
    let send = wrap(NewMsg::Send);
    let body = column![
        seg(&PICK_KINDS, &d.kind, NewMsg::Kind),
        field("Title", text_input("A short name for it", &d.title).on_input(NewMsg::Title).padding([9, 12]).size(14).style(theme::input)),
        field("Description", text_area(&d.body, "What happened, what you expected, how to repeat it", 6, NewMsg::Body)),
        drop_zone::view(
            column![
                row![
                    btn_icon(Icon::Plus, "Add images", Btn::Default, Some(NewMsg::AddFiles)),
                    small_muted(format!("Up to {MAX_FILES} files, 8 MB each. Images, .txt and .log files (pick or drop them).")),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .wrap(),
                d.picker.view(NewMsg::RemoveFile),
            ]
            .spacing(12),
            &d.picker.drop,
            "Drop images, .txt or .log files to attach them",
        ),
        small_muted(format!(
            "Sent along: {}the app version ({}) and this computer.",
            if d.page.is_empty() { String::new() } else { format!("the page you came from ({}), ", d.page) },
            if app_version.is_empty() { "unknown" } else { app_version }
        )),
    ]
    .spacing(12);
    let footer = row![
        btn("Cancel", Btn::Default, Some(on_close.clone())),
        btn(if d.sending { "Sending…" } else { "Send" }, Btn::Primary, (!d.sending).then_some(send)),
    ]
    .spacing(10);
    modal("New report", El::from(body).map(wrap), Some(footer.into()), false, on_close)
}

// ---- the reply box of a report's thread ---------------------------------------------------

/// The reply box's state: the text, its files and whether it is on its way.
#[derive(Debug)]
pub struct Reply {
    /// The reply as typed.
    pub text: text_editor::Content,
    /// The attached files.
    pub picker: Picker,
    /// The reply is on its way: Send is off and reads "Sending…".
    pub sending: bool,
}

impl Default for Reply {
    fn default() -> Self {
        Self { text: text_editor::Content::new(), picker: Picker::default(), sending: false }
    }
}

/// The reply box's messages.
#[derive(Clone, Debug)]
pub enum ReplyMsg {
    /// An edit in the reply.
    Text(text_editor::Action),
    /// *Add images*: the app opens the file dialog ([`ReplyOut::PickFiles`]).
    AddFiles,
    /// The files the file dialog answered with.
    Picked(Vec<Picked>),
    /// A file dragged over or dropped on the window: it joins the reply's attachments.
    Drop(drop_zone::Event),
    /// The × of the attached file at this place.
    RemoveFile(usize),
    /// Send.
    Send,
    /// The server's answer to [`ReplyOut::Send`]: sent, or the words of the error.
    Sent(Result<(), String>),
}

/// A reply to send: `POST /api/bug-reports/{id}/messages`, multipart, with [`ReplyPost::fields`] as text
/// fields and one [`FILE_FIELD`] part per file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplyPost {
    /// The reply, trimmed (may be "" when files go).
    pub body: String,
    /// The attached files, in order.
    pub files: Vec<PathBuf>,
}

impl ReplyPost {
    /// The text fields of the form: `body`.
    pub fn fields(&self) -> [(&'static str, &str); 1] {
        [("body", &self.body)]
    }
}

/// What the app does after a reply box message, in this order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyOut {
    /// Open the file dialog, as [`NewOut::PickFiles`], and answer with [`ReplyMsg::Picked`].
    PickFiles,
    /// Show an error toast with these words.
    Error(String),
    /// Show a toast with these words.
    Done(String),
    /// Send this reply, then answer with [`ReplyMsg::Sent`].
    Send(ReplyPost),
    /// The reply was sent (the box is empty again): refresh what counts reports and load the report again.
    Reload,
}

/// Drive the reply box.
pub fn reply_update(r: &mut Reply, msg: ReplyMsg) -> Vec<ReplyOut> {
    match msg {
        ReplyMsg::Text(a) => r.text.perform(a),
        ReplyMsg::AddFiles => return vec![ReplyOut::PickFiles],
        ReplyMsg::Picked(files) => return r.picker.add(files).into_iter().map(ReplyOut::Error).collect(),
        ReplyMsg::Drop(ev) => return r.picker.dropped(ev).into_iter().map(ReplyOut::Error).collect(),
        ReplyMsg::RemoveFile(i) => r.picker.remove(i),
        ReplyMsg::Send => {
            if r.sending {
                return Vec::new();
            }
            let body = r.text.text().trim().to_string();
            if body.is_empty() && r.picker.files.is_empty() {
                return vec![ReplyOut::Error("Write a reply or add a file".into())];
            }
            r.sending = true;
            return vec![ReplyOut::Send(ReplyPost { body, files: r.picker.files.clone() })];
        }
        ReplyMsg::Sent(Ok(())) => {
            *r = Reply::default();
            return vec![ReplyOut::Done("Reply sent".into()), ReplyOut::Reload];
        }
        ReplyMsg::Sent(Err(e)) => {
            r.sending = false;
            return vec![ReplyOut::Error(e)];
        }
    }
    Vec::new()
}

/// The reply box: a card with the text, *Add images*, the attached files and Send, and the drop place.
pub fn reply_view(r: &Reply) -> El<'_, ReplyMsg> {
    card(drop_zone::view(
        column![
            text_area(&r.text, "Write a reply", 3, ReplyMsg::Text),
            row![
                btn_icon(Icon::Plus, "Add images", Btn::Default, Some(ReplyMsg::AddFiles)),
                container(r.picker.view(ReplyMsg::RemoveFile)).width(Length::Fill),
                btn(if r.sending { "Sending…" } else { "Send" }, Btn::Primary, (!r.sending).then_some(ReplyMsg::Send)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
        &r.picker.drop,
        "Drop images, .txt or .log files to attach them to your reply",
    ))
    .width(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn words_match_the_web() {
        assert_eq!(kind_label("howto"), "How do I…");
        assert_eq!(kind_label("nope"), "Bug");
        assert_eq!(status_label("wontfix"), "Won't fix");
        assert_eq!(status_label("?"), "New");
        assert_eq!(count_words(0, 0), "No replies");
        assert_eq!(count_words(1, 1), "1 reply · 1 file");
        assert_eq!(count_words(3, 2), "3 replies · 2 files");
        assert_eq!(status_table().len(), 5);
    }

    #[test]
    fn pictures_by_extension() {
        assert!(is_picture(Path::new("/a/b.PNG")));
        assert!(!is_picture(Path::new("/a/b.log")));
    }

    #[test]
    fn the_meta_line_lists_what_is_known() {
        let r = json!({ "username": "ann", "created_at": 1_792_311_210.0, "page": "#/library", "app_version": "0.27.0", "device": "Encore desktop" });
        assert_eq!(meta_line(&r), "by ann · 18 Oct 2026 · page #/library · app 0.27.0 · Encore desktop");
        let r = json!({ "username": "ann", "created_at": 1_792_311_210.0, "page": "", "app_version": "" });
        assert_eq!(meta_line(&r), "by ann · 18 Oct 2026");
        assert_eq!(status_label("fixed"), "Fixed");
    }

    #[test]
    fn the_limits_match_the_contract() {
        // contracts/bug-reports.md: 8 MiB per file, 6 files per report or message
        assert_eq!(MAX_BYTES, 8 * 1024 * 1024);
        assert_eq!(MAX_FILES, 6);
        assert_eq!(KINDS.map(|(k, _)| k), ["bug", "not_bug", "howto", "crash"]);
        assert_eq!(STATUSES.map(|(k, _, _)| k), ["new", "looking", "fixed", "wontfix", "answered"]);
        assert!(!PICK_KINDS.iter().any(|(k, _)| *k == "crash"), "a crash log comes from the app");
    }

    #[test]
    fn the_device_line_reads_like_encore() {
        assert_eq!(device_line("Encore-Desktop/0.27 (Linux)", 1279.6, 800.2), "Encore-Desktop/0.27 (Linux) · 1280x800");
    }

    fn picked(names: &[&str], size: u64) -> Vec<Picked> {
        names.iter().map(|n| (PathBuf::from(format!("/a/{n}")), size)).collect()
    }

    #[test]
    fn the_picker_stops_at_six_files() {
        let mut p = Picker::default();
        let errors = p.add(picked(&["1.png", "2.png", "3.png", "4.png", "5.png", "6.png", "7.png", "8.png"], 10));
        assert_eq!(p.files.len(), MAX_FILES);
        assert_eq!(errors, ["At most 6 files per report"], "one toast, then it stops");
        assert_eq!(p.add(picked(&["9.png"], 10)), ["At most 6 files per report"]);
    }

    #[test]
    fn the_picker_turns_away_big_files_and_keeps_going() {
        let mut p = Picker::default();
        let errors = p.add(vec![("/a/big.png".into(), MAX_BYTES + 1), ("/a/edge.png".into(), MAX_BYTES), ("/a/log.txt".into(), 1)]);
        assert_eq!(errors, ["big.png is over 8 MB"]);
        assert_eq!(p.files, [PathBuf::from("/a/edge.png"), PathBuf::from("/a/log.txt")]);
    }

    #[test]
    fn the_picker_takes_a_file_once_and_removes_by_place() {
        let mut p = Picker::default();
        assert!(p.add(picked(&["a.png", "a.png", "b.log"], 1)).is_empty());
        assert_eq!(p.files.len(), 2);
        p.remove(9);
        assert_eq!(p.files.len(), 2);
        p.remove(0);
        assert_eq!(p.files, [PathBuf::from("/a/b.log")]);
    }

    #[test]
    fn a_dropped_file_is_checked_like_a_picked_one() {
        let mut p = Picker::default();
        assert!(p.dropped(drop_zone::Event::Hovered).is_empty());
        assert!(p.drop.hovering);
        assert_eq!(
            p.dropped(drop_zone::Event::Dropped("/a/setup.exe".into())),
            ["setup.exe can't be uploaded here (it takes .png, .jpg, .jpeg, .webp, .gif, .txt, .log)"]
        );
        let dir = std::env::temp_dir().join(format!("kit-iced-bug-report-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("encore.log");
        std::fs::write(&log, "line").unwrap();
        assert!(p.dropped(drop_zone::Event::Dropped(log.clone())).is_empty());
        assert_eq!(p.files, std::slice::from_ref(&log));
        assert_eq!(with_sizes([log.clone(), dir.join("gone.txt")]), [(log, 4), (dir.join("gone.txt"), 0)]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn typed(d: &mut NewReport, title: &str, body: &str) {
        new_update(d, NewMsg::Title(title.into()), "", "");
        new_update(d, NewMsg::Body(text_editor::Action::SelectAll), "", "");
        for c in body.chars() {
            new_update(d, NewMsg::Body(text_editor::Action::Edit(text_editor::Edit::Insert(c))), "", "");
        }
    }

    #[test]
    fn a_report_needs_a_title_and_a_description() {
        let mut d = NewReport::default();
        assert_eq!(new_update(&mut d, NewMsg::Send, "", ""), [NewOut::Error("Give the report a title".into())]);
        typed(&mut d, "   ", "it broke");
        assert_eq!(new_update(&mut d, NewMsg::Send, "", ""), [NewOut::Error("Give the report a title".into())]);
        let mut d = NewReport::default();
        typed(&mut d, "Crash", "  ");
        assert_eq!(validate(&d), Err("Say what happened"));
        assert_eq!(new_update(&mut d, NewMsg::Send, "", ""), [NewOut::Error("Say what happened".into())]);
        assert!(!d.sending);
    }

    #[test]
    fn send_asks_the_app_to_send_the_report_once() {
        let mut d = NewReport { page: "#/library".into(), ..NewReport::default() };
        new_update(&mut d, NewMsg::Kind("howto".into()), "", "");
        typed(&mut d, "  Crash on start ", " It closes. ");
        new_update(&mut d, NewMsg::Picked(picked(&["shot.png"], 10)), "", "");
        let out = new_update(&mut d, NewMsg::Send, "0.27.0", "Encore · 1280x800");
        let report = Report {
            kind: "howto".into(),
            title: "Crash on start".into(),
            body: "It closes.".into(),
            page: "#/library".into(),
            app_version: "0.27.0".into(),
            device: "Encore · 1280x800".into(),
            files: vec!["/a/shot.png".into()],
        };
        assert_eq!(out, [NewOut::Send(report.clone())]);
        assert_eq!(
            report.fields(),
            [
                ("kind", "howto"),
                ("title", "Crash on start"),
                ("body", "It closes."),
                ("page", "#/library"),
                ("app_version", "0.27.0"),
                ("device", "Encore · 1280x800")
            ]
        );
        assert!(d.sending);
        assert!(new_update(&mut d, NewMsg::Send, "0.27.0", "").is_empty(), "no second send while one is on its way");
        assert_eq!(new_update(&mut d, NewMsg::Sent(Err("Too many reports".into())), "", ""), [NewOut::Error("Too many reports".into())]);
        assert!(!d.sending);
        new_update(&mut d, NewMsg::Send, "0.27.0", "");
        assert_eq!(new_update(&mut d, NewMsg::Sent(Ok(41)), "", ""), [NewOut::Done("Report sent. Thanks.".into()), NewOut::Show(41)]);
        assert!(!d.sending);
    }

    #[test]
    fn the_dialog_asks_for_the_file_dialog_and_passes_on_the_limits() {
        let mut d = NewReport::default();
        assert_eq!(new_update(&mut d, NewMsg::AddFiles, "", ""), [NewOut::PickFiles]);
        assert_eq!(
            new_update(&mut d, NewMsg::Picked(vec![("/a/big.png".into(), MAX_BYTES + 1)]), "", ""),
            [NewOut::Error("big.png is over 8 MB".into())]
        );
        assert_eq!(
            new_update(&mut d, NewMsg::Drop(drop_zone::Event::Dropped("/a/x.zip".into())), "", ""),
            [NewOut::Error("x.zip can't be uploaded here (it takes .png, .jpg, .jpeg, .webp, .gif, .txt, .log)".into())]
        );
        new_update(&mut d, NewMsg::Picked(picked(&["a.png", "b.png"], 1)), "", "");
        new_update(&mut d, NewMsg::RemoveFile(0), "", "");
        assert_eq!(d.picker.files, [PathBuf::from("/a/b.png")]);
    }

    #[test]
    fn a_reply_needs_words_or_a_file() {
        let mut r = Reply::default();
        assert_eq!(reply_update(&mut r, ReplyMsg::Send), [ReplyOut::Error("Write a reply or add a file".into())]);
        assert_eq!(reply_update(&mut r, ReplyMsg::AddFiles), [ReplyOut::PickFiles]);
        reply_update(&mut r, ReplyMsg::Picked(picked(&["shot.png"], 1)));
        assert_eq!(
            reply_update(&mut r, ReplyMsg::Send),
            [ReplyOut::Send(ReplyPost { body: String::new(), files: vec!["/a/shot.png".into()] })]
        );
        assert!(r.sending);
        assert!(reply_update(&mut r, ReplyMsg::Send).is_empty());
        assert_eq!(reply_update(&mut r, ReplyMsg::Sent(Err("No".into()))), [ReplyOut::Error("No".into())]);
        assert!(!r.sending);
        for c in " thanks ".chars() {
            reply_update(&mut r, ReplyMsg::Text(text_editor::Action::Edit(text_editor::Edit::Insert(c))));
        }
        let out = reply_update(&mut r, ReplyMsg::Send);
        assert_eq!(out, [ReplyOut::Send(ReplyPost { body: "thanks".into(), files: vec!["/a/shot.png".into()] })]);
        assert_eq!(ReplyPost { body: "thanks".into(), files: vec![] }.fields(), [("body", "thanks")]);
        assert_eq!(reply_update(&mut r, ReplyMsg::Sent(Ok(()))), [ReplyOut::Done("Reply sent".into()), ReplyOut::Reload]);
        assert!(r.picker.files.is_empty() && r.text.text().trim().is_empty() && !r.sending, "the box is empty again");
    }

    #[derive(Clone, Debug)]
    enum Page {
        New(NewMsg),
        Close,
    }

    #[test]
    fn the_dialog_has_its_title_field_and_send() {
        let d = NewReport { title: "Crash on start".into(), page: "#/library".into(), ..NewReport::default() };
        let mut ui = iced_test::simulator(new_dialog(&d, "0.27.0", Page::New, Page::Close));
        assert!(ui.find("Crash on start").is_ok(), "the title field holds the title");
        assert!(ui.find("New report").is_ok());
        assert!(ui.find("Sent along: the page you came from (#/library), the app version (0.27.0) and this computer.").is_ok());
        ui.click("Send").expect("Send is there");
        assert!(ui.into_messages().any(|m| matches!(m, Page::New(NewMsg::Send))));

        let busy = NewReport { sending: true, ..NewReport::default() };
        let mut ui = iced_test::simulator(new_dialog(&busy, "", Page::New, Page::Close));
        assert!(ui.find("Sending…").is_ok());
        assert!(ui.find("Sent along: the app version (unknown) and this computer.").is_ok());
        ui.click("Sending…").expect("the button is there");
        assert!(!ui.into_messages().any(|m| matches!(m, Page::New(NewMsg::Send))), "no Send while sending");
    }
}
