//! [`NoticeCheck`] (one registered notice), [`Notices`] (the app's list) and [`spawn_rechecks`].

use crate::{Notice, NoticesHost, Severity, Toasts};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A boxed future the registry keeps.
type Boxed<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// How a notice reads now: the app's state and whether an admin asks.
type CurrentFn<S> = Arc<dyn Fn(S, bool) -> Notice + Send + Sync>;
/// Check a notice again: the app's state and whether an admin asks.
type CheckFn<S> = Arc<dyn Fn(S, bool) -> Boxed<Notice> + Send + Sync>;

/// How long Check again waits for a probe that does not end before it answers with `checking` (Encore's
/// storage check: a network mount that stopped answering is not waited for longer than 15 s).
pub const CHECK_WAIT: Duration = Duration::from_secs(15);

/// One notice the app registers.
pub struct NoticeCheck<S> {
    /// the notice's id (`downloads`)
    id: String,
    /// how serious it is
    severity: Severity,
    /// the Check again toasts, when the app gives them
    toasts: Option<Toasts>,
    /// how often it is checked again by [`spawn_rechecks`]
    every: Option<Duration>,
    /// how it reads now
    current: CurrentFn<S>,
    /// check it again
    check: CheckFn<S>,
}

impl<S> Clone for NoticeCheck<S> {
    fn clone(&self) -> Self {
        NoticeCheck {
            id: self.id.clone(),
            severity: self.severity,
            toasts: self.toasts.clone(),
            every: self.every,
            current: self.current.clone(),
            check: self.check.clone(),
        }
    }
}

/// What a [`NoticeCheck::kept`] notice keeps between checks.
struct Kept {
    /// the last probe's notice
    last: Option<Notice>,
    /// a probe runs now
    running: bool,
}

impl<S: Clone + Send + Sync + 'static> NoticeCheck<S> {
    /// A notice whose state the app keeps itself (Encore's storage probe): `current` reads it now without
    /// probing (it runs on the blocking pool, so it may read the database), `check` probes again and answers
    /// the new state. Both are told whether an admin asks (everyone else gets less: Encore answers `{ok}`).
    pub fn new<C, K, Fut>(id: impl Into<String>, severity: Severity, current: C, check: K) -> Self
    where
        C: Fn(&S, bool) -> Notice + Send + Sync + 'static,
        K: Fn(S, bool) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Notice> + Send + 'static,
    {
        NoticeCheck {
            id: id.into(),
            severity,
            toasts: None,
            every: None,
            current: Arc::new(move |s, admin| current(&s, admin)),
            check: Arc::new(move |s, admin| Box::pin(check(s, admin))),
        }
    }

    /// A notice from one probe; the kit keeps its last result. Until the first probe ends it reads as fine,
    /// with `checking` while a probe runs. Check again starts a probe (or joins the one that runs) and waits
    /// for it up to [`CHECK_WAIT`]. Admins get the whole notice, everyone else `{ok}`.
    pub fn kept<P, Fut>(id: impl Into<String>, severity: Severity, probe: P) -> Self
    where
        P: Fn(S) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Notice> + Send + 'static,
    {
        let kept = Arc::new(Mutex::new(Kept { last: None, running: false }));
        let (done, _) = tokio::sync::watch::channel(0u64);
        let done = Arc::new(done);
        let probe = Arc::new(probe);
        let read = {
            let kept = kept.clone();
            move |admin: bool| {
                let k = kept.lock().unwrap_or_else(|e| e.into_inner());
                let n = k.last.clone().unwrap_or_else(Notice::fine).checking(k.running);
                if admin {
                    n
                } else {
                    n.plain()
                }
            }
        };
        let read = Arc::new(read);
        let current = {
            let read = read.clone();
            move |_: &S, admin: bool| read(admin)
        };
        let check = move |s: S, admin: bool| {
            let (kept, done, probe, read) = (kept.clone(), done.clone(), probe.clone(), read.clone());
            async move {
                let mut rx = done.subscribe();
                let seen = *rx.borrow_and_update();
                let start = {
                    let mut k = kept.lock().unwrap_or_else(|e| e.into_inner());
                    !std::mem::replace(&mut k.running, true)
                };
                if start {
                    let (kept, done) = (kept.clone(), done.clone());
                    tokio::spawn(async move {
                        let n = probe(s).await;
                        {
                            let mut k = kept.lock().unwrap_or_else(|e| e.into_inner());
                            k.last = Some(n);
                            k.running = false;
                        }
                        done.send_modify(|g| *g += 1);
                    });
                }
                let _ = tokio::time::timeout(CHECK_WAIT, rx.wait_for(|g| *g > seen)).await;
                read(admin)
            }
        };
        Self::new(id, severity, current, check)
    }

    /// The same notice with the three Check again toasts.
    pub fn toasts(mut self, toasts: Toasts) -> Self {
        self.toasts = Some(toasts);
        self
    }

    /// The same notice checked again every `every` by [`spawn_rechecks`] (the first check at once).
    pub fn every(mut self, every: Duration) -> Self {
        self.every = Some(every);
        self
    }

    /// The notice's id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// How serious it is.
    pub fn severity(&self) -> Severity {
        self.severity
    }

    /// The Check again toasts, when the app gave them.
    pub fn toast_texts(&self) -> Option<&Toasts> {
        self.toasts.as_ref()
    }

    /// How often it is checked again on its own.
    pub fn interval(&self) -> Option<Duration> {
        self.every
    }

    /// How it reads now (on the blocking pool).
    pub async fn current(&self, st: &S, admin: bool) -> Notice {
        let (f, s) = (self.current.clone(), st.clone());
        tokio::task::spawn_blocking(move || f(s, admin)).await.unwrap_or_else(|e| {
            tracing::error!("notice: reading it failed: {e}");
            Notice::fine()
        })
    }

    /// Check it again now, and how it reads then.
    pub async fn check(&self, st: &S, admin: bool) -> Notice {
        (self.check)(st.clone(), admin).await
    }

    /// The notice as the kit's routes list it: with its `id` and `severity`; only `ok` for anyone but admins.
    fn listed(&self, n: Notice, admin: bool) -> Value {
        let n = if admin { n } else { n.plain() };
        n.with("id", Value::String(self.id.clone())).with("severity", Value::String(self.severity.as_str().into())).into_json()
    }
}

