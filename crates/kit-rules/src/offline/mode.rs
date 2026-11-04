//! Offline mode: when it opens, what it says, how it tries again, and the 30 days of grace.
//!
//! From Encore's `web-rs/src/offline/mod.rs` (the grace, the remembered user, a sign-out the server has not
//! confirmed) and `web-rs/src/offline/mode.rs` (`decide`, `settle`, `route_path`, `retry`, the banner, the
//! words), with Android's `features/offline/OfflineMode.kt` (its banner words, `noticeUnreachable`).

use serde_json::{json, Value};

/// How long after the last contact with the server the downloaded songs still play, in days. From
/// `web-rs/src/offline/mod.rs` `GRACE_DAYS`.
pub const GRACE_DAYS: i64 = 30;
/// One day in seconds.
const DAY: f64 = 86_400.0;
/// How often offline mode tries to reach the server again. From `mode.rs` `RETRY_MS` and `OfflineMode.RETRY_MS`.
pub const RETRY_MS: u64 = 30_000;
/// How long a check gets to answer (a server whose address drops packets would otherwise keep the page waiting
/// for the connect timeout). From `mode.rs` `WAIT_MS` and `OfflineMode.WAIT_SECONDS`.
pub const WAIT_MS: u64 = 8_000;
/// While the server answers, the remembered user (the grace) is refreshed at most this often. From `mode.rs`
/// `REMEMBER_EVERY_MS`.
pub const REMEMBER_EVERY_MS: u64 = 3_600_000;
/// After "Back online" the page reloads this long later. From `mode.rs` `retry` (`sleep(1500)`).
pub const BACK_ONLINE_RELOAD_MS: u64 = 1500;
/// A failed call asks for a check at most this often (Android). From `OfflineMode.NOTICE_GAP_MS`.
pub const NOTICE_GAP_MS: u64 = 10_000;
/// A network change is checked this long after it (Android). From `OfflineMode.start` (`recheck(a, 1500L)`).
pub const NETWORK_CHANGE_CHECK_MS: u64 = 1500;

/// The one page offline mode shows on the web. From `mode.rs` `PAGE`.
pub const PAGE: &str = "/downloads";
/// The web's banner. From `mode.rs` `BANNER`.
pub const BANNER: &str = "Offline – the server can't be reached. Downloaded songs play.";
/// The web's banner once the grace has ended. From `mode.rs` `BANNER_ENDED`.
pub const BANNER_ENDED: &str = "Offline – sign in again when the server is back to keep playing downloaded songs";
/// The title of an entry switched off in offline mode. From `mode.rs` `DISABLED_TITLE`.
pub const DISABLED_TITLE: &str = "Not available offline";
/// The toast after a Retry the server still didn't answer. From `mode.rs` `STILL_OFFLINE` and
/// `OfflineMode.STILL_OFFLINE`.
pub const STILL_OFFLINE: &str = "The server still can't be reached";
/// The banner's button. From `mode.rs` `banner`.
pub const RETRY: &str = "Retry";
/// The banner's button while it tries. From `mode.rs` `retry_button`.
pub const TRYING: &str = "Trying…";
/// Android's full message (the offline sheet). From `OfflineMode.BANNER`.
pub const ANDROID_BANNER: &str = "Offline – showing what this phone saved. Downloaded songs play.";
/// Android's banner, first line. From `OfflineMode.BANNER_TITLE`.
pub const ANDROID_BANNER_TITLE: &str = "Offline";
/// Android's banner, second line. From `OfflineMode.BANNER_LINE`.
pub const ANDROID_BANNER_LINE: &str = "Downloaded songs play · tap for details";
/// The web's connection banner (`#conn-banner`) while the browser is offline. From `web-rs/src/app.rs` `init`
/// (`conn_banner(true, …)` on `offline`).
pub const NOT_CONNECTED: &str = "Not connected – check your internet or server";

/// The web's connection banner while `app`'s server does not answer ("The Encore server isn't answering").
/// From `web-rs/src/app.rs` `init` (`conn_banner(true, …)` on `encore:server-down`).
pub fn server_down(app: &str) -> String {
    format!("The {app} server isn't answering")
}

/// The web's banner text: [`BANNER`], or [`BANNER_ENDED`] once the grace has ended. From `mode.rs` `banner`.
pub fn banner(grace_ended: bool) -> &'static str {
    if grace_ended {
        BANNER_ENDED
    } else {
        BANNER
    }
}

/// localStorage: the remembered user (`<app>.offline.me`). From `web-rs/src/offline/mod.rs` `ME_KEY`.
pub fn me_key(app: &str) -> String {
    format!("{}.offline.me", app.to_ascii_lowercase())
}

