//! Offline mode: when it opens, what it says, how it tries again, and the days of grace.
//!
//! An app that keeps things on the device (files, a cache, pages) can keep working while its server can't be
//! reached: offline mode opens when the server gives no answer, someone was signed in on this device lately
//! (within [`GRACE_DAYS`] of the last contact) and something was saved here to show. In offline mode only the
//! pages the app names work ([`route_path`]); the rest of the app says [`DISABLED_TITLE`]. The banner says so
//! with Retry; changes made meanwhile wait in the [outbox](super::outbox) and go once the server is back
//! ([`back_online_text`]).
//!
//! Everything kept in the browser is under the app's id ([`me_key`], [`signout_pending_key`]). The words are
//! [`OfflineWords`].

use serde_json::{json, Value};

/// How long after the last contact with the server offline mode still opens, in days.
pub const GRACE_DAYS: i64 = 30;
/// One day in seconds.
const DAY: f64 = 86_400.0;
/// How often offline mode tries to reach the server again.
pub const RETRY_MS: u64 = 30_000;
/// How long a check gets to answer (a server whose address drops packets would otherwise keep the page waiting
/// for the connect timeout).
pub const WAIT_MS: u64 = 8_000;
/// While the server answers, the remembered user (the grace) is refreshed at most this often.
pub const REMEMBER_EVERY_MS: u64 = 3_600_000;
/// After "Back online" the page reloads this long later.
pub const BACK_ONLINE_RELOAD_MS: u64 = 1500;
/// A failed call asks for a check at most this often (Android).
pub const NOTICE_GAP_MS: u64 = 10_000;
/// A network change is checked this long after it (Android).
pub const NETWORK_CHANGE_CHECK_MS: u64 = 1500;

/// The web's banner.
pub const BANNER: &str = "Offline – the server can't be reached. What this device saved still works.";
/// The web's banner once the grace has ended.
pub const BANNER_ENDED: &str = "Offline – sign in again when the server is back to keep using what this device saved";
/// The title of an entry switched off in offline mode.
pub const DISABLED_TITLE: &str = "Not available offline";
/// The toast after a Retry the server still didn't answer.
pub const STILL_OFFLINE: &str = "The server still can't be reached";
/// The banner's button.
pub const RETRY: &str = "Retry";
/// The banner's button while it tries.
pub const TRYING: &str = "Trying…";
/// Android's full message (the offline sheet).
pub const ANDROID_BANNER: &str = "Offline – showing what this device saved.";
/// Android's banner, first line.
pub const ANDROID_BANNER_TITLE: &str = "Offline";
/// Android's banner, second line.
pub const ANDROID_BANNER_LINE: &str = "What this device saved still works · tap for details";
/// The web's connection banner (`#conn-banner`) while the browser is offline.
pub const NOT_CONNECTED: &str = "Not connected – check your internet or server";
/// The connection banner while the app's server does not answer, `{app}` filled in ([`server_down`]).
pub const SERVER_DOWN: &str = "The {app} server isn't answering";
/// The toast once the server answers again ([`back_online_text`]).
pub const BACK_ONLINE: &str = "Back online";
/// The toast once the server answers again and the outbox went: `{sent}` is [`counts_text`]'s.
pub const BACK_ONLINE_SENT: &str = "Back online – {sent} sent";

/// Offline mode's words. [`OfflineWords::DEFAULT`] holds the kit's; an app passes its own to change any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OfflineWords {
    /// The banner ([`BANNER`]).
    pub banner: &'static str,
    /// The banner once the grace has ended ([`BANNER_ENDED`]).
    pub banner_ended: &'static str,
    /// The title of an entry switched off in offline mode ([`DISABLED_TITLE`]).
    pub disabled: &'static str,
    /// The toast after a Retry the server still didn't answer ([`STILL_OFFLINE`]).
    pub still_offline: &'static str,
    /// The banner's button ([`RETRY`]).
    pub retry: &'static str,
    /// The banner's button while it tries ([`TRYING`]).
    pub trying: &'static str,
    /// The connection banner while the browser is offline ([`NOT_CONNECTED`]).
    pub not_connected: &'static str,
    /// The connection banner while the server does not answer, with `{app}` ([`SERVER_DOWN`]).
    pub server_down: &'static str,
    /// The toast once the server answers again ([`BACK_ONLINE`]).
    pub back_online: &'static str,
    /// The same once the outbox went, with `{sent}` ([`BACK_ONLINE_SENT`]).
    pub back_online_sent: &'static str,
}

