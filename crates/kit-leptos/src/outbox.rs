//! The outbox store: the app's actions made while the server can't be reached, kept in `localStorage` per
//! account and sent once it answers again.
//!
//! The rules (kept once, the last state per subject wins, order, the cap, chunks, per account, what counts as
//! sent) are kit-rules' [`Outbox`]; this is the browser glue around it:
//!
//! - the outbox is saved under `<app id>.outbox.<name>` after every change, and read back at start;
//! - [`Store::send_or_keep`] sends an action now, or keeps it when the server can't take it yet (or when an
//!   action about the same subject still waits, so the server sees them in order);
//! - [`Store::flush`] sends what waits, in calls of at most [`Limits::per_call`], under the account it belongs
//!   to only;
//! - [`Store::watch`] flushes at start, on the browser's `online` and on `<app id>:server-up`, and says what went
//!   with a toast ("Back online – 3 changes sent");
//! - [`Store::waiting`] is a signal of how many wait, for a badge or the offline banner.
//!
//! ```rust,ignore
//! use kit_leptos::outbox::Store;
//!
//! let outbox: Store<MyAction> = Store::open("changes", |actions| {
//!     Box::pin(async move { kit_leptos::api::post("/sync", serde_json::json!({ "actions": actions })).await.map(|_| ()) })
//! })
//! .noun("star", "star", "stars");
//! outbox.watch(|| current_user_id()); // Some(id) while signed in
//! // a tap: sent now, or kept until the server is back
//! outbox.send_or_keep(user_id, MyAction::Star { item: 7, on: true }).await;
//! ```

use crate::api::ApiError;
use kit_rules::offline::mode::Count;
use kit_rules::offline::outbox::{gone, retry_later, Action, Limits, Outbox, Owner, NO_ROOM_FOR_OUTBOX};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// A call that sends some actions: done, or why not.
pub type SendFuture = Pin<Box<dyn Future<Output = Result<(), ApiError>>>>;

/// What [`Store::send_or_keep`] says of an action about nothing (its [`Action::valid`] is false).
pub const NOTHING: &str = "There is nothing to change";

/// The noun for a kind the app did not name ([`Store::noun`]).
pub const CHANGE: (&str, &str) = ("change", "changes");

/// Where an outbox is kept: `localStorage` in the browser ([`LocalStorage`]), memory in tests ([`Memory`]).
pub trait Keep {
    /// The text saved under `key`.
    fn read(&self, key: &str) -> Option<String>;
    /// Saves `text` under `key`; false when there is no room.
    fn write(&self, key: &str, text: &str) -> bool;
    /// Forgets `key`.
    fn remove(&self, key: &str);
}

/// The browser's `localStorage` (off the browser, or where it is blocked, nothing is kept and every write
/// succeeds).
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalStorage;

fn local() -> Option<web_sys::Storage> {
    crate::rt::window()?.local_storage().ok().flatten()
}

impl Keep for LocalStorage {
    fn read(&self, key: &str) -> Option<String> {
        local()?.get_item(key).ok().flatten()
    }
    fn write(&self, key: &str, text: &str) -> bool {
        local().is_none_or(|s| s.set_item(key, text).is_ok())
    }
    fn remove(&self, key: &str) {
        if let Some(s) = local() {
            let _ = s.remove_item(key);
        }
    }
}

/// Kept in memory, shared by its clones (tests, or an app that must not write to the device). `full` makes every
/// write fail, as a device with no room left.
#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// What is kept, by key.
    pub items: Rc<RefCell<HashMap<String, String>>>,
    /// Every write fails (no room).
    pub full: Rc<Cell<bool>>,
}

impl Keep for Memory {
    fn read(&self, key: &str) -> Option<String> {
        self.items.borrow().get(key).cloned()
    }
    fn write(&self, key: &str, text: &str) -> bool {
        if self.full.get() {
            return false;
        }
        self.items.borrow_mut().insert(key.to_string(), text.to_string());
        true
    }
    fn remove(&self, key: &str) {
        self.items.borrow_mut().remove(key);
    }
}

/// What a [`Store::flush`] did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flushed {
    /// How many of each kind went (the server took them, or what they were about is gone), in the order each kind
    /// first went.
    pub sent: Vec<(String, usize)>,
    /// How many the server refused for good (they are dropped).
    pub refused: usize,
    /// Why the flush stopped or what was refused: the last failed call, if any.
    pub error: Option<ApiError>,
}

