//! The outbox: plays and likes made while the server could not be reached, kept per account until they can be
//! sent.
//!
//! From Encore's `web-rs/src/offline/outbox.rs` (`Outbox`, `Play`, `Like`, `plays_body`, `PLAYS_PER_CALL`,
//! `MAX_PLAYS`, `NO_ROOM_FOR_OUTBOX`) and Android's `features/offline/OfflineOutbox.kt` (`Outbox`, `OutPlay`,
//! `OutLike`; tests in `OutboxTest.kt`).
//!
//! The rules are the same on both: a play is kept once (the same song at the same second is the same play), at
//! most [`MAX_PLAYS`] wait (the oldest go first); a song's likes and unlikes collapse into the last tap; what was
//! sent is forgotten, but a like that changed meanwhile stays; the outbox belongs to the account that made the
//! changes, a change by another account drops it first, and it only goes out under that account's session.
//!
//! What differs is how the account is named: the web keeps the user's id (`user_id`, a number), Android
//! `<server>|<username>` (`owner`, a string). That is the [`Owner`] trait, implemented for `i64` (the web's
//! document) and `String` (Android's, see [`account`]).

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// The server takes at most this many plays per call. From `outbox.rs` `PLAYS_PER_CALL`.
pub const PLAYS_PER_CALL: usize = 500;
/// The outbox keeps at most this many plays (the oldest go first). From `outbox.rs` `MAX_PLAYS`.
pub const MAX_PLAYS: usize = 5000;
/// Said once when the device has no room for the outbox. From `outbox.rs` `NO_ROOM_FOR_OUTBOX`.
pub const NO_ROOM_FOR_OUTBOX: &str = "There is no room left on this device – plays and likes are kept only until the page closes";

/// A play the server did not get yet.
///
/// From `outbox.rs` `Play` and `OutPlay`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Play {
    /// The song.
    pub track_id: i64,
    /// How long it played, in seconds.
    pub seconds: f64,
    /// When (unix seconds).
    pub played_at: i64,
}

/// A like (`liked`) or an unlike the server did not get yet.
///
/// From `outbox.rs` `Like` and `OutLike`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Like {
    /// The song.
    pub track_id: i64,
    /// A like, or an unlike.
    pub liked: bool,
    /// When it was pressed (unix seconds).
    pub at: i64,
}

/// How an outbox names the account it belongs to.
pub trait Owner: Clone + PartialEq + Default {
    /// The document's field for it (`user_id` on the web, `owner` on Android).
    const FIELD: &'static str;
    /// Whether the account is known: an outbox without a known owner never goes out.
    fn known(&self) -> bool;
    /// Whether reading a document leaves out entries without a song (Android's `parse` does, the web's not).
    const DROP_UNKNOWN_TRACKS: bool;
    /// The owner from the document's field (missing or unreadable: the default, nobody).
    fn read(v: Option<&Value>) -> Self;
    /// The owner as the document's field.
    fn write(&self) -> Value;
}

/// The web's owner: the user's id (0: nobody known). From `outbox.rs` `Outbox::user_id`.
impl Owner for i64 {
    const FIELD: &'static str = "user_id";
    const DROP_UNKNOWN_TRACKS: bool = false;
    fn known(&self) -> bool {
        *self > 0
    }
    fn read(v: Option<&Value>) -> Self {
        v.and_then(Value::as_i64).unwrap_or(0)
    }
    fn write(&self) -> Value {
        json!(self)
    }
}

/// Android's owner: `<server>|<username>` ([`account`]; "": nobody known). From `OfflineOutbox.kt`
/// `Outbox.owner`.
impl Owner for String {
    const FIELD: &'static str = "owner";
    const DROP_UNKNOWN_TRACKS: bool = true;
    fn known(&self) -> bool {
        !self.is_empty()
    }
    fn read(v: Option<&Value>) -> Self {
        v.and_then(Value::as_str).unwrap_or("").to_string()
    }
    fn write(&self) -> Value {
        json!(self)
    }
}

/// Android's name for an account: `<server>|<username>`. From `OfflineOutbox.kt` (`owner`).
pub fn account(server: &str, username: &str) -> String {
    format!("{server}|{username}")
}