/// localStorage: the liked track ids as this device last knew them (`<app>.offline.likes`). From
/// `web-rs/src/offline/mod.rs` `LIKES_KEY`.
pub fn likes_key(app: &str) -> String {
    format!("{}.offline.likes", app.to_ascii_lowercase())
}

/// localStorage: "1" while a sign-out has not reached the server (`<app>.signout_pending`). From
/// `web-rs/src/offline/mod.rs` `SIGNOUT_PENDING_KEY`.
pub fn signout_pending_key(app: &str) -> String {
    format!("{}.signout_pending", app.to_ascii_lowercase())
}

/// Whole days of grace left (rounded up): [`GRACE_DAYS`] right after a contact, 0 once it has ended. A clock
/// that went back counts as no time passed. Times in unix seconds, as the stored document keeps them.
///
/// From `web-rs/src/offline/mod.rs` `grace_left`.
pub fn grace_left(cached_at_s: f64, now_s: f64) -> i64 {
    let passed = (now_s - cached_at_s).max(0.0);
    let left = GRACE_DAYS as f64 * DAY - passed;
    if left <= 0.0 {
        0
    } else {
        (left / DAY).ceil() as i64
    }
}

/// What is kept of the `/auth/me` answer: all of it but the email address, which offline mode never shows.
///
/// From `web-rs/src/offline/mod.rs` `me_to_keep`.
pub fn me_to_keep(me: &Value) -> Value {
    let mut u = me.clone();
    if let Some(o) = u.as_object_mut() {
        o.remove("email");
    }
    u
}

/// The stored document: `{user, cached_at}` (unix seconds, whole).
///
/// From `web-rs/src/offline/mod.rs` `me_doc`.
pub fn me_doc(me: &Value, now_s: f64) -> Value {
    json!({ "user": me_to_keep(me), "cached_at": now_s.floor() })
}

/// The user and when it was remembered, from the stored document (None without a user object or a time).
///
/// From `web-rs/src/offline/mod.rs` `parse_me`.
pub fn parse_me(text: &str) -> Option<(Value, f64)> {
    let v: Value = serde_json::from_str(text).ok()?;
    let user = v.get("user")?;
    if !user.is_object() {
        return None;
    }
    let at = v.get("cached_at").and_then(Value::as_f64).filter(|t| t.is_finite() && *t > 0.0)?;
    Some((user.clone(), at))
}

/// Whether `/auth/me` failing with `status` opens offline mode, and with which user: the one remembered within
/// the grace (`cached`), else the last one remembered (`last`; the grace has ended → true). None: the app
/// behaves as before (the sign-in page). It needs no answer from the server, a remembered user and `saved`
/// songs on this device.
///
/// From `web-rs/src/offline/mode.rs` `decide`.
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

/// `POST /auth/logout` answered `status` (0: no answer): whether the session may still be valid. Only an answer
/// that it ended, or that there was none (401), settles it.
///
/// From `web-rs/src/offline/mod.rs` `session_may_live`.
pub fn session_may_live(status: u16) -> bool {
    !((200..300).contains(&status) || status == 401)
}

/// What boot does about a pending sign-out, from the status of the `/auth/me` answer (200: signed in).
///
/// From `web-rs/src/offline/mod.rs` `AtBoot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtBoot {
    /// No sign-out pending: as usual.
    AsUsual,
    /// The session still works: end it now (`POST /auth/logout`), then open signed out; it stays pending while
    /// [`session_may_live`] says so.
    SignOut,
    /// The server says there is no session: nothing left to end (no longer pending).
    Gone,
    /// The server can't be reached (or failed): open signed out, without offline mode, and try again next time.
    Wait,
}

/// What boot does about a pending sign-out.
///
/// From `web-rs/src/offline/mod.rs` `at_boot` (used by `mode.rs` `settle`).
pub fn at_boot(pending: bool, me_status: u16) -> AtBoot {
    match (pending, me_status) {
        (false, _) => AtBoot::AsUsual,
        (true, 200..=299) => AtBoot::SignOut,
        (true, 401) => AtBoot::Gone,
        (true, _) => AtBoot::Wait,
    }
}

/// The path to show for `path`: in offline mode everything but [`PAGE`] becomes [`PAGE`] (the web replaces the
/// address, so Back doesn't land on a page that can't open).
///
/// From `web-rs/src/offline/mode.rs` `route_path` / `allowed`.
pub fn route_path(active: bool, path: &str) -> &str {
    if !active || path == PAGE {
        path
    } else {
        PAGE
    }
}

/// What a retry's `/auth/me` answer means (status 200 for an answer).
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
///
/// From `web-rs/src/offline/mode.rs` `retry`.
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
///
/// From `OfflineMode.noticeUnreachable`.
pub fn notice_unreachable(active: bool, last_ms: &mut u64, now_ms: u64) -> bool {
    if active || now_ms.saturating_sub(*last_ms) < NOTICE_GAP_MS {
        return false;
    }
    *last_ms = now_ms;
    true
}

