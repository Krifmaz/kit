//! Notice banners: a plain sentence saying what is wrong and what to check, with Check again.
//!
//! A server keeps a list of notices (`contracts/notices.md`): each an object `{ok, text, checking, id,
//! severity}`. While one fails, admins see its sentence in a banner with Check again; the check's answer says
//! how it went in a toast made of three sentences per notice ([`CheckWords`], the app's for each notice, the
//! kit's [`CheckWords::DEFAULT`] otherwise). The notices are asked for every minute, every 15 s while one
//! fails ([`NoticePoll`]).

use crate::toasts::ToastKind;
use serde::{Deserialize, Serialize};

/// The banner's button.
pub const CHECK_AGAIN: &str = "Check again";
/// The button while the check runs.
pub const CHECKING: &str = "Checking…";
/// How often the notices are looked at; a failing notice is asked for at every look.
pub const TICK_MS: u64 = 15_000;
/// While every notice is fine, it is asked for every this many looks (every minute).
pub const QUIET_TICKS: u32 = 4;

/// How serious a notice is (`contracts/notices.md`; `warn` when the server says nothing).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// For information.
    Info,
    /// Something to check.
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
    /// A check still runs (a disk or a service that does not answer yet).
    pub checking: bool,
    /// The notice's id (`storage`, `mail`): which notice it is, for its own Check again words.
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
    /// The kit's words for a notice the app has none of its own for.
    pub const DEFAULT: CheckWords =
        CheckWords { failing: "The problem is still there", checking: "Still checking", fixed: "Fixed – everything is fine again" };
}

impl Default for CheckWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The notice with id `id` among `notices`, if there is one.
pub fn find<'a>(notices: &'a [Notice], id: &str) -> Option<&'a Notice> {
    notices.iter().find(|n| n.id == id)
}

/// The notices of an answer: the `notices` list (`GET /api/notices`), or the answer itself when it is one
/// notice object. Unreadable entries count as fine.
pub fn parse(answer: &serde_json::Value) -> Vec<Notice> {
    match answer.get("notices").and_then(serde_json::Value::as_array) {
        Some(list) => list
            .iter()
            .map(|v| if v.is_object() { serde_json::from_value(v.clone()).unwrap_or_default() } else { Notice::default() })
            .collect(),
        None if answer.is_object() && answer.get("ok").is_some() => vec![serde_json::from_value(answer.clone()).unwrap_or_default()],
        None => vec![],
    }
}

/// The toast after Check again and its kind: still failing, a check that still runs, or fixed.
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
    use serde_json::json;

    const TEXT: &str = "The data folder /data is read-only – check the disk (dmesg, fsck) or the mount";

    #[test]
    fn the_banner_shows_the_servers_sentence_while_a_notice_fails() {
        let d: Notice = serde_json::from_str(&format!(r#"{{ "ok": false, "text": "{TEXT}", "waiting": 3 }}"#)).unwrap();
        assert_eq!(d.banner(), Some(TEXT));
        // fine, an older server, a member's answer: nothing
        let ok: Notice = serde_json::from_str(r#"{ "ok": true, "text": null }"#).unwrap();
        assert_eq!(ok.banner(), None);
        assert_eq!(Notice::default().banner(), None);
        let silent: Notice = serde_json::from_str(r#"{ "ok": false }"#).unwrap();
        assert_eq!(silent.banner(), None);
    }

    #[test]
    fn check_again_says_how_it_went_in_the_apps_words() {
        let failing: Notice = serde_json::from_str(r#"{ "ok": false, "text": "x" }"#).unwrap();
        let fixed: Notice = serde_json::from_str(r#"{ "ok": true }"#).unwrap();
        let busy: Notice = serde_json::from_str(r#"{ "ok": true, "checking": true }"#).unwrap();
        let w = CheckWords::DEFAULT;
        assert_eq!(check_toast(&failing, &w), ("The problem is still there", ToastKind::Warn));
        assert_eq!(check_toast(&busy, &w), ("Still checking", ToastKind::Warn));
        assert_eq!(check_toast(&fixed, &w), ("Fixed – everything is fine again", ToastKind::Ok));
        let data = CheckWords {
            failing: "The data folder still can't be written",
            checking: "Still checking the data folder",
            fixed: "The data folder can be written",
        };
        assert_eq!(check_toast(&failing, &data), ("The data folder still can't be written", ToastKind::Warn));
        assert_eq!(CheckWords::default(), CheckWords::DEFAULT);
    }

    #[test]
    fn only_admins_see_the_banners() {
        let n: Vec<Notice> =
            serde_json::from_str(r#"[{"ok":false,"text":"x","id":"storage","severity":"warn"},{"ok":true,"id":"mail"}]"#).unwrap();
        assert_eq!(banners(&n, true).len(), 1);
        assert_eq!(banners(&n, true)[0].id, "storage");
        assert!(banners(&n, false).is_empty());
        assert_eq!(n[0].severity, Severity::Warn);
        assert_eq!(find(&n, "mail").map(|m| m.ok), Some(true));
        assert!(find(&n, "nope").is_none());
    }

    #[test]
    fn an_answer_is_a_list_or_one_notice() {
        let list = parse(&json!({ "notices": [{ "id": "storage", "ok": false, "text": TEXT }, { "id": "mail", "ok": true }, 3] }));
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].banner(), Some(TEXT));
        assert!(list[2].ok, "an unreadable entry counts as fine");
        let one = parse(&json!({ "ok": false, "text": TEXT }));
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].banner(), Some(TEXT));
        assert!(parse(&json!({})).is_empty());
        assert!(parse(&json!(null)).is_empty());
    }

    #[test]
    fn asked_every_minute_and_every_15_s_while_failing() {
        let mut p = NoticePoll::default();
        let asks: Vec<bool> = (0..8).map(|_| p.tick(false)).collect();
        assert_eq!(asks, [false, false, false, true, false, false, false, true]);
        assert!(p.tick(true));
    }
}
