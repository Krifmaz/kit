//! Who may make an account, and how.
//!
//! An admin picks **who can sign up** ([`Who`]):
//!
//! - **Closed**: only admins make accounts (in the app's own account pages).
//! - **Invite only** (the default): an admin makes invite links, one-time or multi-use, with an expiry, a most
//!   uses, and the role and presets the new account gets; revokes them at any time; and sees who used each.
//! - **Open**: anyone may sign up, held back by limits per address and per day, with an optional email
//!   confirmation and an optional admin approval. An invite still works and skips both.
//!
//! The app keeps its accounts: the kit never reads or writes its tables. It asks the app to make an account
//! ([`SignupHost::create_account`]), switched off while it waits for its email or an admin, and to switch it on
//! ([`SignupHost::enable_account`]) or remove it ([`SignupHost::remove_account`]) once that is settled. The roles
//! and presets an invite may give are the app's ([`SignupHost::roles`], [`SignupHost::presets`]): "presets" are
//! whatever an app hands a new account (folders it may see, what it may do …), named by id.
//!
//! When apps share accounts (single sign-on), one app is the account source: its setting applies to every app,
//! and the others show it read-only and send people there to sign up ([`SignupHost::account_source`]). With
//! kit-server-suite (feature `suite`), [`suite_account_source`] gives that from the suite's account-source link.
//!
//! ```text
//! GET  /api/signup                       → {who, verify_email, approval, managed_by}   (anyone)
//! POST /api/signup                       {username, password, email?, invite?} → {status, message, account_id}
//! POST /api/signup/invite                {token} → {ok, role, presets, expires_at}       (anyone: checks a link)
//! POST /api/signup/verify                {token} → {status, message}                     (anyone: the mail's link)
//! GET  /api/signup/settings              (admin) → {settings, roles, presets, mail_delivers, managed_by, read_only}
//! PUT  /api/signup/settings              (admin) {who, open_role, open_presets, verify_email, approval, per_hour, per_day}
//! GET  /api/signup/invites               (admin) → {invites: [{…, status, used_by: […]}]}
//! POST /api/signup/invites               (admin) {note, role, presets, max_uses, expires_in_hours} → {invite, token, link}
//! GET  /api/signup/invites/{id}          (admin) → {invite}
//! POST /api/signup/invites/{id}/revoke   (admin) → {invite}
//! GET  /api/signup/pending               (admin) → {pending: [{id, account_id, username, email, waiting_for, …}]}
//! POST /api/signup/pending/{id}/approve  (admin) → {ok, status}
//! POST /api/signup/pending/{id}/reject   (admin) → {ok}: the waiting account is removed
//! ```
//!
//! `status` is `active` (sign in now), `verify_email` (open the mailed link) or `approval` (wait for an admin).
//!
//! An app adds the routes with `.merge(kit_server_signup::router())` (needs [`SignupHost`]) or
//! `router_with(SignupConfig { … })`. What they keep is behind [`SignupStore`]: the kit has it in SQLite
//! ([`SqliteSignup`], feature `sqlite`, on by default) and Postgres (`PgSignup`, feature `postgres`), in tables of
//! its own (`kit_signup_settings`, `kit_invites`, `kit_invite_uses`, `kit_signup_requests`). Tokens are kept only
//! as their SHA-256; an invite's link is shown once, when it is made.

pub mod mail;
#[cfg(feature = "postgres")]
mod pg;
mod routes;
#[cfg(feature = "sqlite")]
mod sqlite;
pub mod store;
pub mod token;

pub use mail::{LogMailer, Mailer};
#[cfg(feature = "postgres")]
pub use pg::PgSignup;
#[cfg(feature = "sqlite")]
pub use sqlite::SqliteSignup;
pub use store::{Invite, InviteUse, NewInvite, NewSignupRequest, SignupRequest, SignupStore};

use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Extension, Router};
use kit_server_core::{Host, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::future::Future;
use std::sync::Arc;

/// Who can sign up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Who {
    /// only admins make accounts
    Closed,
    /// only people with an invite link (the default)
    #[default]
    Invite,
    /// anyone, within the limits
    Open,
}

/// The admin's sign-up setting. Missing fields take their defaults, so a setting saved by an older kit still reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SignupSettings {
    /// who can sign up (default: invite only)
    pub who: Who,
    /// the role an open sign-up gets; "" for the app's first role
    pub open_role: String,
    /// the presets an open sign-up gets
    pub open_presets: Vec<String>,
    /// whether an open sign-up must confirm its email before it can sign in (default: no)
    pub verify_email: bool,
    /// whether an admin must let an open sign-up in (default: no)
    pub approval: bool,
    /// open sign-ups one address may make in an hour (default 5, at least 1)
    pub per_hour: u32,
    /// open sign-ups in all in a day; 0 for no limit (default 50)
    pub per_day: u32,
}