impl Flushed {
    /// Nothing went.
    pub fn nothing_sent(&self) -> bool {
        self.sent.iter().all(|(_, n)| *n == 0)
    }
    fn tally(&mut self, sent: &[impl Action]) {
        for a in sent {
            match self.sent.iter_mut().find(|(k, _)| k == a.kind()) {
                Some((_, n)) => *n += 1,
                None => self.sent.push((a.kind().to_string(), 1)),
            }
        }
    }
}

/// What [`Store::send_or_keep`] did with an action.
#[derive(Clone, Debug, PartialEq)]
pub enum Sent {
    /// The server took it.
    Now,
    /// It waits in the outbox (the server can't take it yet, or an action about the same subject waits).
    Kept,
    /// The server refused it for good (not kept).
    Refused(ApiError),
}

struct Inner<A, O> {
    key: String,
    keep: Box<dyn Keep>,
    outbox: RefCell<Outbox<A, O>>,
    send: Box<dyn Fn(Vec<A>) -> SendFuture>,
    nouns: RefCell<Vec<(String, &'static str, &'static str)>>,
    no_room: Cell<&'static str>,
    said_no_room: Cell<bool>,
    waiting: ArcRwSignal<usize>,
    flushing: Cell<bool>,
}

/// One outbox of the app's actions `A`, owned by an account named by `O` (a user id by default), kept on the
/// device. Clones share the same outbox.
pub struct Store<A, O = i64> {
    inner: Rc<Inner<A, O>>,
}

impl<A, O> Clone for Store<A, O> {
    fn clone(&self) -> Self {
        Store { inner: self.inner.clone() }
    }
}

impl<A: Action + 'static, O: Owner + 'static> Store<A, O> {
    /// The outbox `name` of the configured app, in `localStorage` under `<app id>.outbox.<name>`; `send` makes
    /// the call that sends a chunk of actions.
    pub fn open(name: &str, send: impl Fn(Vec<A>) -> SendFuture + 'static) -> Self {
        let key = crate::config::get().app.key(&format!("outbox.{name}"));
        Self::new(&key, LocalStorage, send)
    }

    /// The outbox kept under `key` in `keep`.
    pub fn new(key: &str, keep: impl Keep + 'static, send: impl Fn(Vec<A>) -> SendFuture + 'static) -> Self {
        let outbox = keep.read(key).map(|t| Outbox::parse(&t)).unwrap_or_else(|| Outbox::of(O::default()));
        let waiting = ArcRwSignal::new(outbox.len());
        Store {
            inner: Rc::new(Inner {
                key: key.to_string(),
                keep: Box::new(keep),
                outbox: RefCell::new(outbox),
                send: Box::new(send),
                nouns: RefCell::new(Vec::new()),
                no_room: Cell::new(NO_ROOM_FOR_OUTBOX),
                said_no_room: Cell::new(false),
                waiting,
                flushing: Cell::new(false),
            }),
        }
    }

    /// The words for one and for several actions of `kind` ("star", "stars"); a kind without them is a
    /// [`CHANGE`].
    pub fn noun(self, kind: &str, one: &'static str, many: &'static str) -> Self {
        {
            let mut n = self.inner.nouns.borrow_mut();
            n.retain(|(k, _, _)| k != kind);
            n.push((kind.to_string(), one, many));
        }
        self
    }

    /// Other limits than kit-rules' (at most 5000 wait, 500 per call). A smaller cap applies at once.
    pub fn limits(self, limits: Limits) -> Self {
        {
            let mut o = self.inner.outbox.borrow_mut();
            let taken = std::mem::take(&mut *o);
            *o = taken.with_limits(limits);
        }
        self.save();
        self
    }

    /// The toast said once when the device has no room for the outbox (kit-rules' [`NO_ROOM_FOR_OUTBOX`]).
    pub fn no_room_words(self, words: &'static str) -> Self {
        self.inner.no_room.set(words);
        self
    }

    /// How many actions wait, as a signal.
    pub fn waiting(&self) -> ArcReadSignal<usize> {
        self.inner.waiting.read_only()
    }

    /// How many actions wait.
    pub fn len(&self) -> usize {
        self.inner.outbox.borrow().len()
    }

    /// Nothing waits.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether an action about `subject` waits.
    pub fn has(&self, subject: &str) -> bool {
        self.inner.outbox.borrow().has(subject)
    }

    /// A copy of the outbox as it is now.
    pub fn snapshot(&self) -> Outbox<A, O> {
        self.inner.outbox.borrow().clone()
    }

    /// What waits in words: "3 stars and 1 change" ("" when nothing waits).
    pub fn waiting_text(&self) -> String {
        self.words(&self.inner.outbox.borrow().counts())
    }

