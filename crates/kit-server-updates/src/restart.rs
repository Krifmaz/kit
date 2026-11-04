//! "Restart server": [`Restart`] plans it so the 202 answer gets out first; the app's server loop waits for
//! [`Restart::requested`], stops gracefully and exits with [`EXIT_RESTART`] (Encore's image entrypoint starts it
//! again at once; a service unit can do the same with `RestartForceExitStatus=75`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// The exit code that asks for a restart.
pub const EXIT_RESTART: i32 = 75;

/// How long after "Restart server" the restart comes, so the answer gets out first.
pub const RESTART_DELAY: Duration = Duration::from_millis(500);

/// A restart asked for: [`Restart::schedule`] sets it off after a delay, [`Restart::requested`] waits for it.
/// Clones share it.
#[derive(Clone)]
pub struct Restart(Arc<Inner>);

/// The state behind [`Restart`].
struct Inner {
    /// a restart is scheduled
    pending: AtomicBool,
    /// turns true when the restart is due
    tx: tokio::sync::watch::Sender<bool>,
}

impl Default for Restart {
    fn default() -> Self {
        Restart(Arc::new(Inner { pending: AtomicBool::new(false), tx: tokio::sync::watch::channel(false).0 }))
    }
}

impl Restart {
    /// Set the restart off after `delay`; false when one is already on its way.
    pub fn schedule(&self, delay: Duration) -> bool {
        if self.0.pending.swap(true, Ordering::SeqCst) {
            return false;
        }
        let me = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            me.0.tx.send_replace(true);
        });
        true
    }

    /// Whether the restart is due now.
    pub fn is_requested(&self) -> bool {
        *self.0.tx.borrow()
    }

    /// Wait until the restart is due.
    pub async fn requested(&self) {
        let mut rx = self.0.tx.subscribe();
        let _ = rx.wait_for(|r| *r).await;
    }
}