impl Default for SignupSettings {
    fn default() -> Self {
        SignupSettings {
            who: Who::Invite,
            open_role: String::new(),
            open_presets: Vec::new(),
            verify_email: false,
            approval: false,
            per_hour: 5,
            per_day: 50,
        }
    }
}

/// A role or preset the app offers, for the admin's forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choice {
    /// the id kept on the invite and handed back to [`SignupHost::create_account`]
    pub id: String,
    /// what the admin reads
    pub label: String,
    /// one more sentence, if any
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl Choice {
    /// A choice with no description.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Choice {
        Choice { id: id.into(), label: label.into(), description: String::new() }
    }
}

/// An account to make.
#[derive(Debug, Clone, PartialEq)]
pub struct NewAccount {
    /// the name given (the app checks it)
    pub username: String,
    /// the password given (the app checks and hashes it)
    pub password: String,
    /// the mail address given; may be empty
    pub email: String,
    /// its role (one of [`SignupHost::roles`])
    pub role: String,
    /// its presets (ids of [`SignupHost::presets`])
    pub presets: Vec<String>,
    /// false while it waits for its email or an admin: it must not be able to sign in
    pub enabled: bool,
}

/// The app that keeps the accounts of linked apps (single sign-on), as this app knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountSource {
    /// its name, as people know it
    pub name: String,
    /// its address (`https://accounts.example.com`), where people sign up
    pub url: String,
    /// its `GET /api/signup` answer; `None` when it can't be reached now
    pub policy: Option<Value>,
}

/// The words people see. Each has a plain English default; `{app}`, `{username}`, `{link}` and `{hours}` are
/// filled in where they appear.
#[derive(Debug, Clone, PartialEq)]
pub struct Words {
    /// a sign-up while sign-up is closed
    pub closed: String,
    /// a sign-up without an invite while sign-up is by invite only
    pub invite_only: String,
    /// an invite token that matches nothing
    pub invite_unknown: String,
    /// a revoked invite
    pub invite_revoked: String,
    /// an expired invite
    pub invite_expired: String,
    /// an invite with no uses left
    pub invite_used_up: String,
    /// too many tries or sign-ups from one address
    pub too_many: String,
    /// the day's open sign-ups are all taken
    pub full_today: String,
    /// an open sign-up without an email while confirmation is on
    pub email_needed: String,
    /// an email that doesn't look like one
    pub email_bad: String,
    /// a sign-up or a change on an app whose accounts are kept by another (`{app}`: the source's name)
    pub managed: String,
    /// the confirmation mail's subject
    pub verify_subject: String,
    /// the confirmation mail's text
    pub verify_text: String,
    /// a confirmation link that is wrong, used or too old
    pub verify_bad: String,
    /// the answer while the email waits to be confirmed
    pub waiting_email: String,
    /// the answer while an admin has to look
    pub waiting_approval: String,
    /// the answer when the account is ready
    pub ready: String,
}

impl Default for Words {
    fn default() -> Self {
        Words {
            closed: "Sign-up is closed. Ask an admin to make you an account.".into(),
            invite_only: "Sign-up is by invite only. Ask an admin for an invite link.".into(),
            invite_unknown: "This invite link isn't valid. Check that you copied all of it.".into(),
            invite_revoked: "This invite was revoked. Ask an admin for a new one.".into(),
            invite_expired: "This invite has expired. Ask an admin for a new one.".into(),
            invite_used_up: "This invite has been used up. Ask an admin for a new one.".into(),
            too_many: "Too many sign-ups from this address. Try again in an hour.".into(),
            full_today: "Sign-up is busy today. Try again tomorrow.".into(),
            email_needed: "Enter your email address: we send a link to confirm it.".into(),
            email_bad: "That email address doesn't look right.".into(),
            managed: "Accounts are managed by {app}. Sign up there: the same account works here.".into(),
            verify_subject: "{app}: confirm your email".into(),
            verify_text: "Hello {username},\n\nOpen this link to confirm your email for {app}:\n\n{link}\n\nThe link works for {hours} hours. If you didn't sign up, ignore this mail.\n".into(),
            verify_bad: "This confirmation link isn't valid any more. Sign in to see if your account is ready, or ask an admin.".into(),
            waiting_email: "Check your email: open the link we sent to confirm your address.".into(),
            waiting_approval: "Thanks! An admin will look at your sign-up. You can sign in once it is approved.".into(),
            ready: "Your account is ready. Sign in.".into(),
        }
    }
}

