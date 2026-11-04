//! Who is asking: the app tells the kit crates through [`Host`], and the crates' handlers take [`SignedIn`]
//! or [`Admin`], which answer 401 / 403 with the app's own words when the request may not go on.

use crate::{err, Result, Row, RowExt};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use std::future::Future;

/// A signed-in account, as the app tells the kit: only what the kit's routes need. The app keeps its accounts
/// wherever it likes; the kit never reads its tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// the account's id, the key of everything the kit keeps per account
    pub id: i64,
    /// the account's name, shown to admins
    pub username: String,
    /// whether the account may use admin routes
    pub admin: bool,
    /// whether the account is the server's owner (an admin who may also see saved secrets)
    pub owner: bool,
}

impl Account {
    /// An account that is neither an admin nor the owner.
    pub fn member(id: i64, username: impl Into<String>) -> Account {
        Account { id, username: username.into(), admin: false, owner: false }
    }

    /// An account from a row with `id`, `username` and `role`: `admin` may use admin routes, `owner` may also
    /// see saved secrets. For an app that keeps roles that way; any other app fills the fields itself.
    pub fn from_row(row: &Row) -> Account {
        let role = row.s("role");
        Account { id: row.i("id"), username: row.s("username"), admin: matches!(role.as_str(), "admin" | "owner"), owner: role == "owner" }
    }
}

/// The app state the kit crates are given. An app implements it once, for its own state type; each crate asks
/// for its own store in a trait of its own (`ChangelogHost::prefs`, `BugReportsHost::reports` …).
pub trait Host: Clone + Send + Sync + 'static {
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
        let a = Account::from_row(&row(json!({ "id": 3, "username": "ana", "role": "admin" })));
        assert!(a.admin && !a.owner && a.id == 3 && a.username == "ana");
        let o = Account::from_row(&row(json!({ "id": 1, "role": "owner" })));
        assert!(o.admin && o.owner);
        assert_eq!(Account::from_row(&row(json!({ "id": 2, "username": "bo", "role": "member" }))), Account::member(2, "bo"));
    }
}
