//! First-run setup on the desktop: the "Create your account" page a new server shows instead of the sign-in page
//! while it has no owner ([`Setup`]).
//!
//! Like every kit-iced part, it holds no network code. Its `update` keeps the state and answers what the app must
//! do ([`Out`]): send a request ([`Request`]) and hand its answer back as [`SetupMsg::Answer`], or sign in with
//! the new account ([`Out::SignIn`]). The server is kit-server-setup (`contracts/setup.md`); the decisions and
//! words are [`kit_rules::setup`].
//!
//! An app asks [`Setup::check`] after connecting (before it shows the sign-in page) and shows this page when the
//! answer, read with [`kit_rules::setup::needed`], says so.

use crate::settings;
use crate::theme::Btn;
use crate::widgets::{self, btn, field, small_muted, El};
use iced::widget::column;
use iced::Length;
use kit_rules::setup::{body, check_form, SetupWords};
use serde_json::Value;

/// The route the requests use (the server's default).
pub const PATH: &str = "/api/setup";

/// A request for the app to send; its answer comes back as [`SetupMsg::Answer`].
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    /// `GET` or `POST`
    pub method: &'static str,
    /// the path from the server's root (`/api/setup`)
    pub path: String,
    /// the JSON body (`null` for a `GET`)
    pub body: Value,
}

/// What the app does after an `update`.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// nothing
    None,
    /// send this request
    Send(Request),
    /// the account is made: sign in with this username and password, as the sign-in page does
    SignIn {
        /// the new account's name
        username: String,
        /// its password
        password: String,
    },
}

/// The words.
fn words() -> SetupWords {
    SetupWords::default()
}

/// The setup page's state.
#[derive(Clone, Debug, Default)]
pub struct Setup {
    /// the setup code, as typed or pasted
    pub code: String,
    /// the username
    pub username: String,
    /// the email
    pub email: String,
    /// the password
    pub password: String,
    /// the password again
    pub again: String,
    /// the form's error, or the server's
    pub error: String,
    /// the account is being made
    pub busy: bool,
}

/// The setup page's messages.
#[derive(Clone, Debug, PartialEq)]
pub enum SetupMsg {
    /// the setup code
    Code(String),
    /// the username
    Username(String),
    /// the email
    Email(String),
    /// the password
    Password(String),
    /// the password again
    Again(String),
    /// Create account
    Submit,
    /// the answer to `POST /api/setup`
    Answer(Result<Value, String>),
}

impl Setup {
    /// The request that says whether a server wants this page (`GET /api/setup`); read its answer with
    /// [`kit_rules::setup::needed`].
    pub fn check() -> Request {
        Request { method: "GET", path: PATH.into(), body: Value::Null }
    }

    /// Applies a message.
    pub fn update(&mut self, msg: SetupMsg) -> Out {
        match msg {
            SetupMsg::Code(s) => self.code = s,
            SetupMsg::Username(s) => self.username = s,
            SetupMsg::Email(s) => self.email = s,
            SetupMsg::Password(s) => self.password = s,
            SetupMsg::Again(s) => self.again = s,
            SetupMsg::Submit => {
                if self.busy {
                    return Out::None;
                }
                if let Err(e) = check_form(&self.code, &self.username, &self.password, &self.again, &words()) {
                    self.error = e;
                    return Out::None;
                }
                self.error.clear();
                self.busy = true;
                return Out::Send(Request {
                    method: "POST",
                    path: PATH.into(),
                    body: body(&self.code, &self.username, &self.password, &self.email),
                });
            }
            SetupMsg::Answer(r) => {
                self.busy = false;
                match r {
                    Ok(_) => {
                        let out = Out::SignIn { username: self.username.trim().to_string(), password: std::mem::take(&mut self.password) };
                        self.again.clear();
                        self.code.clear();
                        return out;
                    }
                    Err(e) => self.error = e,
                }
            }
        }
        Out::None
    }

    /// The page's body (the app puts it in its sign-in frame, under its name).
    pub fn view<'a, M: Clone + 'a>(&'a self, on: impl Fn(SetupMsg) -> M + Clone + 'a) -> El<'a, M> {
        let w = words();
        let (o1, o2, o3, o4, o5, o6) = (on.clone(), on.clone(), on.clone(), on.clone(), on.clone(), on.clone());
        column![
            widgets::h2(w.title),
            small_muted(w.intro),
            widgets::error_box(self.error.as_str()),
            field(w.code, settings::input("ABCD-EFGH-JKMN", &self.code, move |s| o1(SetupMsg::Code(s)))),
            small_muted(w.code_hint),
            field(w.username, settings::input("", &self.username, move |s| o2(SetupMsg::Username(s)))),
            field(format!("{} ({})", w.email, w.email_optional), settings::input("", &self.email, move |s| o3(SetupMsg::Email(s)))),
            field(w.password, settings::input("", &self.password, move |s| o4(SetupMsg::Password(s))).secure(true)),
            field(w.again, settings::input("", &self.again, move |s| o5(SetupMsg::Again(s))).secure(true).on_submit(o6(SetupMsg::Submit)),),
            btn(if self.busy { w.creating } else { w.create }, Btn::Primary, (!self.busy).then(|| on(SetupMsg::Submit)))
                .width(Length::Fill),
        ]
        .spacing(12)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_form_is_checked_sent_and_then_signs_in() {
        assert_eq!(Setup::check(), Request { method: "GET", path: "/api/setup".into(), body: Value::Null });
        let mut s = Setup::default();
        assert_eq!(s.update(SetupMsg::Submit), Out::None);
        assert_eq!(s.error, words().need_code);
        s.update(SetupMsg::Code("abcd-efgh-jkmn".into()));
        s.update(SetupMsg::Username(" ana ".into()));
        s.update(SetupMsg::Password("correct horse".into()));
        s.update(SetupMsg::Again("correct horsE".into()));
        assert_eq!(s.update(SetupMsg::Submit), Out::None);
        assert_eq!(s.error, words().mismatch);
        s.update(SetupMsg::Again("correct horse".into()));
        let Out::Send(r) = s.update(SetupMsg::Submit) else { panic!("sends") };
        assert_eq!((r.method, r.path.as_str()), ("POST", "/api/setup"));
        assert_eq!(r.body, json!({ "code": "abcd-efgh-jkmn", "username": "ana", "password": "correct horse", "email": "" }));
        assert!(s.busy);
        assert_eq!(s.update(SetupMsg::Submit), Out::None, "one at a time");
        // the server says no: its words, and the form stays
        s.update(SetupMsg::Answer(Err("That setup code isn't right.".into())));
        assert_eq!((s.error.as_str(), s.busy), ("That setup code isn't right.", false));
        s.update(SetupMsg::Submit);
        let out = s.update(SetupMsg::Answer(Ok(json!({ "ok": true, "username": "ana" }))));
        assert_eq!(out, Out::SignIn { username: "ana".into(), password: "correct horse".into() });
        assert!(s.password.is_empty() && s.code.is_empty());
        let _ = s.view(|m| m);
    }
}
