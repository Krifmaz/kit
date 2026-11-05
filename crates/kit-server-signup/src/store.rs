//! [`SignupStore`]: where the sign-up setting, the invites, who used each invite and the sign-ups still waiting
//! (for their email or an admin) are kept. [`SqliteSignup`](crate::SqliteSignup) or, with feature `postgres`,
//! `PgSignup` keep them in the kit's own tables (`kit_signup_settings`, `kit_invites`,
//! `kit_invite_uses`, `kit_signup_requests`); an app may implement the trait on its own storage.
//!
//! Invite and verification tokens are never stored: only their SHA-256 ([`crate::token::hash`]).

use kit_server_core::{async_trait, Migrations, Result, Row, RowExt};
use serde::Serialize;
use serde_json::Value;

/// An invite as it is kept.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Invite {
    /// its id
    pub id: i64,
    /// the admin's note ("for the team", "Ana's family"); may be empty
    pub note: String,
    /// the role an account made with it gets (one of the app's [`crate::Choice`]s)
    pub role: String,
    /// the presets an account made with it gets (ids of the app's [`crate::Choice`]s)
    pub presets: Vec<String>,
    /// how many accounts it may make: 1 for a one-time link, 0 for no limit
    pub max_uses: i64,
    /// how many accounts it made
    pub uses: i64,
    /// when it stops working (seconds since 1970); 0 for never
    pub expires_at: f64,
    /// when an admin revoked it; 0 while it is not revoked
    pub revoked_at: f64,
    /// the admin who made it
    pub created_by: i64,
    /// that admin's name, as it was then
    pub created_by_name: String,
    /// when it was made
    pub created_at: f64,
}

impl Invite {
    /// Why it can't be used at `now`: `"revoked"`, `"expired"` or `"used_up"`; `None` while it works.
    pub fn unusable(&self, now: f64) -> Option<&'static str> {
        if self.revoked_at > 0.0 {
            Some("revoked")
        } else if self.expires_at > 0.0 && self.expires_at <= now {
            Some("expired")
        } else if self.max_uses > 0 && self.uses >= self.max_uses {
            Some("used_up")
        } else {
            None
        }
    }

    /// Its state at `now` for the admin's list: `"active"`, `"revoked"`, `"expired"` or `"used_up"`.
    pub fn status(&self, now: f64) -> &'static str {
        self.unusable(now).unwrap_or("active")
    }
}

/// A new invite.
#[derive(Debug, Clone, PartialEq)]
pub struct NewInvite {
    /// the SHA-256 of its token
    pub token_hash: String,
    /// the admin's note
    pub note: String,
    /// the role it gives
    pub role: String,
    /// the presets it gives
    pub presets: Vec<String>,
    /// 1 for one-time, 0 for no limit, or any limit
    pub max_uses: i64,
    /// when it stops working; 0 for never
    pub expires_at: f64,
    /// the admin who makes it
    pub created_by: i64,
    /// that admin's name
    pub created_by_name: String,
}

/// One account an invite made.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct InviteUse {
    /// the invite
    pub invite_id: i64,
    /// the account it made (the app's id)
    pub account_id: i64,
    /// the account's name then
    pub username: String,
    /// when
    pub used_at: f64,
}

/// A sign-up whose account waits for its email to be confirmed, for an admin, or for both. The app's account
/// exists from the start, switched off; the kit switches it on when nothing is left to wait for.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SignupRequest {
    /// its id
    pub id: i64,
    /// the account (the app's id)
    pub account_id: i64,
    /// the account's name
    pub username: String,
    /// the mail address given
    pub email: String,
    /// whether the mail address was confirmed (true when no confirmation was asked)
    pub email_verified: bool,
    /// whether an admin must let the account in
    pub needs_approval: bool,
    /// whether an admin let it in
    pub approved: bool,
    /// the address the sign-up came from
    pub address: String,
    /// when it was made
    pub created_at: f64,
    /// when the confirmation link stops working; 0 when none was sent
    #[serde(skip)]
    pub verify_expires_at: f64,
}

