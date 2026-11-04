//! Who is asking: the app tells the kit crates through [`Host`], and the crates' handlers take [`SignedIn`]
//! or [`Admin`], which answer 401 / 403 with the app's own words when the request may not go on.

use crate::{err, Db, Result, Row, RowExt};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::future::Future;

/// A signed-in account, as the app knows it.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    /// the account's id (`users.id`)
    pub id: i64,
    /// the account's name, shown to admins
    pub username: String,
    /// whether the account may use admin routes
    pub admin: bool,
    /// the account's whole row (the app's `users` row without the password hash): `prefs` and the like
    pub row: Row,
}

impl Account {
    /// An account from an app's `users` row: `id`, `username` and `role` (`admin` or `owner` may use admin
    /// routes).
    pub fn from_row(row: Row) -> Account {
        let admin = matches!(row.s("role").as_str(), "admin" | "owner");
        Account { id: row.i("id"), username: row.s("username"), admin, row }
    }
}

/// The app state the kit crates are given. An app implements it once, for its own state type.
pub trait Host: Clone + Send + Sync + 'static {
    /// The app's database.
    fn db(&self) -> &dyn Db;

    /// The account that sent this request; the app's own error (401 "Not authenticated", 403 for a missing
    /// CSRF header …) when there is none or the request may not go on.
    fn signed_in(&self, parts: &mut Parts) -> impl Future<Output = Result<Account>> + Send;

    /// The signed-in admin; 403 "Admin only" for any other account.
    fn admin(&self, parts: &mut Parts) -> impl Future<Output = Result<Account>> + Send {
        async move {
            let a = self.signed_in(parts).await?;
            if !a.admin {
                return Err(err(403, "Admin only"));
            }
            Ok(a)
        }
    }
}

/// A signed-in account; the app's 401 otherwise.
#[derive(Debug, Clone)]
pub struct SignedIn(pub Account);

/// A signed-in admin; the app's 401 / 403 otherwise.
#[derive(Debug, Clone)]
pub struct Admin(pub Account);

impl<S: Host> FromRequestParts<S> for SignedIn {
    type Rejection = crate::Error;
    async fn from_request_parts(parts: &mut Parts, st: &S) -> Result<Self> {
        st.signed_in(parts).await.map(SignedIn)
    }
}

impl<S: Host> FromRequestParts<S> for Admin {
    type Rejection = crate::Error;
    async fn from_request_parts(parts: &mut Parts, st: &S) -> Result<Self> {
        st.admin(parts).await.map(Admin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accounts_from_rows() {
        let row = |v: serde_json::Value| v.as_object().unwrap().clone();
        let a = Account::from_row(row(json!({ "id": 3, "username": "ana", "role": "admin" })));
        assert!(a.admin && a.id == 3 && a.username == "ana");
        assert!(Account::from_row(row(json!({ "id": 1, "role": "owner" }))).admin);
        assert!(!Account::from_row(row(json!({ "id": 2, "role": "member" }))).admin);
    }
}