impl OfflineWords {
    /// The kit's words.
    pub const DEFAULT: OfflineWords = OfflineWords {
        banner: BANNER,
        banner_ended: BANNER_ENDED,
        disabled: DISABLED_TITLE,
        still_offline: STILL_OFFLINE,
        retry: RETRY,
        trying: TRYING,
        not_connected: NOT_CONNECTED,
        server_down: SERVER_DOWN,
        back_online: BACK_ONLINE,
        back_online_sent: BACK_ONLINE_SENT,
    };

    /// The banner's text: [`OfflineWords::banner`], or [`OfflineWords::banner_ended`] once the grace has ended.
    pub fn banner(&self, grace_ended: bool) -> &'static str {
        if grace_ended {
            self.banner_ended
        } else {
            self.banner
        }
    }

    /// The connection banner while `app`'s server does not answer.
    pub fn server_down(&self, app: &str) -> String {
        crate::app::fill(self.server_down, &[("app", app)])
    }

    /// The toast once the server answers again, saying what the outbox sent (`counts` as [`counts_text`]
    /// takes them).
    pub fn back_online(&self, counts: &[Count]) -> String {
        let sent = counts_text(counts);
        if sent.is_empty() {
            self.back_online.to_string()
        } else {
            crate::app::fill(self.back_online_sent, &[("sent", &sent)])
        }
    }
}

impl Default for OfflineWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The web's connection banner while `app`'s server does not answer ("The Kit Demo server isn't answering").
pub fn server_down(app: &str) -> String {
    OfflineWords::DEFAULT.server_down(app)
}

/// The web's banner text: [`BANNER`], or [`BANNER_ENDED`] once the grace has ended.
pub fn banner(grace_ended: bool) -> &'static str {
    OfflineWords::DEFAULT.banner(grace_ended)
}

/// localStorage: the remembered user (`<app id>.offline.me`).
pub fn me_key(app_id: &str) -> String {
    crate::app::key(&app_id.to_ascii_lowercase(), "offline.me")
}

/// localStorage: "1" while a sign-out has not reached the server (`<app id>.signout_pending`).
pub fn signout_pending_key(app_id: &str) -> String {
    crate::app::key(&app_id.to_ascii_lowercase(), "signout_pending")
}

/// Whole days of grace left (rounded up): [`GRACE_DAYS`] right after a contact, 0 once it has ended. A clock
/// that went back counts as no time passed. Times in unix seconds, as the stored document keeps them.
pub fn grace_left(cached_at_s: f64, now_s: f64) -> i64 {
    let passed = (now_s - cached_at_s).max(0.0);
    let left = GRACE_DAYS as f64 * DAY - passed;
    if left <= 0.0 {
        0
    } else {
        (left / DAY).ceil() as i64
    }
}

/// What is kept of the signed-in user's answer: all of it but the email address, which offline mode never
/// shows.
pub fn me_to_keep(me: &Value) -> Value {
    let mut u = me.clone();
    if let Some(o) = u.as_object_mut() {
        o.remove("email");
    }
    u
}

/// The stored document: `{user, cached_at}` (unix seconds, whole).
pub fn me_doc(me: &Value, now_s: f64) -> Value {
    json!({ "user": me_to_keep(me), "cached_at": now_s.floor() })
}

