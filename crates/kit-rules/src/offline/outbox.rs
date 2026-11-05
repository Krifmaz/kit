//! The outbox: actions made while the server could not be reached, kept per account until they can be sent.
//!
//! What an action is belongs to the app (a note saved, a file starred, a page visited); it implements
//! [`Action`] to say how two of them relate. The outbox keeps the rules every app needs:
//!
//! - **once**: the same action twice (the same [`Action::key`]) is kept once;
//! - **the last state wins**: an action about a subject ([`Action::subject`]) replaces the one still waiting
//!   about the same subject (star, unstar, star → one star), in the place of the first; the same again changes
//!   nothing;
//! - **order**: actions go out in the order they were made;
//! - **a cap**: at most [`Limits::max`] wait, the oldest go first;
//! - **chunks**: they go out in calls of at most [`Limits::per_call`] ([`Outbox::calls`]);
//! - **forgetting what went**: what was sent is forgotten, but an action about the same subject that changed
//!   meanwhile stays ([`Outbox::forget`]);
//! - **per account**: the outbox belongs to the account that made the changes; a change by another account drops
//!   it first ([`Outbox::own`]), and it only goes out under that account's session ([`Outbox::belongs_to`]).
//!
//! How the account is named is the [`Owner`] trait: a user id (`i64`, the web's `user_id`) or a
//! `<server>|<username>` string (`String`, see [`account`], for a device that knows several servers).

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

/// The server takes at most this many actions per call by default.
pub const PER_CALL: usize = 500;
/// The outbox keeps at most this many actions by default (the oldest go first).
pub const MAX_ACTIONS: usize = 5000;
/// Said once when the device has no room for the outbox.
pub const NO_ROOM_FOR_OUTBOX: &str = "There is no room left on this device – changes are kept only until the page closes";

/// One action of the app's, kept until it can be sent.
pub trait Action: Clone + PartialEq + Serialize + DeserializeOwned {
    /// The action's identity: two actions with the same key are the same action (kept once, forgotten together
    /// once sent). A visit: `"visit:<page>@<time>"`; a star: `"star:<item>:<on>"`.
    fn key(&self) -> String;

    /// What the action is about when only its last state counts (a star and an unstar of item 7 are both about
    /// `"star:7"`): a new one replaces the one waiting about the same subject. None (the default): every action
    /// counts on its own.
    fn subject(&self) -> Option<String> {
        None
    }

    /// Whether the action may be kept at all (one about nothing, an id of 0, is dropped). The default: every
    /// action.
    fn valid(&self) -> bool {
        true
    }

    /// The kind of action, for the counts ("change", "upload"; see [`Outbox::counts`]).
    fn kind(&self) -> &str;
}

/// How many actions wait and go per call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// At most this many wait; the oldest go first.
    pub max: usize,
    /// At most this many go in one call.
    pub per_call: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max: MAX_ACTIONS, per_call: PER_CALL }
    }
}

/// How an outbox names the account it belongs to.
pub trait Owner: Clone + PartialEq + Default {
    /// The document's field for it (`user_id`, `owner`).
    const FIELD: &'static str;
    /// Whether the account is known: an outbox without a known owner never goes out.
    fn known(&self) -> bool;
    /// The owner from the document's field (missing or unreadable: the default, nobody).
    fn read(v: Option<&Value>) -> Self;
    /// The owner as the document's field.
    fn write(&self) -> Value;
}

/// An owner named by the user's id (0: nobody known).
impl Owner for i64 {
    const FIELD: &'static str = "user_id";
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

/// An owner named `<server>|<username>` ([`account`]; "": nobody known).
impl Owner for String {
    const FIELD: &'static str = "owner";
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

/// The name of an account on a device that knows several servers: `<server>|<username>`.
pub fn account(server: &str, username: &str) -> String {
    format!("{server}|{username}")
}

/// The outbox of one account: the app's actions `A`, the account named by `O` ([`Owner`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Outbox<A, O = i64> {
    /// The account whose actions these are.
    pub owner: O,
    /// The actions, oldest first.
    pub actions: Vec<A>,
    /// How many wait and go per call.
    pub limits: Limits,
}

impl<A, O: Default> Default for Outbox<A, O> {
    fn default() -> Self {
        Outbox { owner: O::default(), actions: Vec::new(), limits: Limits::default() }
    }
}

impl<A: Action, O: Owner> Outbox<A, O> {
    /// An empty outbox of `owner`.
    pub fn of(owner: O) -> Self {
        Outbox { owner, actions: Vec::new(), limits: Limits::default() }
    }