/// The outbox document of one account, `O` naming the account ([`Owner`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outbox<O: Owner = i64> {
    /// The account whose plays and likes these are.
    pub owner: O,
    /// The plays, oldest first.
    pub plays: Vec<Play>,
    /// The likes and unlikes, one per song, in the order first pressed.
    pub likes: Vec<Like>,
}

impl<O: Owner> Outbox<O> {
    /// An empty outbox of `owner`.
    pub fn of(owner: O) -> Self {
        Outbox { owner, plays: vec![], likes: vec![] }
    }

    /// Plays and likes waiting.
    pub fn len(&self) -> usize {
        self.plays.len() + self.likes.len()
    }

    /// Nothing waits.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// (plays, likes) waiting: "3 plays and 1 like will be sent when the server is back".
    pub fn counts(&self) -> (usize, usize) {
        (self.plays.len(), self.likes.len())
    }

    /// A change by `owner` is coming: the outbox of another account is dropped first.
    ///
    /// From `outbox.rs` `Outbox::own` and `Outbox.own`.
    pub fn own(&mut self, owner: O) {
        if self.owner != owner {
            *self = Outbox::of(owner);
        }
    }

    /// These may go out under the session of `owner` (an outbox without a known owner never goes out).
    ///
    /// From `outbox.rs` `Outbox::belongs_to` and `Outbox.belongsTo`.
    pub fn belongs_to(&self, owner: &O) -> bool {
        self.owner.known() && self.owner == *owner
    }

    /// A play, once (the same song at the same second is the same play); at most [`MAX_PLAYS`], the oldest go.
    ///
    /// From `outbox.rs` `Outbox::add_play` and `Outbox.addPlay`.
    pub fn add_play(&mut self, p: Play) {
        if p.track_id <= 0 || self.plays.iter().any(|x| x.track_id == p.track_id && x.played_at == p.played_at) {
            return;
        }
        self.plays.push(p);
        if self.plays.len() > MAX_PLAYS {
            let extra = self.plays.len() - MAX_PLAYS;
            self.plays.drain(..extra);
        }
    }

    /// A like or an unlike: it replaces the one still waiting for the same song (the last tap wins); the same
    /// again changes nothing.
    ///
    /// From `outbox.rs` `Outbox::add_like` and `Outbox.addLike`.
    pub fn add_like(&mut self, l: Like) {
        if l.track_id <= 0 {
            return;
        }
        match self.likes.iter().position(|x| x.track_id == l.track_id) {
            None => self.likes.push(l),
            Some(i) if self.likes[i].liked == l.liked => {}
            Some(i) => self.likes[i] = l,
        }
    }

    /// What went: the plays sent, and the likes sent (an entry that changed meanwhile stays).
    ///
    /// From `outbox.rs` `Outbox::forget` and `Outbox.forget`.
    pub fn forget(&mut self, plays: &[Play], likes: &[Like]) {
        let sent: HashSet<(i64, i64)> = plays.iter().map(|p| (p.track_id, p.played_at)).collect();
        self.plays.retain(|p| !sent.contains(&(p.track_id, p.played_at)));
        self.likes.retain(|l| !likes.iter().any(|s| s.track_id == l.track_id && s.liked == l.liked));
    }

    /// Whether a like or an unlike of this song waits: a new tap must go after it, through the outbox, so the
    /// server sees them in order.
    ///
    /// From `outbox.rs` `has_like`.
    pub fn has_like(&self, track_id: i64) -> bool {
        self.likes.iter().any(|l| l.track_id == track_id)
    }

    /// The plays in calls of at most [`PLAYS_PER_CALL`], each the body of `POST /library/plays`.
    ///
    /// From `outbox.rs` `send`.
    pub fn play_calls(&self) -> Vec<&[Play]> {
        self.plays.chunks(PLAYS_PER_CALL).collect()
    }