/// The user and when it was remembered, from the stored document (None without a user object or a time).
pub fn parse_me(text: &str) -> Option<(Value, f64)> {
    let v: Value = serde_json::from_str(text).ok()?;
    let user = v.get("user")?;
    if !user.is_object() {
        return None;
    }
    let at = v.get("cached_at").and_then(Value::as_f64).filter(|t| t.is_finite() && *t > 0.0)?;
    Some((user.clone(), at))
}

/// Whether the signed-in user's call failing with `status` opens offline mode, and with which user: the one
/// remembered within the grace (`cached`), else the last one remembered (`last`; the grace has ended → true).
/// None: the app behaves as before (the sign-in page). It needs no answer from the server, a remembered user and
/// `saved` things on this device to show.
pub fn decide<U>(status: u16, cached: Option<U>, last: Option<U>, saved: usize) -> Option<(U, bool)> {
    if !super::reason::unreachable(status) || saved == 0 {
        return None;
    }
    match (cached, last) {
        (Some(u), _) => Some((u, false)),
        (None, Some(u)) => Some((u, true)),
        _ => None,
    }
}

/// The sign-out call answered `status` (0: no answer): whether the session may still be valid. Only an answer
/// that it ended, or that there was none (401), settles it.
pub fn session_may_live(status: u16) -> bool {
    !((200..300).contains(&status) || status == 401)
}

/// What boot does about a pending sign-out, from the status of the signed-in user's call (200: signed in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtBoot {
    /// No sign-out pending: as usual.
    AsUsual,
    /// The session still works: end it now (the sign-out call), then open signed out; it stays pending while
    /// [`session_may_live`] says so.
    SignOut,
    /// The server says there is no session: nothing left to end (no longer pending).
    Gone,
    /// The server can't be reached (or failed): open signed out, without offline mode, and try again next time.
    Wait,
}

/// What boot does about a pending sign-out.
pub fn at_boot(pending: bool, me_status: u16) -> AtBoot {
    match (pending, me_status) {
        (false, _) => AtBoot::AsUsual,
        (true, 200..=299) => AtBoot::SignOut,
        (true, 401) => AtBoot::Gone,
        (true, _) => AtBoot::Wait,
    }
}

/// The path to show for `path`: in offline mode a path that is not one of the app's offline pages (`allowed`)
/// becomes `home`, the app's first offline page (the web replaces the address, so Back doesn't land on a page
/// that can't open). Outside offline mode every path stays.
pub fn route_path<'a>(active: bool, path: &'a str, allowed: &[&str], home: &'a str) -> &'a str {
    if !active || path == home || allowed.contains(&path) {
        path
    } else {
        home
    }
}

/// What a retry's answer means (status 200 for an answer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryOutcome {
    /// The server answers: send the outbox, say [`back_online_text`], reload after [`BACK_ONLINE_RELOAD_MS`].
    BackOnline,
    /// The server answers but the session is gone (401, 403): reload into the sign-in page.
    SignIn,
    /// Still offline (any other answer keeps offline mode); a Retry that was pressed says [`STILL_OFFLINE`].
    StillOffline,
}

/// What a retry's answer means.
pub fn after_retry(status: u16) -> RetryOutcome {
    match status {
        200..=299 => RetryOutcome::BackOnline,
        401 | 403 => RetryOutcome::SignIn,
        _ => RetryOutcome::StillOffline,
    }
}

/// A call just failed to reach the server at `now_ms` (Android): whether to check the server now. Never while
/// offline mode is on (its own retries run), at most once per [`NOTICE_GAP_MS`]; `last_ms` is the last time it
/// said yes and is moved on a yes.
pub fn notice_unreachable(active: bool, last_ms: &mut u64, now_ms: u64) -> bool {
    if active || now_ms.saturating_sub(*last_ms) < NOTICE_GAP_MS {
        return false;
    }
    *last_ms = now_ms;
    true
}