/// The app's notices, in the order the routes list them. Clones share them.
pub struct Notices<S> {
    /// the notices
    list: Vec<NoticeCheck<S>>,
}

impl<S> Clone for Notices<S> {
    fn clone(&self) -> Self {
        Notices { list: self.list.clone() }
    }
}

impl<S> Default for Notices<S> {
    fn default() -> Self {
        Notices { list: vec![] }
    }
}

impl<S: Clone + Send + Sync + 'static> Notices<S> {
    /// No notices yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a notice (the routes list them in this order).
    pub fn register(mut self, n: NoticeCheck<S>) -> Self {
        self.list.push(n);
        self
    }

    /// The notice `id`.
    pub fn get(&self, id: &str) -> Option<&NoticeCheck<S>> {
        self.list.iter().find(|n| n.id == id)
    }

    /// Every notice, in order.
    pub fn all(&self) -> &[NoticeCheck<S>] {
        &self.list
    }

    /// How the notice `id` reads now, as its check made it (no `id` or `severity` added: an app serving it
    /// inside its own answer gets its own object back); `None` for an unknown id.
    pub async fn current(&self, st: &S, id: &str, admin: bool) -> Option<Notice> {
        Some(self.get(id)?.current(st, admin).await)
    }

    /// Check the notice `id` again and how it reads then; `None` for an unknown id.
    pub async fn check(&self, st: &S, id: &str, admin: bool) -> Option<Notice> {
        Some(self.get(id)?.check(st, admin).await)
    }

    /// Every notice as it reads now, as `GET /api/notices` lists them.
    pub async fn current_all(&self, st: &S, admin: bool) -> Vec<Value> {
        let futs = self.list.iter().map(|n| async move { n.listed(n.current(st, admin).await, admin) });
        futures_util::future::join_all(futs).await
    }

    /// Every notice checked again at once (as an admin), as `POST /api/notices/check` lists them.
    pub async fn check_all(&self, st: &S) -> Vec<Value> {
        let futs = self.list.iter().map(|n| async move { n.listed(n.check(st, true).await, true) });
        futures_util::future::join_all(futs).await
    }
}

/// Check every notice that has an interval ([`NoticeCheck::every`]) again on a timer, the first time at once;
/// one task per notice, running until the app stops (or the handle is aborted). The app's notices must be
/// kept in its state for the results of [`NoticeCheck::kept`] notices to stay.
pub fn spawn_rechecks<S: NoticesHost>(st: S) -> Vec<tokio::task::JoinHandle<()>> {
    st.notices()
        .all()
        .iter()
        .filter_map(|n| Some((n.clone(), n.every?)))
        .map(|(n, every)| {
            let st = st.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(every);
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tick.tick().await;
                    let r = n.check(&st, true).await;
                    tracing::debug!("notice {}: ok={}", n.id, r.ok());
                }
            })
        })
        .collect()
}