    /// The saved document; an unreadable one is empty.
    ///
    /// From `outbox.rs` `Outbox::parse` and `Outbox.parse`.
    pub fn parse(text: &str) -> Self {
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text) else { return Outbox::default() };
        let list = |k: &str| o.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
        let mut plays: Vec<Play> = list("plays").into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect();
        let mut likes: Vec<Like> = list("likes").into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect();
        if O::DROP_UNKNOWN_TRACKS {
            plays.retain(|p| p.track_id > 0);
            likes.retain(|l| l.track_id > 0);
        }
        Outbox { owner: O::read(o.get(O::FIELD)), plays, likes }
    }

    /// The document to save (`{<owner field>, plays, likes}`).
    ///
    /// From the web's `serde_json::to_string(&Outbox)` and `Outbox.toJson`.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert(O::FIELD.into(), self.owner.write());
        m.insert("plays".into(), json!(self.plays));
        m.insert("likes".into(), json!(self.likes));
        Value::Object(m)
    }
}

/// The body of `POST /library/plays` (the list form): `{"plays": [{track_id, seconds, played_at}, …]}`, the
/// fields in that order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlaysBody {
    /// The plays, seconds rounded to tenths.
    pub plays: Vec<Play>,
}

/// The body of `POST /library/plays` for these plays (the list form), seconds rounded to tenths and never below
/// zero. Serialize it (`serde_json::to_string`) to send it.
///
/// From `outbox.rs` `plays_body` and `Outbox.playsBody`.
pub fn plays_body(plays: &[Play]) -> PlaysBody {
    PlaysBody {
        plays: plays
            .iter()
            .map(|p| Play { track_id: p.track_id, seconds: (p.seconds.max(0.0) * 10.0).round() / 10.0, played_at: p.played_at })
            .collect(),
    }
}

/// The seconds a play is queued with: a number that is not finite counts as 0, never below 0.
///
/// From `outbox.rs` `queue_play`.
pub fn play_seconds(seconds: f64) -> f64 {
    if seconds.is_finite() {
        seconds.max(0.0)
    } else {
        0.0
    }
}

/// Whether a like failing with `status` still counts as sent: 404, the song is gone from the library – nothing
/// left to like.
///
/// From `outbox.rs` `send`.
pub fn like_gone(status: u16) -> bool {
    status == 404
}

#[cfg(test)]
mod tests {
    use super::*;

    fn like(id: i64, liked: bool) -> Like {
        Like { track_id: id, liked, at: 100 }
    }
    fn play(id: i64, at: i64) -> Play {
        Play { track_id: id, seconds: 120.0, played_at: at }
    }

    // ---- the web (web-rs/src/offline/outbox.rs tests) --------------------------------------------------------

    #[test]
    fn a_like_then_an_unlike_collapses_into_the_unlike() {
        let mut o: Outbox = Outbox::default();
        o.add_like(like(1, true));
        o.add_like(like(1, false));
        assert_eq!(o.likes, vec![like(1, false)], "the last tap is what the server gets");
        o.add_like(like(2, false));
        o.add_like(like(2, true));
        assert_eq!(o.likes, vec![like(1, false), like(2, true)]);
        o.add_like(like(2, false));
        o.add_like(like(2, true));
        assert_eq!(o.likes, vec![like(1, false), like(2, true)], "four presses: one like");
        o.add_like(like(2, true));
        assert_eq!(o.likes.len(), 2, "the same again changes nothing");
        o.add_like(like(0, true));
        assert_eq!(o.likes.len(), 2, "no song, no like");
        assert!(o.has_like(2) && !o.has_like(3));
    }

    #[test]
    fn a_like_on_its_way_is_replaced_not_cancelled() {
        let mut o: Outbox = Outbox::default();
        o.add_like(like(1, true));
        o.add_like(like(1, false));
        assert_eq!(o.likes, vec![like(1, false)]);
        o.forget(&[], &[like(1, true)]);
        assert_eq!(o.likes, vec![like(1, false)]);
        o.forget(&[], &[like(1, false)]);
        assert!(o.is_empty());
    }

