//! First-run setup: the "Create your account" screen a new server shows instead of the sign-in page while it has
//! no owner, as every client shows it.
//!
//! The server is kit-server-setup (`contracts/setup.md`). [`needed`] reads `GET /api/setup` (show the screen?);
//! [`check_form`] checks the form before it is sent and [`body`] is what `POST /api/setup` gets. After the server
//! answers, the client signs in with the same name and password. The words are [`SetupWords`].

use crate::signup::MIN_PASSWORD;
use serde_json::{json, Value};

/// Every word the setup screen shows. An app replaces any of them.
#[derive(Debug, Clone, PartialEq)]
pub struct SetupWords {
    /// the screen's title
    pub title: &'static str,
    /// the line under it
    pub intro: &'static str,
    /// the setup code field's label
    pub code: &'static str,
    /// where the code is, under the field
    pub code_hint: &'static str,
    /// the username field's label
    pub username: &'static str,
    /// the password field's label
    pub password: &'static str,
    /// the password check field's label
    pub again: &'static str,
    /// the email field's label
    pub email: &'static str,
    /// the email field's hint
    pub email_optional: &'static str,
    /// no code given
    pub need_code: &'static str,
    /// no name given
    pub need_name: &'static str,
    /// the password is too short (`{n}`)
    pub short_password: &'static str,
    /// the two passwords differ
    pub mismatch: &'static str,
    /// the button
    pub create: &'static str,
    /// the button while it goes
    pub creating: &'static str,
}

impl Default for SetupWords {
    fn default() -> Self {
        SetupWords {
            title: "Create your account",
            intro: "This server has no account yet. Make yours: it is the owner, who can change everything.",
            code: "Setup code",
            code_hint: "The server prints it in its log when it starts (for Docker: docker logs, then the container's name).",
            username: "Username",
            password: "Password",
            again: "Password again",
            email: "Email",
            email_optional: "optional",
            need_code: "Enter the setup code from the server's log.",
            need_name: "Choose a username",
            short_password: "Use a password of at least {n} characters",
            mismatch: "The two passwords aren't the same",
            create: "Create account",
            creating: "Creating…",
        }
    }
}

/// Whether the screen shows, from `GET /api/setup` (`{needed}`); anything else is "no".
pub fn needed(v: &Value) -> bool {
    v.get("needed").and_then(Value::as_bool).unwrap_or(false)
}

/// The form's problem before it is sent, in `w`'s words; `Ok` when it can go.
pub fn check_form(code: &str, username: &str, password: &str, again: &str, w: &SetupWords) -> Result<(), String> {
    if code.trim().is_empty() {
        return Err(w.need_code.to_string());
    }
    if username.trim().is_empty() {
        return Err(w.need_name.to_string());
    }
    if password.chars().count() < MIN_PASSWORD {
        return Err(crate::app::fill(w.short_password, &[("n", &MIN_PASSWORD.to_string())]));
    }
    if password != again {
        return Err(w.mismatch.to_string());
    }
    Ok(())
}

/// The `POST /api/setup` body.
pub fn body(code: &str, username: &str, password: &str, email: &str) -> Value {
    json!({ "code": code.trim(), "username": username.trim(), "password": password, "email": email.trim() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_and_its_form() {
        let w = SetupWords::default();
        assert!(needed(&json!({ "needed": true })));
        assert!(!needed(&json!({ "needed": false })));
        assert!(!needed(&json!({ "detail": "Not found" })), "an older server without setup: sign in as before");
        assert_eq!(check_form(" ", "ana", "password1", "password1", &w), Err(w.need_code.into()));
        assert_eq!(check_form("ABCD", " ", "password1", "password1", &w), Err(w.need_name.into()));
        assert_eq!(check_form("ABCD", "ana", "short", "short", &w), Err("Use a password of at least 8 characters".into()));
        assert_eq!(check_form("ABCD", "ana", "password1", "password2", &w), Err(w.mismatch.into()));
        assert_eq!(check_form("ABCD", "ana", "password1", "password1", &w), Ok(()));
        assert_eq!(
            body(" abcd-efgh ", " ana ", " pw ", " a@b.co "),
            json!({ "code": "abcd-efgh", "username": "ana", "password": " pw ", "email": "a@b.co" })
        );
    }
}