/// How many of one kind of thing, with its words for one and for many (`Count { n: 3, one: "change", many:
/// "changes" }`). The outbox counts its waiting actions this way ([`super::outbox::Outbox::counts`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count<'a> {
    /// How many.
    pub n: usize,
    /// The word for one ("change").
    pub one: &'a str,
    /// The word for several ("changes").
    pub many: &'a str,
}

/// "3 changes and 1 upload", "1 change", "2 uploads, 1 note and 4 changes" ("" for none): each count that is
/// not 0, in the order given.
pub fn counts_text(counts: &[Count]) -> String {
    let parts: Vec<String> =
        counts.iter().filter(|c| c.n > 0).map(|c| format!("{} {}", c.n, if c.n == 1 { c.one } else { c.many })).collect();
    match parts.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The toast once the server answers again: "Back online – 3 changes sent".
pub fn back_online_text(counts: &[Count]) -> String {
    OfflineWords::DEFAULT.back_online(counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grace_lasts_thirty_days_from_the_last_contact() {
        let t0 = 1_760_000_000.0;
        assert_eq!(grace_left(t0, t0), 30);
        assert_eq!(grace_left(t0, t0 + 3600.0), 30, "rounded up");
        assert_eq!(grace_left(t0, t0 + DAY), 29);
        assert_eq!(grace_left(t0, t0 + 29.5 * DAY), 1);
        assert_eq!(grace_left(t0, t0 + 30.0 * DAY - 1.0), 1);
        assert_eq!(grace_left(t0, t0 + 30.0 * DAY), 0, "ended");
        assert_eq!(grace_left(t0, t0 + 400.0 * DAY), 0);
        assert_eq!(grace_left(t0, t0 - 5.0 * DAY), 30, "a clock that went back");
    }

    #[test]
    fn a_pending_sign_out_is_settled_only_by_the_server() {
        assert_eq!(at_boot(false, 200), AtBoot::AsUsual);
        assert_eq!(at_boot(false, 0), AtBoot::AsUsual);
        assert_eq!(at_boot(true, 200), AtBoot::SignOut);
        assert_eq!(at_boot(true, 401), AtBoot::Gone);
        assert_eq!(at_boot(true, 0), AtBoot::Wait);
        assert_eq!(at_boot(true, 502), AtBoot::Wait);
        assert_eq!(at_boot(true, 500), AtBoot::Wait);
        assert!(!session_may_live(200));
        assert!(!session_may_live(401), "no session to end");
        assert!(session_may_live(0), "no answer: the cookie is still valid");
        assert!(session_may_live(503));
        assert!(session_may_live(500));
        assert!(session_may_live(403));
    }

    #[test]
    fn the_remembered_user_round_trips_without_the_email() {
        let me = json!({ "id": 3, "username": "ana", "email": "ana@example.org", "is_admin": false, "prefs": { "lang": "" } });
        let doc = me_doc(&me, 1_760_000_000.7);
        assert_eq!(doc["cached_at"], json!(1_760_000_000.0));
        let (user, at) = parse_me(&doc.to_string()).unwrap();
        assert_eq!(at, 1_760_000_000.0);
        assert_eq!(user["username"], "ana");
        assert!(user.get("email").is_none(), "{user}");
        assert_eq!(parse_me("not json"), None);
        assert_eq!(parse_me(r#"{"user":null,"cached_at":5}"#), None);
        assert_eq!(parse_me(r#"{"user":{"id":1}}"#), None, "no time, no grace");
    }

    #[test]
    fn every_key_is_under_the_apps_id() {
        assert_eq!(me_key("kit-demo"), "kit-demo.offline.me");
        assert_eq!(signout_pending_key("Photovault"), "photovault.signout_pending");
    }

    #[test]
    fn offline_mode_needs_no_answer_a_remembered_user_and_something_saved() {
        let ana = || Some(json!({ "id": 1, "username": "ana" }));
        assert_eq!(decide(0, ana(), ana(), 3), Some((json!({ "id": 1, "username": "ana" }), false)));
        assert_eq!(decide(502, ana(), ana(), 3).map(|x| x.1), Some(false), "a proxy saying the server is down");
        assert_eq!(decide(0, None, ana(), 3).map(|x| x.1), Some(true), "the grace has ended: sign in again");
        assert_eq!(decide::<Value>(0, None, None, 3), None, "nobody remembered");
        assert_eq!(decide(0, ana(), ana(), 0), None, "nothing saved on this device");
        assert_eq!(decide(401, ana(), ana(), 3), None, "the server answered: signed out");
        assert_eq!(decide(500, ana(), ana(), 3), None);
    }

    #[test]
    fn the_counts_read_as_words() {
        let c = |n, one, many| Count { n, one, many };
        assert_eq!(counts_text(&[]), "");
        assert_eq!(counts_text(&[c(0, "change", "changes"), c(0, "upload", "uploads")]), "");
        assert_eq!(counts_text(&[c(1, "change", "changes")]), "1 change");
        assert_eq!(counts_text(&[c(3, "change", "changes"), c(1, "upload", "uploads")]), "3 changes and 1 upload");
        assert_eq!(counts_text(&[c(0, "change", "changes"), c(2, "upload", "uploads")]), "2 uploads");
        assert_eq!(
            counts_text(&[c(2, "upload", "uploads"), c(1, "note", "notes"), c(4, "change", "changes")]),
            "2 uploads, 1 note and 4 changes"
        );
        assert_eq!(back_online_text(&[]), "Back online");
        assert_eq!(back_online_text(&[c(3, "change", "changes")]), "Back online – 3 changes sent");
        assert_eq!(back_online_text(&[c(3, "change", "changes"), c(1, "upload", "uploads")]), "Back online – 3 changes and 1 upload sent");
    }

    #[test]
    fn offline_mode_shows_only_the_apps_offline_pages() {
        let allowed = ["/saved", "/settings"];
        assert_eq!(route_path(false, "/projects", &allowed, "/saved"), "/projects");
        assert_eq!(route_path(true, "/projects", &allowed, "/saved"), "/saved");
        assert_eq!(route_path(true, "/saved", &allowed, "/saved"), "/saved");
        assert_eq!(route_path(true, "/settings", &allowed, "/saved"), "/settings");
        assert_eq!(route_path(true, "/x", &[], "/"), "/", "an app with one offline page");
        assert_eq!(banner(false), BANNER);
        assert_eq!(banner(true), BANNER_ENDED);
    }

    #[test]
    fn the_connection_banner_says_what_is_down() {
        assert_eq!(NOT_CONNECTED, "Not connected – check your internet or server");
        assert_eq!(server_down("Kit Demo"), "The Kit Demo server isn't answering");
        let mine = OfflineWords { server_down: "{app} antwortet nicht", banner: "Offline", ..OfflineWords::DEFAULT };
        assert_eq!(mine.server_down("Filebrary"), "Filebrary antwortet nicht");
        assert_eq!(mine.banner(false), "Offline");
        assert_eq!(OfflineWords::default(), OfflineWords::DEFAULT);
    }

    #[test]
    fn a_retry_goes_back_online_signs_in_or_stays() {
        assert_eq!(after_retry(200), RetryOutcome::BackOnline);
        assert_eq!(after_retry(401), RetryOutcome::SignIn);
        assert_eq!(after_retry(403), RetryOutcome::SignIn);
        assert_eq!(after_retry(0), RetryOutcome::StillOffline);
        assert_eq!(after_retry(500), RetryOutcome::StillOffline);
    }

    #[test]
    fn a_failed_call_asks_for_a_check_at_most_every_ten_seconds() {
        let mut last = 0;
        assert!(notice_unreachable(false, &mut last, 20_000));
        assert!(!notice_unreachable(false, &mut last, 29_999));
        assert!(notice_unreachable(false, &mut last, 30_000));
        assert!(!notice_unreachable(true, &mut last, 100_000), "offline mode retries by itself");
    }
}