    #[test]
    fn plays_are_kept_once_and_forgotten_once_sent() {
        let mut o: Outbox = Outbox::default();
        o.add_play(play(1, 1000));
        o.add_play(play(1, 1000));
        o.add_play(play(1, 2000));
        o.add_play(play(2, 1000));
        o.add_play(play(-3, 1000));
        assert_eq!(o.plays.len(), 3);
        let sent = o.plays[..2].to_vec();
        o.add_play(play(4, 3000));
        o.forget(&sent, &[]);
        assert_eq!(o.plays, vec![play(2, 1000), play(4, 3000)], "a play queued while sending stays");
        let mut big: Outbox = Outbox::default();
        for i in 0..(MAX_PLAYS as i64 + 3) {
            big.add_play(play(7, i));
        }
        assert_eq!(big.plays.len(), MAX_PLAYS);
        assert_eq!(big.plays[0].played_at, 3, "the oldest went");
        assert_eq!(big.play_calls().len(), 10);
        assert!(big.play_calls().iter().all(|c| c.len() == PLAYS_PER_CALL));
    }

    #[test]
    fn the_plays_go_in_the_list_form() {
        let body = serde_json::to_value(plays_body(&[
            Play { track_id: 5, seconds: 61.26, played_at: 1_760_000_000 },
            Play { track_id: 6, seconds: -2.0, played_at: 1_760_000_100 },
        ]))
        .unwrap();
        assert_eq!(
            body,
            json!({ "plays": [
                { "track_id": 5, "seconds": 61.3, "played_at": 1_760_000_000 },
                { "track_id": 6, "seconds": 0.0, "played_at": 1_760_000_100 },
            ] })
        );
        assert_eq!(serde_json::to_value(plays_body(&[])).unwrap(), json!({ "plays": [] }));
        assert_eq!(play_seconds(f64::NAN), 0.0);
        assert_eq!(play_seconds(-1.0), 0.0);
        assert_eq!(play_seconds(12.5), 12.5);
        assert!(like_gone(404) && !like_gone(500));
    }

