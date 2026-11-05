//! [`Mailer`]: how the confirmation mail of an open sign-up is sent. The kit sends no mail itself and needs no
//! mail service: [`LogMailer`], the default, writes each mail to the server's log, so an admin can still hand on a
//! link; an app with mail implements [`Mailer`] on its own SMTP (or anything else).

use kit_server_core::async_trait;

/// Sends a plain text mail.
#[async_trait]
pub trait Mailer: Send + Sync {
    /// Send `text` to `to` with `subject`; the error in plain words when it fails.
    async fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String>;

    /// Whether mail really leaves the server. The admin's sign-up page says when it doesn't, before email
    /// confirmation is switched on. Default: yes.
    fn delivers(&self) -> bool {
        true
    }
}

/// The default [`Mailer`]: writes each mail to the log (`tracing`, level info) and sends nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct LogMailer;

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        tracing::info!("mail (not sent: no mailer is set up) to {to}: {subject}\n{text}");
        Ok(())
    }

    fn delivers(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_log_mailer_sends_nothing_and_says_so() {
        assert!(LogMailer.send("a@example.com", "Hi", "text").await.is_ok());
        assert!(!LogMailer.delivers());
    }
}