impl SignupRequest {
    /// What it waits for: `"verify_email"` first, then `"approval"`; `None` when nothing is left.
    pub fn waiting_for(&self) -> Option<&'static str> {
        if !self.email_verified {
            Some("verify_email")
        } else if self.needs_approval && !self.approved {
            Some("approval")
        } else {
            None
        }
    }
}

/// A new waiting sign-up.
#[derive(Debug, Clone, PartialEq)]
pub struct NewSignupRequest {
    /// the account, already made and switched off
    pub account_id: i64,
    /// its name
    pub username: String,
    /// the mail address given
    pub email: String,
    /// the SHA-256 of the confirmation token; empty when no confirmation is asked
    pub verify_hash: String,
    /// when the confirmation link stops working; 0 when none is sent
    pub verify_expires_at: f64,
    /// whether an admin must let it in
    pub needs_approval: bool,
    /// the address it came from
    pub address: String,
}

/// Where the sign-up data is kept.
#[async_trait]
pub trait SignupStore: Send + Sync {
    /// The saved setting (a JSON object), if one was saved.
    async fn settings(&self) -> Result<Option<Value>>;
    /// Keep `settings` (a JSON object), replacing what was there.
    async fn save_settings(&self, settings: &Value) -> Result<()>;

    /// Keep a new invite; its id.
    async fn create_invite(&self, new: &NewInvite) -> Result<i64>;
    /// Every invite, newest first.
    async fn invites(&self) -> Result<Vec<Invite>>;
    /// Invite `id`.
    async fn invite(&self, id: i64) -> Result<Option<Invite>>;
    /// The invite whose token has the SHA-256 `token_hash`.
    async fn invite_by_token(&self, token_hash: &str) -> Result<Option<Invite>>;
    /// Count one use of invite `id` if it still works at `now` (not revoked, not expired, uses left), in one
    /// step, so two sign-ups never both take the last use; whether it was counted.
    async fn take_invite(&self, id: i64, now: f64) -> Result<bool>;
    /// Give back a use [`SignupStore::take_invite`] counted (the account could not be made).
    async fn give_back_invite(&self, id: i64) -> Result<()>;
    /// Revoke invite `id` at `now` (a revoked one stays revoked); whether there is such an invite.
    async fn revoke_invite(&self, id: i64, now: f64) -> Result<bool>;
    /// Record that an invite made an account.
    async fn record_use(&self, used: &InviteUse) -> Result<()>;
    /// The accounts invite `id` made, oldest first.
    async fn uses(&self, invite_id: i64) -> Result<Vec<InviteUse>>;

    /// Keep a waiting sign-up; its id.
    async fn add_request(&self, new: &NewSignupRequest) -> Result<i64>;
    /// Every waiting sign-up, oldest first.
    async fn requests(&self) -> Result<Vec<SignupRequest>>;
    /// Waiting sign-up `id`.
    async fn request(&self, id: i64) -> Result<Option<SignupRequest>>;
    /// The waiting sign-up of account `account_id`.
    async fn request_for_account(&self, account_id: i64) -> Result<Option<SignupRequest>>;
    /// The waiting sign-up whose confirmation token has the SHA-256 `verify_hash`.
    async fn request_by_token(&self, verify_hash: &str) -> Result<Option<SignupRequest>>;
    /// Mark the mail address of sign-up `id` confirmed (its token stops working).
    async fn verify_request(&self, id: i64) -> Result<()>;
    /// Mark sign-up `id` let in by an admin.
    async fn approve_request(&self, id: i64) -> Result<()>;
    /// Forget sign-up `id` (its account is on, or it was turned away).
    async fn remove_request(&self, id: i64) -> Result<()>;
}