    #[test]
    fn one_users_outbox_never_goes_out_for_another() {
        let mut o: Outbox = Outbox::default();
        o.own(1);
        o.add_play(play(5, 1000));
        o.add_like(like(6, false));
        assert!(o.belongs_to(&1));
        assert!(!o.belongs_to(&2));
        assert!(!o.belongs_to(&0));
        let text = o.to_json().to_string();
        assert_eq!(Outbox::<i64>::parse(&text).owner, 1, "the owner is saved with the document");
        let old = Outbox::<i64>::parse(r#"{"plays":[{"track_id":3,"seconds":30,"played_at":10}],"likes":[]}"#);
        assert_eq!(old.owner, 0);
        assert!(!old.belongs_to(&0) && !old.belongs_to(&3));
    }

    #[test]
    fn a_change_by_another_user_empties_the_outbox_first() {
        let mut o: Outbox = Outbox::default();
        o.own(1);
        o.add_play(play(5, 1000));
        o.own(1);
        o.add_like(like(6, true));
        assert_eq!((o.owner, o.len()), (1, 2));
        o.own(2);
        o.add_like(like(8, false));
        assert_eq!(o, Outbox { owner: 2, plays: vec![], likes: vec![like(8, false)] }, "user 1's entries are gone");
    }

    #[test]
    fn the_document_round_trips() {
        let mut o: Outbox = Outbox::default();
        o.add_play(play(1, 10));
        o.add_like(like(2, true));
        let text = o.to_json().to_string();
        assert_eq!(Outbox::parse(&text), o);
        assert_eq!(Outbox::<i64>::parse("garbage"), Outbox::default());
        assert_eq!(Outbox::<i64>::parse(r#"{"plays":[{"track_id":3}]}"#).plays, vec![Play { track_id: 3, seconds: 0.0, played_at: 0 }]);
        assert_eq!(o.len(), 2);
        assert_eq!(o.to_json()["user_id"], json!(0));
    }

    // ---- Android (OutboxTest.kt) ------------------------------------------------------------------------------

    fn ana() -> String {
        account("https://music.example.com", "ana")
    }

    #[test]
    fn android_a_like_then_an_unlike_collapses_into_the_unlike() {
        let mut o = Outbox::of(ana());
        o.add_like(Like { track_id: 1, liked: true, at: 100 });
        o.add_like(Like { track_id: 1, liked: false, at: 101 });
        assert_eq!(o.likes, vec![Like { track_id: 1, liked: false, at: 101 }]);
        let before = o.clone();
        o.add_like(Like { track_id: 1, liked: false, at: 200 });
        o.add_like(Like { track_id: 0, liked: true, at: 1 });
        assert_eq!(o, before);
    }

    #[test]
    fn android_a_play_is_kept_once() {
        let mut o = Outbox::of(ana());
        o.add_play(Play { track_id: 1, seconds: 120.0, played_at: 100 });
        o.add_play(Play { track_id: 1, seconds: 30.0, played_at: 100 });
        o.add_play(Play { track_id: 1, seconds: 30.0, played_at: 400 });
        assert_eq!(o.plays.len(), 2);
        o.add_play(Play { track_id: -1, seconds: 1.0, played_at: 1 });
        assert_eq!(o.plays.len(), 2);
    }

    #[test]
    fn android_at_most_five_thousand_plays_wait_the_oldest_go_first() {
        let mut o = Outbox::of(ana());
        for i in 1..=5003 {
            o.add_play(Play { track_id: 7, seconds: 10.0, played_at: i });
        }
        assert_eq!(o.plays.len(), MAX_PLAYS);
        assert_eq!(o.plays[0].played_at, 4);
    }

    #[test]
    fn android_another_accounts_outbox_is_dropped() {
        let mut o = Outbox::of(ana());
        o.add_play(Play { track_id: 1, seconds: 60.0, played_at: 5 });
        assert!(o.belongs_to(&ana()));
        let bob = account("https://music.example.com", "bob");
        assert!(!o.belongs_to(&bob));
        assert!(!Outbox::of(String::new()).belongs_to(&String::new()));
        let mut same = o.clone();
        same.own(ana());
        assert_eq!(same, o);
        o.own(bob);
        assert!(o.is_empty());
    }

    #[test]
    fn android_what_went_is_forgotten() {
        let mut o = Outbox::of(ana());
        o.add_play(Play { track_id: 1, seconds: 60.0, played_at: 5 });
        o.add_play(Play { track_id: 2, seconds: 60.0, played_at: 6 });
        o.add_like(Like { track_id: 3, liked: true, at: 7 });
        o.add_like(Like { track_id: 4, liked: false, at: 8 });
        // the like of 3 changed meanwhile: it stays
        o.add_like(Like { track_id: 3, liked: false, at: 9 });
        o.forget(
            &[Play { track_id: 1, seconds: 60.0, played_at: 5 }],
            &[Like { track_id: 3, liked: true, at: 7 }, Like { track_id: 4, liked: false, at: 8 }],
        );
        assert_eq!(o.plays, vec![Play { track_id: 2, seconds: 60.0, played_at: 6 }]);
        assert_eq!(o.likes, vec![Like { track_id: 3, liked: false, at: 9 }]);
    }

    #[test]
    fn android_the_document_reads_back() {
        let mut o = Outbox::of(ana());
        o.add_play(Play { track_id: 1, seconds: 241.5, played_at: 1_792_310_400 });
        o.add_like(Like { track_id: 9, liked: true, at: 1_792_310_460 });
        assert_eq!(Outbox::<String>::parse(&o.to_json().to_string()), o);
        assert_eq!(o.to_json()["owner"], json!(ana()));
        assert_eq!(Outbox::<String>::parse("not json"), Outbox::default());
        // Android leaves out entries without a song when it reads
        assert!(Outbox::<String>::parse(r#"{"owner":"x","plays":[{"track_id":0}]}"#).plays.is_empty());
    }

    #[test]
    fn android_the_plays_go_in_the_list_form() {
        let body = serde_json::to_string(&plays_body(&[
            Play { track_id: 1234, seconds: 241.56, played_at: 1_792_310_400 },
            Play { track_id: 1235, seconds: -3.0, played_at: 1_792_310_500 },
        ]))
        .unwrap();
        assert_eq!(
            body,
            r#"{"plays":[{"track_id":1234,"seconds":241.6,"played_at":1792310400},{"track_id":1235,"seconds":0.0,"played_at":1792310500}]}"#
        );
    }
}
