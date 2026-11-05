//! [`PgSignup`]: the sign-up data in Postgres (feature `postgres`).

use crate::store::{
    invite_of, request_of, settings_of, use_of, Invite, InviteUse, NewInvite, NewSignupRequest, SignupRequest, SignupStore, INVITE_COLS,
    POSTGRES, REQUEST_COLS,
};
use kit_server_core::pg::PgDb;
use kit_server_core::{async_trait, db::now, p, Result};
use serde_json::Value;

/// The sign-up data in Postgres (feature `postgres`), in the kit's `kit_signup_settings`, `kit_invites`, `kit_invite_uses` and
/// `kit_signup_requests`.
#[derive(Clone, Debug)]
pub struct PgSignup {
    /// the database
    db: PgDb,
}

impl PgSignup {
    /// The sign-up data on `db`, its tables made or brought up to date.
    pub async fn new(db: PgDb) -> Result<PgSignup> {
        db.migrate(&POSTGRES).await?;
        Ok(PgSignup { db })
    }
}

#[async_trait]
impl SignupStore for PgSignup {
    async fn settings(&self) -> Result<Option<Value>> {
        Ok(settings_of(self.db.one("SELECT settings FROM kit_signup_settings WHERE id = 1", p![]).await?))
    }

    async fn save_settings(&self, settings: &Value) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO kit_signup_settings (id, settings, updated_at) VALUES (1, $1, $2)
             ON CONFLICT (id) DO UPDATE SET settings = EXCLUDED.settings, updated_at = EXCLUDED.updated_at",
                p![settings.to_string(), now()],
            )
            .await?;
        Ok(())
    }

    async fn create_invite(&self, n: &NewInvite) -> Result<i64> {
        self.db
            .one(
                "INSERT INTO kit_invites (token_hash, note, role, presets, max_uses, expires_at, created_by, created_by_name, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
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
            .await?
            .map(|r| kit_server_core::RowExt::i(&r, "id"))
            .ok_or_else(|| kit_server_core::Error::database("no id returned"))
    }

    async fn invites(&self) -> Result<Vec<Invite>> {
        Ok(self.db.query(&format!("SELECT {INVITE_COLS} FROM kit_invites ORDER BY id DESC"), p![]).await?.iter().map(invite_of).collect())
    }

    async fn invite(&self, id: i64) -> Result<Option<Invite>> {
        Ok(self.db.one(&format!("SELECT {INVITE_COLS} FROM kit_invites WHERE id = $1"), p![id]).await?.as_ref().map(invite_of))
    }

    async fn invite_by_token(&self, token_hash: &str) -> Result<Option<Invite>> {
        Ok(self
            .db
            .one(&format!("SELECT {INVITE_COLS} FROM kit_invites WHERE token_hash = $1"), p![token_hash])
            .await?
            .as_ref()
            .map(invite_of))
    }

    async fn take_invite(&self, id: i64, at: f64) -> Result<bool> {
        let r = self
            .db
            .one(
                "UPDATE kit_invites SET uses = uses + 1
             WHERE id = $1 AND revoked_at = 0 AND (expires_at = 0 OR expires_at > $2) AND (max_uses = 0 OR uses < max_uses)
             RETURNING id",
                p![id, at],
            )
            .await?;
        Ok(r.is_some())
    }

    async fn give_back_invite(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_invites SET uses = uses - 1 WHERE id = $1 AND uses > 0", p![id]).await?;
        Ok(())
    }

    async fn revoke_invite(&self, id: i64, at: f64) -> Result<bool> {
        self.db.execute("UPDATE kit_invites SET revoked_at = $1 WHERE id = $2 AND revoked_at = 0", p![at, id]).await?;
        Ok(self.db.one("SELECT id FROM kit_invites WHERE id = $1", p![id]).await?.is_some())
    }

    async fn record_use(&self, u: &InviteUse) -> Result<()> {
        self.db
            .execute(
                "INSERT INTO kit_invite_uses (invite_id, account_id, username, used_at) VALUES ($1, $2, $3, $4)",
                p![u.invite_id, u.account_id, u.username, u.used_at],
            )
            .await?;
        Ok(())
    }

    async fn uses(&self, invite_id: i64) -> Result<Vec<InviteUse>> {
        let rows = self
            .db
            .query("SELECT invite_id, account_id, username, used_at FROM kit_invite_uses WHERE invite_id = $1 ORDER BY id", p![invite_id])
            .await?;
        Ok(rows.iter().map(use_of).collect())
    }

    async fn add_request(&self, n: &NewSignupRequest) -> Result<i64> {
        self.db
            .one(
            "INSERT INTO kit_signup_requests (account_id, username, email, verify_hash, verify_expires_at, email_verified, needs_approval, address, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            p![n.account_id, n.username, n.email, n.verify_hash, n.verify_expires_at, n.verify_hash.is_empty(), n.needs_approval, n.address, now()],
        )
        .await?
        .map(|r| kit_server_core::RowExt::i(&r, "id"))
        .ok_or_else(|| kit_server_core::Error::database("no id returned"))
    }

    async fn requests(&self) -> Result<Vec<SignupRequest>> {
        Ok(self
            .db
            .query(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests ORDER BY id"), p![])
            .await?
            .iter()
            .map(request_of)
            .collect())
    }

    async fn request(&self, id: i64) -> Result<Option<SignupRequest>> {
        Ok(self.db.one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE id = $1"), p![id]).await?.as_ref().map(request_of))
    }

    async fn request_for_account(&self, account_id: i64) -> Result<Option<SignupRequest>> {
        let r = self.db.one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE account_id = $1"), p![account_id]).await?;
        Ok(r.as_ref().map(request_of))
    }

    async fn request_by_token(&self, verify_hash: &str) -> Result<Option<SignupRequest>> {
        let r = self
            .db
            .one(&format!("SELECT {REQUEST_COLS} FROM kit_signup_requests WHERE verify_hash = $1 AND verify_hash <> ''"), p![verify_hash])
            .await?;
        Ok(r.as_ref().map(request_of))
    }

    async fn verify_request(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_signup_requests SET email_verified = TRUE, verify_hash = '' WHERE id = $1", p![id]).await?;
        Ok(())
    }

    async fn approve_request(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE kit_signup_requests SET approved = TRUE WHERE id = $1", p![id]).await?;
        Ok(())
    }

    async fn remove_request(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM kit_signup_requests WHERE id = $1", p![id]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn postgres_keeps_the_contract() {
        crate::store::signup_contract(&PgSignup::new(kit_server_core::pg::test_db().await).await.unwrap()).await;
    }
}