    /// The same outbox under other limits (a smaller cap is applied at once, the oldest go).
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self.trim();
        self
    }

    /// Actions waiting.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Nothing waits.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// How many of each kind wait, in the order each kind first appears: `[("change", 3), ("upload", 1)]`, for
    /// "3 changes and 1 upload will be sent when the server is back" ([`super::mode::counts_text`]).
    pub fn counts(&self) -> Vec<(String, usize)> {
        let mut out: Vec<(String, usize)> = Vec::new();
        for a in &self.actions {
            match out.iter_mut().find(|(k, _)| k == a.kind()) {
                Some((_, n)) => *n += 1,
                None => out.push((a.kind().to_string(), 1)),
            }
        }
        out
    }

    /// A change by `owner` is coming: the outbox of another account is dropped first.
    pub fn own(&mut self, owner: O) {
        if self.owner != owner {
            self.owner = owner;
            self.actions.clear();
        }
    }

    /// These may go out under the session of `owner` (an outbox without a known owner never goes out).
    pub fn belongs_to(&self, owner: &O) -> bool {
        self.owner.known() && self.owner == *owner
    }

    /// Keeps an action: once (an action with the same key waits already: nothing changes); about a subject that
    /// waits already, it takes that one's place; else at the end. At most [`Limits::max`] wait, the oldest go.
    /// Returns whether the outbox changed.
    pub fn add(&mut self, a: A) -> bool {
        if !a.valid() {
            return false;
        }
        let key = a.key();
        if let Some(subject) = a.subject() {
            if let Some(i) = self.actions.iter().position(|x| x.subject().as_deref() == Some(subject.as_str())) {
                if self.actions[i].key() == key {
                    return false;
                }
                self.actions[i] = a;
                return true;
            }
        } else if self.actions.iter().any(|x| x.key() == key) {
            return false;
        }
        self.actions.push(a);
        self.trim();
        true
    }

    fn trim(&mut self) {
        if self.actions.len() > self.limits.max {
            let extra = self.actions.len() - self.limits.max;
            self.actions.drain(..extra);
        }
    }

    /// What went: every action with the key of one sent is forgotten (one about the same subject that changed
    /// meanwhile has another key, and stays).
    pub fn forget(&mut self, sent: &[A]) {
        let keys: HashSet<String> = sent.iter().map(Action::key).collect();
        self.actions.retain(|a| !keys.contains(&a.key()));
    }

    /// Whether an action about `subject` waits: a new one must go after it, through the outbox, so the server
    /// sees them in order.
    pub fn has(&self, subject: &str) -> bool {
        self.actions.iter().any(|a| a.subject().as_deref() == Some(subject))
    }

    /// The actions in calls of at most [`Limits::per_call`], in order.
    pub fn calls(&self) -> Vec<&[A]> {
        self.actions.chunks(self.limits.per_call.max(1)).collect()
    }

    /// The actions of one kind, in order (an app that sends each kind to its own endpoint).
    pub fn of_kind(&self, kind: &str) -> Vec<&A> {
        self.actions.iter().filter(|a| a.kind() == kind).collect()
    }

    /// The saved document; an unreadable one is empty, and unreadable or invalid actions are left out.
    pub fn parse(text: &str) -> Self {
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(text) else { return Outbox::of(O::default()) };
        let list = o.get("actions").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut out = Outbox::of(O::read(o.get(O::FIELD)));
        for v in list {
            if let Ok(a) = serde_json::from_value::<A>(v) {
                out.add(a);
            }
        }
        out
    }

    /// The document to save: `{<owner field>, actions}`.
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert(O::FIELD.into(), self.owner.write());
        m.insert("actions".into(), json!(self.actions));
        Value::Object(m)
    }
}

/// Whether an action failing with `status` still counts as sent: 404, what it was about is gone – nothing left
/// to change.
pub fn gone(status: u16) -> bool {
    status == 404
}