/// Everything about sign-up an app may change: where it is served, its links and its words.
#[derive(Debug, Clone, PartialEq)]
pub struct SignupConfig {
    /// the routes' common start (default `/api/signup`)
    pub base: String,
    /// an invite's link after the app's address, `{token}` filled in (default `/#/sign-up?invite={token}`)
    pub invite_link: String,
    /// the confirmation mail's link after the app's address (default `/#/sign-up/verify?token={token}`)
    pub verify_link: String,
    /// how long a confirmation link works, in hours (default 48)
    pub verify_hours: u32,
    /// tries (sign-ups and invite checks) one address may make in an hour, whatever the setting (default 30)
    pub tries_per_hour: usize,
    /// the words people see
    pub words: Words,
}

impl Default for SignupConfig {
    fn default() -> Self {
        SignupConfig {
            base: "/api/signup".into(),
            invite_link: "/#/sign-up?invite={token}".into(),
            verify_link: "/#/sign-up/verify?token={token}".into(),
            verify_hours: 48,
            tries_per_hour: 30,
            words: Words::default(),
        }
    }
}

/// What sign-up needs from the app besides [`Host`]: where its data is kept, the app's roles and presets, how
/// to make, switch on and remove an account, its rate limiter, and (optionally) a mailer and an account source.
pub trait SignupHost: Host {
    /// Where the setting, invites and waiting sign-ups are kept: a `SqliteSignup` or `PgSignup` the app made once
    /// at start, or the app's own [`SignupStore`].
    fn signup(&self) -> &dyn SignupStore;

    /// The app's name: `{app}` in the confirmation mail.
    fn app_name(&self) -> String;

    /// The roles an invite or an open sign-up may give, the default first. Never offer the owner's role here.
    fn roles(&self) -> Vec<Choice>;

    /// The presets an invite or an open sign-up may give (folders a new account sees, what it may do …).
    /// Default: none.
    fn presets(&self) -> Vec<Choice> {
        Vec::new()
    }

    /// Make an account; its id. The app checks the name and password (and answers 400 / 409 with its own words),
    /// hashes the password, and gives the account its role and presets. An account with `enabled: false` must not
    /// be able to sign in until [`SignupHost::enable_account`].
    fn create_account(&self, account: NewAccount) -> impl Future<Output = Result<i64>> + Send;

    /// Let account `id` sign in: its email is confirmed and an admin let it in, as the setting asked.
    fn enable_account(&self, id: i64) -> impl Future<Output = Result<()>> + Send;

    /// Remove account `id`: an admin turned its sign-up away before it could ever sign in.
    fn remove_account(&self, id: i64) -> impl Future<Output = Result<()>> + Send;

    /// True when `key` already had `limit` events in the last `window_secs` seconds; records one otherwise. The
    /// keys are `signup-try:<address>`, `signup:<address>` and `signup-day`.
    fn limited(&self, key: &str, limit: usize, window_secs: f64) -> bool;

    /// The address a request came from, for the limits. Default: the first `X-Forwarded-For` hop, else "local".
    /// An app that is not behind a proxy it trusts should answer with the socket's address instead.
    fn client_address(&self, headers: &HeaderMap) -> String {
        headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "local".into())
    }

    /// The address the app is reached at (`https://app.example.com`), put before the invite and confirmation
    /// links; "" when unknown (the links are then relative, and each client puts its own server before them).
    /// Default: "".
    fn base_url(&self) -> String {
        String::new()
    }

    /// How the confirmation mail is sent. Default: [`LogMailer`] (written to the log, not sent).
    fn mailer(&self) -> Arc<dyn Mailer> {
        Arc::new(LogMailer)
    }

    /// The app that keeps the accounts when this app is linked to others with single sign-on and is not that app
    /// itself; `None` (the default) when this app keeps its own accounts. When it is set, this app's setting is
    /// read-only, shows the source's, and sign-up and invites send people to the source.
    fn account_source(&self) -> impl Future<Output = Option<AccountSource>> + Send {
        async { None }
    }

    /// A line for the app's activity log (`info` or `warning`): an invite made or revoked, a sign-up, an approval.
    /// The default writes it to the tracing log.
    fn log(&self, level: &str, message: &str) {
        tracing::info!("{level}: {message}");
    }
}

/// The configuration the handlers are given.
pub(crate) type Cfg = Extension<Arc<SignupConfig>>;

/// The sign-up routes with the default paths, links and words.
pub fn router<S: SignupHost>() -> Router<S> {
    router_with(SignupConfig::default())
}