/// The SQLite steps of the sign-up tables.
pub const SQLITE: Migrations = Migrations {
    component: "signup",
    steps: &["CREATE TABLE kit_signup_settings (id INTEGER PRIMARY KEY CHECK (id = 1), settings TEXT NOT NULL, updated_at REAL NOT NULL DEFAULT 0);
        CREATE TABLE kit_invites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            token_hash TEXT NOT NULL UNIQUE,
            note TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL,
            presets TEXT NOT NULL DEFAULT '[]',
            max_uses INTEGER NOT NULL DEFAULT 1,
            uses INTEGER NOT NULL DEFAULT 0,
            expires_at REAL NOT NULL DEFAULT 0,
            revoked_at REAL NOT NULL DEFAULT 0,
            created_by INTEGER NOT NULL,
            created_by_name TEXT NOT NULL DEFAULT '',
            created_at REAL NOT NULL
        );
        CREATE TABLE kit_invite_uses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            invite_id INTEGER NOT NULL,
            account_id INTEGER NOT NULL,
            username TEXT NOT NULL,
            used_at REAL NOT NULL
        );
        CREATE INDEX kit_invite_uses_invite ON kit_invite_uses (invite_id);
        CREATE TABLE kit_signup_requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id INTEGER NOT NULL UNIQUE,
            username TEXT NOT NULL,
            email TEXT NOT NULL DEFAULT '',
            verify_hash TEXT NOT NULL DEFAULT '',
            verify_expires_at REAL NOT NULL DEFAULT 0,
            email_verified INTEGER NOT NULL DEFAULT 1,
            needs_approval INTEGER NOT NULL DEFAULT 0,
            approved INTEGER NOT NULL DEFAULT 0,
            address TEXT NOT NULL DEFAULT '',
            created_at REAL NOT NULL
        );
        CREATE INDEX kit_signup_requests_token ON kit_signup_requests (verify_hash);"],
};