/// Whether actions whose call failed with `status` wait to be sent again later: no answer (0), a timeout (408),
/// too many calls (429), the server's own trouble (5xx), or a session that is not (yet) valid (401, 403: sign in
/// again and they go). Any other answer refused them for good, and they are dropped, as are those [`gone`] says
/// went.
pub fn retry_later(status: u16) -> bool {
    matches!(status, 0 | 401 | 403 | 408 | 429) || status >= 500
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// An app's actions: a page visited (each counts), an item starred or not (the last tap counts).
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    enum Act {
        Visit { page: i64, at: i64 },
        Star { item: i64, on: bool, at: i64 },
    }

    impl Action for Act {
        fn key(&self) -> String {
            match self {
                Act::Visit { page, at } => format!("visit:{page}@{at}"),
                Act::Star { item, on, .. } => format!("star:{item}:{on}"),
            }
        }
        fn subject(&self) -> Option<String> {
            match self {
                Act::Visit { .. } => None,
                Act::Star { item, .. } => Some(format!("star:{item}")),
            }
        }
        fn valid(&self) -> bool {
            match self {
                Act::Visit { page, .. } => *page > 0,
                Act::Star { item, .. } => *item > 0,
            }
        }
        fn kind(&self) -> &str {
            match self {
                Act::Visit { .. } => "visit",
                Act::Star { .. } => "star",
            }
        }
    }

    #[test]
    fn a_failed_call_waits_unless_the_server_refused_it() {
        for s in [0, 401, 403, 408, 429, 500, 502, 503, 504] {
            assert!(retry_later(s), "{s} waits");
        }
        for s in [400, 404, 409, 413, 422] {
            assert!(!retry_later(s), "{s} is dropped");
        }
        assert!(gone(404) && !gone(400));
    }

    fn star(item: i64, on: bool) -> Act {
        Act::Star { item, on, at: 100 }
    }
    fn visit(page: i64, at: i64) -> Act {
        Act::Visit { page, at }
    }

    #[test]
    fn a_star_then_an_unstar_collapses_into_the_unstar() {
        let mut o: Outbox<Act> = Outbox::default();
        o.add(star(1, true));
        o.add(star(1, false));
        assert_eq!(o.actions, vec![star(1, false)], "the last tap is what the server gets");
        o.add(star(2, false));
        o.add(star(2, true));
        assert_eq!(o.actions, vec![star(1, false), star(2, true)]);
        o.add(star(2, false));
        o.add(star(2, true));
        assert_eq!(o.actions, vec![star(1, false), star(2, true)], "four presses: one star");
        assert!(!o.add(Act::Star { item: 2, on: true, at: 999 }), "the same again changes nothing");
        assert_eq!(o.actions[1], star(2, true), "and keeps the first time");
        assert!(!o.add(star(0, true)), "about nothing: not kept");
        assert!(o.has("star:2") && !o.has("star:3"));
    }

    #[test]
    fn a_star_on_its_way_is_replaced_not_cancelled() {
        let mut o: Outbox<Act> = Outbox::default();
        o.add(star(1, true));
        o.add(star(1, false));
        o.forget(&[star(1, true)]);
        assert_eq!(o.actions, vec![star(1, false)], "it changed meanwhile: it stays");
        o.forget(&[star(1, false)]);
        assert!(o.is_empty());
    }

    #[test]
    fn actions_are_kept_once_and_forgotten_once_sent() {
        let mut o: Outbox<Act> = Outbox::default();
        o.add(visit(1, 1000));
        o.add(visit(1, 1000));
        o.add(visit(1, 2000));
        o.add(visit(2, 1000));
        o.add(visit(-3, 1000));
        assert_eq!(o.len(), 3);
        let sent = o.actions[..2].to_vec();
        o.add(visit(4, 3000));
        o.forget(&sent);
        assert_eq!(o.actions, vec![visit(2, 1000), visit(4, 3000)], "an action kept while sending stays");
    }

    #[test]
    fn at_most_the_cap_waits_the_oldest_go_first_and_they_go_in_chunks() {
        let mut big: Outbox<Act> = Outbox::default();
        for i in 0..(MAX_ACTIONS as i64 + 3) {
            big.add(visit(7, i));
        }
        assert_eq!(big.len(), MAX_ACTIONS);
        assert_eq!(big.actions[0], visit(7, 3), "the oldest went");
        assert_eq!(big.calls().len(), 10);
        assert!(big.calls().iter().all(|c| c.len() == PER_CALL));
        let small = big.clone().with_limits(Limits { max: 5, per_call: 2 });
        assert_eq!(small.len(), 5);
        assert_eq!(small.actions[0], visit(7, MAX_ACTIONS as i64 - 2));
        assert_eq!(small.calls().iter().map(|c| c.len()).collect::<Vec<_>>(), [2, 2, 1]);
    }

    #[test]
    fn they_go_in_the_order_they_were_made_and_count_by_kind() {
        let mut o: Outbox<Act> = Outbox::default();
        o.add(star(5, true));
        o.add(visit(1, 10));
        o.add(visit(2, 11));
        o.add(star(6, true));
        o.add(star(5, false));
        assert_eq!(o.actions, vec![star(5, false), visit(1, 10), visit(2, 11), star(6, true)], "a replaced star keeps its place");
        assert_eq!(o.counts(), vec![("star".to_string(), 2), ("visit".to_string(), 2)]);
        assert_eq!(o.of_kind("visit"), vec![&visit(1, 10), &visit(2, 11)]);
        assert!(gone(404) && !gone(500));
    }

    #[test]
    fn one_users_outbox_never_goes_out_for_another() {
        let mut o: Outbox<Act> = Outbox::default();
        o.own(1);
        o.add(visit(5, 1000));
        o.add(star(6, false));
        assert!(o.belongs_to(&1));
        assert!(!o.belongs_to(&2));
        assert!(!o.belongs_to(&0));
        let text = o.to_json().to_string();
        assert_eq!(Outbox::<Act, i64>::parse(&text).owner, 1, "the owner is saved with the document");
        let old = Outbox::<Act, i64>::parse(r#"{"actions":[{"type":"visit","page":3,"at":10}]}"#);
        assert_eq!(old.owner, 0);
        assert_eq!(old.len(), 1);
        assert!(!old.belongs_to(&0) && !old.belongs_to(&3));
    }

    #[test]
    fn a_change_by_another_user_empties_the_outbox_first() {
        let mut o: Outbox<Act> = Outbox::default();
        o.own(1);
        o.add(visit(5, 1000));
        o.own(1);
        o.add(star(6, true));
        assert_eq!((o.owner, o.len()), (1, 2));
        o.own(2);
        o.add(star(8, false));
        assert_eq!(o, Outbox { owner: 2, actions: vec![star(8, false)], limits: Limits::default() }, "user 1's actions are gone");
    }

    #[test]
    fn the_document_round_trips() {
        let mut o: Outbox<Act> = Outbox::default();
        o.add(visit(1, 10));
        o.add(star(2, true));
        let text = o.to_json().to_string();
        assert_eq!(Outbox::parse(&text), o);
        assert_eq!(Outbox::<Act>::parse("garbage"), Outbox::default());
        // unreadable and invalid actions are left out
        let mixed = Outbox::<Act>::parse(
            r#"{"actions":[{"type":"visit","page":3,"at":1},{"type":"what"},{"type":"star","item":0,"on":true,"at":1}]}"#,
        );
        assert_eq!(mixed.actions, vec![visit(3, 1)]);
        assert_eq!(o.to_json()["user_id"], json!(0));
        assert_eq!(o.to_json()["actions"][1], json!({ "type": "star", "item": 2, "on": true, "at": 100 }));
    }

    fn ana() -> String {
        account("https://files.example.com", "ana")
    }

    #[test]
    fn an_account_on_a_device_with_several_servers() {
        let mut o: Outbox<Act, String> = Outbox::of(ana());
        o.add(visit(1, 5));
        assert!(o.belongs_to(&ana()));
        let bob = account("https://files.example.com", "bob");
        assert!(!o.belongs_to(&bob));
        assert!(!Outbox::<Act, String>::of(String::new()).belongs_to(&String::new()));
        let mut same = o.clone();
        same.own(ana());
        assert_eq!(same, o);
        assert_eq!(Outbox::<Act, String>::parse(&o.to_json().to_string()), o);
        assert_eq!(o.to_json()["owner"], json!(ana()));
        o.own(bob);
        assert!(o.is_empty());
    }
}