    /// The toast once the server is back and `sent` went: "Back online – 3 stars sent" (the app's
    /// [`OfflineWords`](kit_rules::offline::mode::OfflineWords)).
    pub fn back_online_text(&self, sent: &[(String, usize)]) -> String {
        let counts = self.counts(sent);
        crate::config::get().words.offline.back_online(&counts)
    }

    fn counts<'a>(&self, counts: &'a [(String, usize)]) -> Vec<Count<'a>> {
        let nouns = self.inner.nouns.borrow();
        counts
            .iter()
            .map(|(kind, n)| {
                let (one, many) = nouns.iter().find(|(k, _, _)| k == kind).map(|(_, o, m)| (*o, *m)).unwrap_or(CHANGE);
                Count { n: *n, one, many }
            })
            .collect()
    }

    fn words(&self, counts: &[(String, usize)]) -> String {
        kit_rules::offline::mode::counts_text(&self.counts(counts))
    }

    fn save(&self) {
        let o = self.inner.outbox.borrow();
        self.inner.waiting.set(o.len());
        let ok = if o.is_empty() {
            self.inner.keep.remove(&self.inner.key);
            true
        } else {
            self.inner.keep.write(&self.inner.key, &o.to_json().to_string())
        };
        drop(o);
        if ok {
            self.inner.said_no_room.set(false);
        } else if !self.inner.said_no_room.replace(true) {
            crate::toasts::toast(self.inner.no_room.get(), "warn");
        }
    }

    /// Keeps `a`, made by `owner` (an outbox of another account is dropped first). Returns whether the outbox
    /// changed.
    pub fn add(&self, owner: O, a: A) -> bool {
        let changed = {
            let mut o = self.inner.outbox.borrow_mut();
            let before = o.owner.clone();
            o.own(owner);
            let owner_changed = o.owner != before;
            o.add(a) || owner_changed
        };
        if changed {
            self.save();
        }
        changed
    }

    /// Forgets every action (the account signed out and its changes must not stay on the device).
    pub fn clear(&self) {
        let limits = self.inner.outbox.borrow().limits;
        *self.inner.outbox.borrow_mut() = Outbox::of(O::default()).with_limits(limits);
        self.save();
    }

    /// Sends `a` now, or keeps it: when an action about the same subject waits (so they reach the server in
    /// order), it is kept and a flush starts; when the call fails in a way that may work later
    /// ([`retry_later`]), it is kept.
    pub async fn send_or_keep(&self, owner: O, a: A) -> Sent {
        if !a.valid() {
            return Sent::Refused(ApiError::new(NOTHING));
        }
        let waits_before = self.inner.outbox.borrow().belongs_to(&owner) && a.subject().is_some_and(|s| self.has(&s));
        if waits_before {
            self.add(owner.clone(), a);
            let me = self.clone();
            crate::rt::spawn(async move {
                me.flush(&owner).await;
            });
            return Sent::Kept;
        }
        match (self.inner.send)(vec![a.clone()]).await {
            Ok(()) => Sent::Now,
            Err(e) if gone(e.status) => Sent::Now,
            Err(e) if retry_later(e.status) => {
                self.add(owner, a);
                Sent::Kept
            }
            Err(e) => Sent::Refused(e),
        }
    }

    /// Sends what waits under `owner`'s session, oldest first, in chunks; what went is forgotten. It stops at the
    /// first call that may work later; a chunk the server refused for good is dropped. Only one flush runs at a
    /// time (a second one meanwhile does nothing).
    pub async fn flush(&self, owner: &O) -> Flushed {
        let mut out = Flushed::default();
        if self.inner.flushing.replace(true) {
            return out;
        }
        loop {
            let chunk: Vec<A> = {
                let o = self.inner.outbox.borrow();
                if !o.belongs_to(owner) {
                    break;
                }
                match o.calls().first() {
                    Some(c) => c.to_vec(),
                    None => break,
                }
            };
            match (self.inner.send)(chunk.clone()).await {
                Ok(()) => out.tally(&chunk),
                Err(e) if gone(e.status) => out.tally(&chunk),
                Err(e) if retry_later(e.status) => {
                    out.error = Some(e);
                    break;
                }
                Err(e) => {
                    out.refused += chunk.len();
                    out.error = Some(e);
                }
            }
            self.inner.outbox.borrow_mut().forget(&chunk);
            self.save();
        }
        self.inner.flushing.set(false);
        out
    }

    /// Flushes now, on the browser's `online` and on the app's `<id>:server-up`, while `owner` names a signed-in
    /// account. When something went it says so ([`Store::back_online_text`]); a refused chunk is an error toast
    /// with the server's words.
    pub fn watch(&self, owner: impl Fn() -> Option<O> + 'static) {
        let owner: Rc<dyn Fn() -> Option<O>> = Rc::new(owner);
        let go = {
            let me = self.clone();
            Rc::new(move || {
                let (me, owner) = (me.clone(), owner.clone());
                crate::rt::spawn(async move {
                    let Some(who) = owner() else { return };
                    let done = me.flush(&who).await;
                    me.say(&done);
                });
            })
        };
        go();
        let id = crate::config::get().app.id.to_ascii_lowercase();
        let g = go.clone();
        crate::rt::on_window("online", move |_| g());
        crate::rt::on_window(&format!("{id}:server-up"), move |_| go());
    }

    /// The toasts after a flush: what went ("Back online – 3 stars sent"), and what the server refused.
    pub fn say(&self, done: &Flushed) {
        if !done.nothing_sent() {
            crate::toasts::toast(&self.back_online_text(&done.sent), "ok");
        }
        if done.refused > 0 {
            if let Some(e) = &done.error {
                crate::toasts::toast(&e.message, "err");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "lowercase")]
    enum Act {
        Note { id: i64, at: i64 },
        Star { item: i64, on: bool },
    }

    impl Action for Act {
        fn key(&self) -> String {
            match self {
                Act::Note { id, at } => format!("note:{id}@{at}"),
                Act::Star { item, on } => format!("star:{item}:{on}"),
            }
        }
        fn subject(&self) -> Option<String> {
            match self {
                Act::Note { .. } => None,
                Act::Star { item, .. } => Some(format!("star:{item}")),
            }
        }
        fn valid(&self) -> bool {
            match self {
                Act::Note { id, .. } => *id > 0,
                Act::Star { item, .. } => *item > 0,
            }
        }
        fn kind(&self) -> &str {
            match self {
                Act::Note { .. } => "note",
                Act::Star { .. } => "star",
            }
        }
    }

    /// A fake server: answers with the next status of `answers` (200 once they run out), and keeps every call.
    #[derive(Clone, Default)]
    struct Server {
        answers: Rc<RefCell<Vec<u16>>>,
        calls: Rc<RefCell<Vec<Vec<Act>>>>,
    }

    impl Server {
        fn answering(statuses: &[u16]) -> Self {
            let s = Server::default();
            *s.answers.borrow_mut() = statuses.to_vec();
            s
        }
        fn sender(&self) -> impl Fn(Vec<Act>) -> SendFuture + 'static {
            let s = self.clone();
            move |actions| {
                s.calls.borrow_mut().push(actions);
                let status = {
                    let mut a = s.answers.borrow_mut();
                    if a.is_empty() {
                        200
                    } else {
                        a.remove(0)
                    }
                };
                Box::pin(async move {
                    if (200..300).contains(&status) {
                        Ok(())
                    } else {
                        Err(ApiError { status, message: format!("status {status}"), data: serde_json::Value::Null })
                    }
                })
            }
        }
        fn sent(&self) -> Vec<Vec<Act>> {
            self.calls.borrow().clone()
        }
    }

    fn note(id: i64, at: i64) -> Act {
        Act::Note { id, at }
    }
    fn star(item: i64, on: bool) -> Act {
        Act::Star { item, on }
    }
    fn store(keep: &Memory, server: &Server) -> Store<Act> {
        Store::new("demo.outbox.changes", keep.clone(), server.sender()).noun("star", "star", "stars")
    }

    #[test]
    fn it_is_saved_and_read_back() {
        let keep = Memory::default();
        let server = Server::default();
        let s = store(&keep, &server);
        assert!(s.add(1, note(1, 10)));
        assert!(s.add(1, star(7, true)));
        assert!(!s.add(1, star(7, true)), "the same action again changes nothing");
        assert_eq!(s.waiting().get_untracked(), 2);
        let again = store(&keep, &server);
        assert_eq!(again.snapshot(), s.snapshot(), "a new page reads what the last one kept");
        assert_eq!(again.snapshot().owner, 1);
        s.clear();
        assert!(keep.read("demo.outbox.changes").is_none(), "an empty outbox is not kept");
        assert_eq!(s.waiting().get_untracked(), 0);
    }

    #[test]
    fn the_last_state_wins_and_another_account_starts_over() {
        let keep = Memory::default();
        let s = store(&keep, &Server::default());
        s.add(1, star(7, true));
        s.add(1, star(7, false));
        assert_eq!(s.snapshot().actions, [star(7, false)]);
        assert!(s.add(2, note(3, 1)));
        assert_eq!(s.snapshot().actions, [note(3, 1)], "user 1's actions are gone");
        assert_eq!(s.snapshot().owner, 2);
    }

    #[test]
    fn a_flush_sends_in_order_in_chunks_and_forgets_what_went() {
        let keep = Memory::default();
        let server = Server::default();
        let s = store(&keep, &server).limits(Limits { max: 100, per_call: 2 });
        for i in 1..=5 {
            s.add(1, note(i, i));
        }
        s.add(1, star(9, true));
        let done = block_on(s.flush(&1));
        assert_eq!(server.sent().iter().map(Vec::len).collect::<Vec<_>>(), [2, 2, 2]);
        assert_eq!(server.sent()[0], [note(1, 1), note(2, 2)], "oldest first");
        assert_eq!(done.sent, [("note".to_string(), 5), ("star".to_string(), 1)]);
        assert!(s.is_empty() && keep.read("demo.outbox.changes").is_none());
        assert_eq!(s.back_online_text(&done.sent), "Back online – 5 changes and 1 star sent");
    }

    #[test]
    fn it_only_goes_out_under_its_own_account() {
        let server = Server::default();
        let s = store(&Memory::default(), &server);
        s.add(1, note(1, 1));
        let done = block_on(s.flush(&2));
        assert!(done.nothing_sent() && server.sent().is_empty() && s.len() == 1);
    }

    #[test]
    fn a_server_that_cannot_take_them_yet_keeps_them() {
        let server = Server::answering(&[0, 503]);
        let s = store(&Memory::default(), &server);
        s.add(1, note(1, 1));
        let done = block_on(s.flush(&1));
        assert_eq!(done.error.map(|e| e.status), Some(0));
        assert_eq!(s.len(), 1, "no answer: it waits");
        let done = block_on(s.flush(&1));
        assert_eq!(done.error.map(|e| e.status), Some(503));
        assert_eq!(s.len(), 1);
        let done = block_on(s.flush(&1));
        assert_eq!(done.sent, [("note".to_string(), 1)]);
        assert!(s.is_empty());
    }

    #[test]
    fn refused_and_gone_are_not_sent_again() {
        let server = Server::answering(&[422, 404]);
        let s = store(&Memory::default(), &server).limits(Limits { max: 100, per_call: 1 });
        s.add(1, note(1, 1));
        s.add(1, note(2, 2));
        s.add(1, note(3, 3));
        let done = block_on(s.flush(&1));
        assert_eq!(done.refused, 1, "the 422 is dropped");
        assert_eq!(done.sent, [("note".to_string(), 2)], "a 404 counts as sent");
        assert_eq!(done.error.map(|e| e.status), Some(422));
        assert!(s.is_empty());
        assert_eq!(server.sent().len(), 3);
    }

    #[test]
    fn send_or_keep_sends_now_or_keeps_for_later() {
        let server = Server::answering(&[200, 0, 400]);
        let s = store(&Memory::default(), &server);
        assert_eq!(block_on(s.send_or_keep(1, note(1, 1))), Sent::Now);
        assert!(s.is_empty());
        assert_eq!(block_on(s.send_or_keep(1, star(7, true))), Sent::Kept, "no answer: kept");
        assert_eq!(s.snapshot().actions, [star(7, true)]);
        assert!(matches!(block_on(s.send_or_keep(1, note(2, 2))), Sent::Refused(e) if e.status == 400));
        assert_eq!(s.len(), 1, "a refused action is not kept");
        // a star of the same item waits: this one goes after it, through the outbox
        assert_eq!(block_on(s.send_or_keep(1, star(7, false))), Sent::Kept);
        assert_eq!(s.snapshot().actions, [star(7, false)]);
        assert_eq!(server.sent().len(), 3, "the unstar was not sent past the waiting star");
    }

    #[test]
    fn no_room_is_said_once() {
        let keep = Memory::default();
        keep.full.set(true);
        let s = store(&keep, &Server::default());
        crate::toasts::clear();
        s.add(1, note(1, 1));
        s.add(1, note(2, 2));
        let said: Vec<String> = crate::toasts::items()
            .get_untracked()
            .iter()
            .filter_map(|t| match t {
                crate::toasts::Item::Toast(e) => Some(e.text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(said, [NO_ROOM_FOR_OUTBOX.to_string()]);
        assert_eq!(s.len(), 2, "kept until the page closes");
    }

    #[test]
    fn the_words_name_each_kind() {
        let s = store(&Memory::default(), &Server::default());
        assert_eq!(s.waiting_text(), "");
        s.add(1, star(1, true));
        s.add(1, star(2, true));
        s.add(1, note(1, 1));
        assert_eq!(s.waiting_text(), "2 stars and 1 change");
        assert_eq!(s.back_online_text(&[]), "Back online");
    }
}