/// The Postgres steps of the sign-up tables.
pub const POSTGRES: Migrations = Migrations {
    component: "signup",
    steps: &["CREATE TABLE kit_signup_settings (id BIGINT PRIMARY KEY CHECK (id = 1), settings TEXT NOT NULL, updated_at DOUBLE PRECISION NOT NULL DEFAULT 0);
        CREATE TABLE kit_invites (
            id BIGSERIAL PRIMARY KEY,
            token_hash TEXT NOT NULL UNIQUE,
            note TEXT NOT NULL DEFAULT '',
            role TEXT NOT NULL,
            presets TEXT NOT NULL DEFAULT '[]',
            max_uses BIGINT NOT NULL DEFAULT 1,
            uses BIGINT NOT NULL DEFAULT 0,
            expires_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            revoked_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            created_by BIGINT NOT NULL,
            created_by_name TEXT NOT NULL DEFAULT '',
            created_at DOUBLE PRECISION NOT NULL
        );
        CREATE TABLE kit_invite_uses (
            id BIGSERIAL PRIMARY KEY,
            invite_id BIGINT NOT NULL,
            account_id BIGINT NOT NULL,
            username TEXT NOT NULL,
            used_at DOUBLE PRECISION NOT NULL
        );
        CREATE INDEX kit_invite_uses_invite ON kit_invite_uses (invite_id);
        CREATE TABLE kit_signup_requests (
            id BIGSERIAL PRIMARY KEY,
            account_id BIGINT NOT NULL UNIQUE,
            username TEXT NOT NULL,
            email TEXT NOT NULL DEFAULT '',
            verify_hash TEXT NOT NULL DEFAULT '',
            verify_expires_at DOUBLE PRECISION NOT NULL DEFAULT 0,
            email_verified BOOLEAN NOT NULL DEFAULT TRUE,
            needs_approval BOOLEAN NOT NULL DEFAULT FALSE,
            approved BOOLEAN NOT NULL DEFAULT FALSE,
            address TEXT NOT NULL DEFAULT '',
            created_at DOUBLE PRECISION NOT NULL
        );
        CREATE INDEX kit_signup_requests_token ON kit_signup_requests (verify_hash);"],
};

/// The columns of an invite row, for both databases' `SELECT`s.
pub(crate) const INVITE_COLS: &str =
    "id, note, role, presets, max_uses, uses, expires_at, revoked_at, created_by, created_by_name, created_at";

/// The columns of a waiting sign-up row.
pub(crate) const REQUEST_COLS: &str =
    "id, account_id, username, email, email_verified, needs_approval, approved, address, created_at, verify_expires_at";

/// A settings row's JSON object; `None` for no row or text that is not an object.
pub(crate) fn settings_of(row: Option<Row>) -> Option<Value> {
    let v: Value = serde_json::from_str(&row?.s("settings")).ok()?;
    v.is_object().then_some(v)
}

/// An invite from its row.
pub(crate) fn invite_of(r: &Row) -> Invite {
    Invite {
        id: r.i("id"),
        note: r.s("note"),
        role: r.s("role"),
        presets: serde_json::from_str(&r.s("presets")).unwrap_or_default(),
        max_uses: r.i("max_uses"),
        uses: r.i("uses"),
        expires_at: r.f("expires_at"),
        revoked_at: r.f("revoked_at"),
        created_by: r.i("created_by"),
        created_by_name: r.s("created_by_name"),
        created_at: r.f("created_at"),
    }
}

/// A use from its row.
pub(crate) fn use_of(r: &Row) -> InviteUse {
    InviteUse { invite_id: r.i("invite_id"), account_id: r.i("account_id"), username: r.s("username"), used_at: r.f("used_at") }
}

/// A waiting sign-up from its row.
pub(crate) fn request_of(r: &Row) -> SignupRequest {
    SignupRequest {
        id: r.i("id"),
        account_id: r.i("account_id"),
        username: r.s("username"),
        email: r.s("email"),
        email_verified: r.b("email_verified"),
        needs_approval: r.b("needs_approval"),
        approved: r.b("approved"),
        address: r.s("address"),
        created_at: r.f("created_at"),
        verify_expires_at: r.f("verify_expires_at"),
    }
}

/// What every [`SignupStore`] must do, for an implementation's tests (the kit runs it on SQLite and Postgres).
#[cfg(any(test, feature = "testing"))]
pub async fn signup_contract(store: &dyn SignupStore) {
    use serde_json::json;
    // the setting
    assert_eq!(store.settings().await.unwrap(), None);
    store.save_settings(&json!({ "who": "open" })).await.unwrap();
    store.save_settings(&json!({ "who": "closed", "per_hour": 3 })).await.unwrap();
    assert_eq!(store.settings().await.unwrap(), Some(json!({ "who": "closed", "per_hour": 3 })));

    // invites
    let new = |hash: &str, max_uses: i64, expires_at: f64| NewInvite {
        token_hash: hash.into(),
        note: "for the team".into(),
        role: "member".into(),
        presets: vec!["read".into(), "write".into()],
        max_uses,
        expires_at,
        created_by: 1,
        created_by_name: "admin".into(),
    };
    let once = store.create_invite(&new("h1", 1, 0.0)).await.unwrap();
    let many = store.create_invite(&new("h2", 0, 0.0)).await.unwrap();
    let old = store.create_invite(&new("h3", 5, 100.0)).await.unwrap();
    let list = store.invites().await.unwrap();
    assert_eq!(list.iter().map(|i| i.id).collect::<Vec<_>>(), vec![old, many, once], "newest first");
    let i = store.invite(once).await.unwrap().unwrap();
    assert_eq!(
        (i.note.as_str(), i.role.as_str(), i.presets.clone(), i.max_uses, i.uses),
        ("for the team", "member", vec!["read".to_string(), "write".to_string()], 1, 0)
    );
    assert_eq!(i.created_by_name, "admin");
    assert_eq!(store.invite_by_token("h2").await.unwrap().map(|i| i.id), Some(many));
    assert!(store.invite_by_token("nope").await.unwrap().is_none());
    assert!(store.invite(999).await.unwrap().is_none());

    // uses: one-time is taken once; given back, it can be taken again
    assert!(store.take_invite(once, 1000.0).await.unwrap());
    assert!(!store.take_invite(once, 1000.0).await.unwrap());
    store.give_back_invite(once).await.unwrap();
    assert!(store.take_invite(once, 1000.0).await.unwrap());
    assert_eq!(store.invite(once).await.unwrap().unwrap().unusable(1000.0), Some("used_up"));
    for _ in 0..3 {
        assert!(store.take_invite(many, 1000.0).await.unwrap());
    }
    assert_eq!(store.invite(many).await.unwrap().unwrap().uses, 3);
    assert!(!store.take_invite(old, 1000.0).await.unwrap(), "expired");
    assert!(store.take_invite(old, 50.0).await.unwrap());

    // revoking
    assert!(store.revoke_invite(many, 2000.0).await.unwrap());
    assert!(store.revoke_invite(many, 3000.0).await.unwrap());
    assert_eq!(store.invite(many).await.unwrap().unwrap().revoked_at, 2000.0, "revoked once");
    assert!(!store.take_invite(many, 2500.0).await.unwrap());
    assert!(!store.revoke_invite(999, 1.0).await.unwrap());

    // who used each
    store.record_use(&InviteUse { invite_id: many, account_id: 7, username: "ana".into(), used_at: 10.0 }).await.unwrap();
    store.record_use(&InviteUse { invite_id: many, account_id: 8, username: "bo".into(), used_at: 11.0 }).await.unwrap();
    store.record_use(&InviteUse { invite_id: once, account_id: 9, username: "cy".into(), used_at: 12.0 }).await.unwrap();
    let uses = store.uses(many).await.unwrap();
    assert_eq!(uses.iter().map(|u| u.username.as_str()).collect::<Vec<_>>(), vec!["ana", "bo"]);
    assert_eq!(uses[0], InviteUse { invite_id: many, account_id: 7, username: "ana".into(), used_at: 10.0 });
    assert!(store.uses(old).await.unwrap().is_empty());

    // waiting sign-ups
    let a = store
        .add_request(&NewSignupRequest {
            account_id: 20,
            username: "dee".into(),
            email: "dee@example.com".into(),
            verify_hash: "v1".into(),
            verify_expires_at: 500.0,
            needs_approval: true,
            address: "10.0.0.1".into(),
        })
        .await
        .unwrap();
    let b = store
        .add_request(&NewSignupRequest {
            account_id: 21,
            username: "eve".into(),
            email: String::new(),
            verify_hash: String::new(),
            verify_expires_at: 0.0,
            needs_approval: true,
            address: "10.0.0.2".into(),
        })
        .await
        .unwrap();
    let all = store.requests().await.unwrap();
    assert_eq!(all.iter().map(|r| r.id).collect::<Vec<_>>(), vec![a, b], "oldest first");
    let ra = store.request(a).await.unwrap().unwrap();
    assert_eq!((ra.email_verified, ra.needs_approval, ra.approved, ra.waiting_for()), (false, true, false, Some("verify_email")));
    assert_eq!(ra.verify_expires_at, 500.0);
    assert_eq!(store.request(b).await.unwrap().unwrap().waiting_for(), Some("approval"), "no mail asked: verified from the start");
    assert_eq!(store.request_for_account(21).await.unwrap().map(|r| r.id), Some(b));
    assert_eq!(store.request_by_token("v1").await.unwrap().map(|r| r.id), Some(a));
    assert!(store.request_by_token("").await.unwrap().is_none(), "an empty token matches nothing");
    store.verify_request(a).await.unwrap();
    assert!(store.request_by_token("v1").await.unwrap().is_none(), "a used token stops working");
    assert_eq!(store.request(a).await.unwrap().unwrap().waiting_for(), Some("approval"));
    store.approve_request(a).await.unwrap();
    assert_eq!(store.request(a).await.unwrap().unwrap().waiting_for(), None);
    store.remove_request(a).await.unwrap();
    assert!(store.request(a).await.unwrap().is_none());
    assert_eq!(store.requests().await.unwrap().len(), 1);
}
