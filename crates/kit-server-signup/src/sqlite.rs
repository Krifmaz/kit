//! [`SqliteSignup`]: the sign-up data in SQLite.

use crate::store::{
    invite_of, request_of, settings_of, use_of, Invite, InviteUse, NewInvite, NewSignupRequest, SignupRequest, SignupStore, INVITE_COLS,
    REQUEST_COLS, SQLITE,
};
use kit_server_core::{async_trait, db::now, p, Db, Result, SqliteDb};
use serde_json::Value;

/// The sign-up data in SQLite, in the kit's `kit_signup_settings`, `kit_invites`, `kit_invite_uses` and
/// `kit_signup_requests`.
#[derive(Clone)]
pub struct SqliteSignup {
    /// the database
    db: SqliteDb,
}

impl SqliteSignup {
    /// The sign-up data on `db`, its tables made or brought up to date.
    pub fn new(db: SqliteDb) -> Result<SqliteSignup> {
        db.migrate(&SQLITE)?;
        Ok(SqliteSignup { db })
    }
}

#[async_trait]
impl SignupStore for SqliteSignup {
    async fn settings(&self) -> Result<Option<Value>> {
        Ok(settings_of(self.db.one("SELECT settings FROM kit_signup_settings WHERE id = 1", p![])?))
    }

    async fn save_settings(&self, settings: &Value) -> Result<()> {
        self.db.execute(
            "INSERT INTO kit_signup_settings (id, settings, updated_at) VALUES (1, ?, ?)
             ON CONFLICT (id) DO UPDATE SET settings = excluded.settings, updated_at = excluded.updated_at",
            p![settings.to_string(), now()],
        )?;
        Ok(())
    }

    async fn create_invite(&self, n: &NewInvite) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_invites (token_hash, note, role, presets, max_uses, expires_at, created_by, created_by_name, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            p![
                n.token_hash,
                n.note,
                n.role,
                serde_json::to_string(&n.presets).unwrap_or_default(),
                n.max_uses,
                n.expires_at,
                n.created_by,
                n.created_by_name,
                now()
            ],
        )
    }

    async fn invites(&self) -> Result<Vec<Invite>> {
        Ok(self.db.query(&format!("SELECT {INVITE_COLS} FROM kit_invites ORDER BY id DESC"), p![])?.iter().map(invite_of).collect())
    }

    async fn invite(&self, id: i64) -> Result<Option<Invite>> {
        Ok(self.db.one(&format!("SELECT {INVITE_COLS} FROM kit_invites WHERE id = ?"), p![id])?.as_ref().map(invite_of))
    }

    async fn invite_by_token(&self, token_hash: &str) -> Result<Option<Invite>> {
        Ok(self.db.one(&format!("SELECT {INVITE_COLS} FROM kit_invites WHERE token_hash = ?"), p![token_hash])?.as_ref().map(invite_of))
    }

    async fn take_invite(&self, id: i64, at: f64) -> Result<bool> {
        let r = self.db.one(
            "UPDATE kit_invites SET uses = uses + 1
             WHERE id = ? AND revoked_at = 0 AND (expires_at = 0 OR expires_at > ?) AND (max_uses = 0 OR uses < max_uses)
             RETURNING id",
            p![id, at],
        )?;
        Ok(r.is_some())
    }

    async fn give_back_invite(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_invites SET uses = uses - 1 WHERE id = ? AND uses > 0", p![id])?;
        Ok(())
    }

    async fn revoke_invite(&self, id: i64, at: f64) -> Result<bool> {
        self.db.execute("UPDATE kit_invites SET revoked_at = ? WHERE id = ? AND revoked_at = 0", p![at, id])?;
        Ok(self.db.one("SELECT id FROM kit_invites WHERE id = ?", p![id])?.is_some())
    }

    async fn record_use(&self, u: &InviteUse) -> Result<()> {
        self.db.execute(
            "INSERT INTO kit_invite_uses (invite_id, account_id, username, used_at) VALUES (?, ?, ?, ?)",
            p![u.invite_id, u.account_id, u.username, u.used_at],
        )?;
        Ok(())
    }

    async fn uses(&self, invite_id: i64) -> Result<Vec<InviteUse>> {
        let rows = self
            .db
            .query("SELECT invite_id, account_id, username, used_at FROM kit_invite_uses WHERE invite_id = ? ORDER BY id", p![invite_id])?;
        Ok(rows.iter().map(use_of).collect())
    }

    async fn add_request(&self, n: &NewSignupRequest) -> Result<i64> {
        self.db.execute(
            "INSERT INTO kit_signup_requests (account_id, username, email, verify_hash, verify_expires_at, email_verified, needs_approval, address, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            p![n.account_id, n.username, n.email, n.verify_hash, n.verify_expires_at, n.verify_hash.is_empty(), n.needs_approval, n.address, now()],
        )
    }

    async fn requests(&self) -> Result<Vec<SignupRequest>> {
        Ok(self.db.query(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests ORDER BY id"), p![])?.iter().map(request_of).collect())
    }

    async fn request(&self, id: i64) -> Result<Option<SignupRequest>> {
        Ok(self.db.one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE id = ?"), p![id])?.as_ref().map(request_of))
    }

    async fn request_for_account(&self, account_id: i64) -> Result<Option<SignupRequest>> {
        let r = self.db.one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE account_id = ?"), p![account_id])?;
        Ok(r.as_ref().map(request_of))
    }

    async fn request_by_token(&self, verify_hash: &str) -> Result<Option<SignupRequest>> {
        let r = self
            .db
            .one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE verify_hash = ? AND verify_hash <> ''"), p![verify_hash])?;
        Ok(r.as_ref().map(request_of))
    }

    async fn verify_request(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_signup_requests SET email_verified = 1, verify_hash = '' WHERE id = ?", p![id])?;
        Ok(())
    }

    async fn approve_request(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_signup_requests SET approved = 1 WHERE id = ?", p![id])?;
        Ok(())
    }

    async fn remove_request(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM kit_signup_requests WHERE id = ?", p![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_keeps_the_contract() {
        crate::store::signup_contract(&SqliteSignup::new(SqliteDb::memory()).unwrap()).await;
    }
}