/// "3 plays and 1 like", "1 play", "2 likes" ("" for none).
///
/// From `web-rs/src/offline/mode.rs` `counts_text` and `Outbox.countsText`.
pub fn counts_text(plays: usize, likes: usize) -> String {
    let one = |n: usize, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    match (plays, likes) {
        (0, 0) => String::new(),
        (p, 0) => one(p, "play"),
        (0, l) => one(l, "like"),
        (p, l) => format!("{} and {}", one(p, "play"), one(l, "like")),
    }
}

/// The toast once the server answers again: "Back online – 3 plays sent".
///
/// From `web-rs/src/offline/mode.rs` `back_online_text` and `Outbox.backOnlineText`.
pub fn back_online_text(plays: usize, likes: usize) -> String {
    let sent = counts_text(plays, likes);
    if sent.is_empty() {
        "Back online".into()
    } else {
        format!("Back online – {sent} sent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ported from `web-rs/src/offline/mod.rs` `the_grace_lasts_thirty_days_from_the_last_contact`.
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

    /// Ported from `web-rs/src/offline/mod.rs` `a_pending_sign_out_is_settled_only_by_the_server`.
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

    /// Ported from `web-rs/src/offline/mod.rs` `the_remembered_user_round_trips_without_the_email`.
    #[test]
    fn the_remembered_user_round_trips_without_the_email() {
        let me = json!({ "id": 3, "username": "ana", "email": "ana@example.org", "is_admin": false, "prefs": { "translate_mode": "" } });
        let doc = me_doc(&me, 1_760_000_000.7);
        assert_eq!(doc["cached_at"], json!(1_760_000_000.0));
        let (user, at) = parse_me(&doc.to_string()).unwrap();
        assert_eq!(at, 1_760_000_000.0);
        assert_eq!(user["username"], "ana");
        assert!(user.get("email").is_none(), "{user}");
        assert_eq!(parse_me("not json"), None);
        assert_eq!(parse_me(r#"{"user":null,"cached_at":5}"#), None);
        assert_eq!(parse_me(r#"{"user":{"id":1}}"#), None, "no time, no grace");
        assert_eq!(me_key("Encore"), "encore.offline.me");
        assert_eq!(likes_key("Encore"), "encore.offline.likes");
        assert_eq!(signout_pending_key("Encore"), "encore.signout_pending");
    }

    /// Ported from `web-rs/src/offline/mode.rs` `offline_mode_needs_no_answer_a_remembered_user_and_downloads`.
    #[test]
    fn offline_mode_needs_no_answer_a_remembered_user_and_downloads() {
        let ana = || Some(json!({ "id": 1, "username": "ana" }));
        assert_eq!(decide(0, ana(), ana(), 3), Some((json!({ "id": 1, "username": "ana" }), false)));
        assert_eq!(decide(502, ana(), ana(), 3).map(|x| x.1), Some(false), "a proxy saying the server is down");
        assert_eq!(decide(0, None, ana(), 3).map(|x| x.1), Some(true), "the grace has ended: sign in again");
        assert_eq!(decide::<Value>(0, None, None, 3), None, "nobody remembered");
        assert_eq!(decide(0, ana(), ana(), 0), None, "nothing downloaded");
        assert_eq!(decide(401, ana(), ana(), 3), None, "the server answered: signed out");
        assert_eq!(decide(500, ana(), ana(), 3), None);
    }

    /// Ported from `web-rs/src/offline/mode.rs` `the_counts_read_as_words` and `OutboxTest.backOnlineSaysWhatWasSent`.
    #[test]
    fn the_counts_read_as_words() {
        assert_eq!(counts_text(0, 0), "");
        assert_eq!(counts_text(1, 0), "1 play");
        assert_eq!(counts_text(3, 1), "3 plays and 1 like");
        assert_eq!(counts_text(0, 2), "2 likes");
        assert_eq!(back_online_text(0, 0), "Back online");
        assert_eq!(back_online_text(3, 0), "Back online – 3 plays sent");
        assert_eq!(back_online_text(3, 1), "Back online – 3 plays and 1 like sent");
    }

    #[test]
    fn offline_mode_shows_only_the_downloads_page() {
        assert_eq!(route_path(false, "/library"), "/library");
        assert_eq!(route_path(true, "/library"), "/downloads");
        assert_eq!(route_path(true, "/downloads"), "/downloads");
        assert_eq!(banner(false), BANNER);
        assert_eq!(banner(true), BANNER_ENDED);
    }

    #[test]
    fn the_connection_banner_says_what_is_down() {
        assert_eq!(NOT_CONNECTED, "Not connected – check your internet or server");
        assert_eq!(server_down("Encore"), "The Encore server isn't answering");
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