/// The sign-up routes with the app's paths, links and words.
pub fn router_with<S: SignupHost>(config: SignupConfig) -> Router<S> {
    use routes::*;
    let base = config.base.trim_end_matches('/').to_string();
    Router::new()
        .route(&base, get(policy::<S>).post(sign_up::<S>))
        .route(&format!("{base}/invite"), post(check_invite::<S>))
        .route(&format!("{base}/verify"), post(verify::<S>))
        .route(&format!("{base}/settings"), get(get_settings::<S>).put(put_settings::<S>))
        .route(&format!("{base}/invites"), get(list_invites::<S>).post(create_invite::<S>))
        .route(&format!("{base}/invites/{{id}}"), get(get_invite::<S>))
        .route(&format!("{base}/invites/{{id}}/revoke"), post(revoke_invite::<S>))
        .route(&format!("{base}/pending"), get(list_pending::<S>))
        .route(&format!("{base}/pending/{{id}}/approve"), post(approve::<S>))
        .route(&format!("{base}/pending/{{id}}/reject"), post(reject::<S>))
        .layer(Extension(Arc::new(config)))
}

/// The setting in effect: the saved one, else the defaults.
pub async fn settings(st: &impl SignupHost) -> Result<SignupSettings> {
    Ok(st.signup().settings().await?.and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default())
}

/// Why account `account_id` can't sign in yet, in the words of `words` ([`Words::waiting_email`] or
/// [`Words::waiting_approval`]); `None` when its sign-up isn't waiting. An app's sign-in answers a switched-off
/// account with this, so people know what they wait for.
pub async fn waiting(st: &impl SignupHost, account_id: i64, words: &Words) -> Result<Option<String>> {
    let r = st.signup().request_for_account(account_id).await?;
    Ok(r.and_then(|r| match r.waiting_for() {
        Some("verify_email") => Some(words.waiting_email.clone()),
        Some(_) => Some(words.waiting_approval.clone()),
        None => None,
    }))
}

/// `text` with each `{name}` of `values` filled in, in one pass: a value that itself holds `{…}` stays as it is.
pub fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let hit = after.find('}').and_then(|end| values.iter().find(|(k, _)| *k == &after[..end]).map(|(_, v)| (end, *v)));
        match hit {
            Some((end, v)) => {
                out.push_str(v);
                rest = &after[end + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Whether `email` looks like a mail address: one `@`, something before it, a dot in the part after it, no spaces,
/// at most 254 characters. (Only the confirmation mail proves it.)
pub fn looks_like_email(email: &str) -> bool {
    let Some((user, domain)) = email.split_once('@') else { return false };
    email.len() <= 254
        && !user.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// The account source of a [kit-server-suite](kit_server_suite) app, for [`SignupHost::account_source`]: the link
/// marked as the account source, with its sign-up policy read through the link's key (`GET {base}` on that app,
/// `base` being its sign-up routes, `/api/signup` by default). `None` when no link is the account source; a policy
/// of `None` when that app can't be reached now (the reason goes to the log), so this app's page says so.
///
/// ```ignore
/// async fn account_source(&self) -> Option<AccountSource> {
///     kit_server_signup::suite_account_source(&self.suite, &self.links, "/api/signup").await
/// }
/// ```
#[cfg(feature = "suite")]
pub async fn suite_account_source(
    suite: &kit_server_suite::Suite,
    store: &dyn kit_server_suite::SuiteStore,
    base: &str,
) -> Option<AccountSource> {
    let link = match suite.account_source(store).await {
        Ok(l) => l?,
        Err(e) => {
            tracing::warn!("sign-up: the suite's account source could not be read: {}", e.detail);
            return None;
        }
    };
    let policy = match suite.fetch_json(&link, base).await {
        Ok(v) if v.is_object() => Some(v),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!("sign-up: {} could not be asked for its sign-up setting: {}", link.base_url, e.detail);
            None
        }
    };
    Some(AccountSource { name: link.name(), url: link.base_url, policy })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn settings_read_with_defaults() {
        let s: SignupSettings = serde_json::from_value(serde_json::json!({ "who": "open", "approval": true })).unwrap();
        assert_eq!((s.who, s.approval, s.verify_email, s.per_hour, s.per_day), (Who::Open, true, false, 5, 50));
        assert_eq!(SignupSettings::default().who, Who::Invite);
        assert_eq!(serde_json::to_value(Who::Closed).unwrap(), "closed");
    }

    #[test]
    fn emails_and_fill() {
        for ok in ["a@b.co", "first.last+tag@mail.example.org"] {
            assert!(looks_like_email(ok), "{ok}");
        }
        for bad in ["", "a", "a@b", "@b.co", "a@@b.co", "a b@c.de", "a@.co", "a@co."] {
            assert!(!looks_like_email(bad), "{bad}");
        }
        assert_eq!(fill("{app}: {x} {link}", &[("app", "Demo"), ("link", "{app}")]), "Demo: {x} {app}");
    }
}
