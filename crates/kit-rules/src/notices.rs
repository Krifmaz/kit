//! Notice banners: a plain sentence saying what is wrong and what to check, with Check again.
//!
//! From Encore's `web-rs/src/notice.rs` (`banner_html`, `check_toast`, `start`), the storage notice that
//! `contracts/notices.md` generalises: an object `{ok, text, checking, id, severity}`, a banner while it fails,
//! and three sentences per notice for Check again's toast.

use crate::toasts::ToastKind;
use serde::{Deserialize, Serialize};

/// The banner's button. From `web-rs/src/notice.rs` `banner_html`.
pub const CHECK_AGAIN: &str = "Check again";
/// The button while the check runs. From `web-rs/src/notice.rs` `start`.
pub const CHECKING: &str = "Checking…";
/// How often the notices are looked at; a failing notice is asked for at every look. From
/// `web-rs/src/notice.rs` `start` (`interval_forever(15_000, …)`).
pub const TICK_MS: u64 = 15_000;
/// While every notice is fine, it is asked for every this many looks (every minute). From
/// `web-rs/src/notice.rs` `start` (`ticks % 4 == 0`).
pub const QUIET_TICKS: u32 = 4;

/// How serious a notice is (`contracts/notices.md`; a kit addition, Encore's storage notice is `warn`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// For information.
    Info,
    /// Something to check (Encore's storage notice).
    #[default]
    Warn,
    /// Something is broken.
    Err,
}

/// One notice, as the server sends it (`contracts/notices.md`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Notice {
    /// `false`: the banner shows; `true` (or missing): everything is fine.
    pub ok: bool,
    /// The sentence the banner shows, as is (no banner without one).
    pub text: Option<String>,
    /// A check still runs (a folder that does not answer).
    pub checking: bool,
    /// The notice's id (Encore's storage notice is `downloads`).
    pub id: String,
    /// How serious it is.
    pub severity: Severity,
}

impl Default for Notice {
    fn default() -> Self {
        Notice { ok: true, text: None, checking: false, id: String::new(), severity: Severity::Warn }
    }
}

impl Notice {
    /// The sentence the banner shows, or None when no banner shows: it shows while the notice fails and has a
    /// sentence.
    ///
    /// From `web-rs/src/notice.rs` `banner_html`.
    pub fn banner(&self) -> Option<&str> {
        if self.ok {
            return None;
        }
        self.text.as_deref().filter(|t| !t.is_empty())
    }

    /// The notice fails (the banner is asked for every [`TICK_MS`] then).
    pub fn failing(&self) -> bool {
        !self.ok
    }
}

/// The notices whose banners show, in the order the server sent them. Nothing for someone who is not an admin.
///
/// From `web-rs/src/notice.rs` `paint` (`app::is_admin()`).
pub fn banners(notices: &[Notice], admin: bool) -> Vec<&Notice> {
    if !admin {
        return vec![];
    }
    notices.iter().filter(|n| n.banner().is_some()).collect()
}

/// The three sentences Check again's toast says for one notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckWords {
    /// Still failing (warn).
    pub failing: &'static str,
    /// Still checking (warn).
    pub checking: &'static str,
    /// Fixed (ok).
    pub fixed: &'static str,
}

impl CheckWords {
    /// Encore's storage notice (`downloads`). From `web-rs/src/notice.rs` `check_toast`.
    pub const DOWNLOADS: CheckWords = CheckWords {
        failing: "A downloads folder still can't be written",
        checking: "Still checking the downloads folders",
        fixed: "The downloads folders can be written",
    };
}

/// The toast after Check again and its kind: still failing, a check that still runs, or fixed.
///
/// From `web-rs/src/notice.rs` `check_toast`.
pub fn check_toast(n: &Notice, words: &CheckWords) -> (&'static str, ToastKind) {
    if !n.ok {
        (words.failing, ToastKind::Warn)
    } else if n.checking {
        (words.checking, ToastKind::Warn)
    } else {
        (words.fixed, ToastKind::Ok)
    }
}

/// When to ask for the notices again: at start, then at every look ([`TICK_MS`]) while one fails, else every
/// [`QUIET_TICKS`] looks.
///
/// From `web-rs/src/notice.rs` `start`.
#[derive(Clone, Debug, Default)]
pub struct NoticePoll {
    ticks: u32,
}

impl NoticePoll {
    /// The look every [`TICK_MS`]: whether to ask now. `failing`: a notice failed at the last answer.
    pub fn tick(&mut self, failing: bool) -> bool {
        self.ticks = self.ticks.wrapping_add(1);
        failing || self.ticks % QUIET_TICKS == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ported from `web-rs/src/notice.rs` `the_banner_shows_the_servers_sentence_while_a_folder_fails`.
    #[test]
    fn the_banner_shows_the_servers_sentence_while_a_folder_fails() {
        let d: Notice = serde_json::from_str(
            r#"{ "ok": false, "text": "The downloads folder /downloads is read-only – check the disk (dmesg, fsck) or the mount", "waiting": 3 }"#,
        )
        .unwrap();
        assert_eq!(d.banner(), Some("The downloads folder /downloads is read-only – check the disk (dmesg, fsck) or the mount"));
        // every folder writable, an older server, a member's answer: nothing
        let ok: Notice = serde_json::from_str(r#"{ "ok": true, "text": null }"#).unwrap();
        assert_eq!(ok.banner(), None);
        assert_eq!(Notice::default().banner(), None);
        let silent: Notice = serde_json::from_str(r#"{ "ok": false }"#).unwrap();
        assert_eq!(silent.banner(), None);
        let words = CheckWords::DOWNLOADS;
        assert_eq!(check_toast(&d, &words), ("A downloads folder still can't be written", ToastKind::Warn));
        let fixed: Notice = serde_json::from_str(r#"{ "ok": true }"#).unwrap();
        assert_eq!(check_toast(&fixed, &words), ("The downloads folders can be written", ToastKind::Ok));
        let busy: Notice = serde_json::from_str(r#"{ "ok": true, "checking": true }"#).unwrap();
        assert_eq!(check_toast(&busy, &words), ("Still checking the downloads folders", ToastKind::Warn));
    }

    #[test]
    fn only_admins_see_the_banners() {
        let n: Vec<Notice> =
            serde_json::from_str(r#"[{"ok":false,"text":"x","id":"downloads","severity":"warn"},{"ok":true,"id":"y"}]"#).unwrap();
        assert_eq!(banners(&n, true).len(), 1);
        assert_eq!(banners(&n, true)[0].id, "downloads");
        assert!(banners(&n, false).is_empty());
        assert_eq!(n[0].severity, Severity::Warn);
    }

    #[test]
    fn asked_every_minute_and_every_15_s_while_failing() {
        let mut p = NoticePoll::default();
        let asks: Vec<bool> = (0..8).map(|_| p.tick(false)).collect();
        assert_eq!(asks, [false, false, false, true, false, false, false, true]);
        assert!(p.tick(true));
    }
}
