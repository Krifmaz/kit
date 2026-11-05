//! Sign-up on the desktop: the sign-up page ([`SignUp`]: paste an invite link, or open sign-up) and the admin's
//! sign-up page ([`Admin`]: who can sign up, waiting sign-ups, invites with their link shown once).
//!
//! Like every kit-iced part, they hold no network code. Their `update` keeps the state and answers what the app
//! must do ([`Out`]): send a request ([`Request`]) and hand its answer back as `Answer(kind, result)`, copy a
//! text, or show a toast. The server is kit-server-signup (`contracts/signup.md`); the decisions and words are
//! [`kit_rules::signup`].

use crate::confirm::{self, Confirm};
use crate::settings::{self, Opt};
use crate::theme::{Badge, Btn};
use crate::widgets::{self, badge, btn, card, field, h2, h3, modal, seg, small, small_muted, El};
use iced::widget::{column, container, radio, row, text_input};
use iced::{Alignment, Length};
use kit_rules::app::fill;
use kit_rules::signup::{
    check_form, choices, day, full_link, invite_status, label_of, parse_uses, sign_up_body, token_from, Choice, InviteForm, InviteRow,
    ManagedBy, PendingRow, Policy, Settings, SignupWords, EXPIRY, WHO,
};
use serde_json::{json, Value};

/// The routes' base the requests use (the server's default).
pub const BASE: &str = "/api/signup";

/// A request for the app to send; its answer comes back as `Answer(kind, result)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    /// `GET`, `POST` or `PUT`
    pub method: &'static str,
    /// the path from the server's root (`/api/signup/invites`)
    pub path: String,
    /// the JSON body (`null` for a `GET`)
    pub body: Value,
    /// what the answer is
    pub kind: Kind,
}

impl Request {
    fn get(p: &str, kind: Kind) -> Request {
        Request { method: "GET", path: format!("{BASE}{p}"), body: Value::Null, kind }
    }
    fn post(p: &str, body: Value, kind: Kind) -> Request {
        Request { method: "POST", path: format!("{BASE}{p}"), body, kind }
    }
}

/// What an answer is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `GET /api/signup`
    Policy,
    /// `POST /api/signup/invite`
    Invite,
    /// `POST /api/signup`
    SignedUp,
    /// `GET /api/signup/settings`
    Settings,
    /// `GET /api/signup/invites`
    Invites,
    /// `GET /api/signup/pending`
    Pending,
    /// `PUT /api/signup/settings`
    Saved,
    /// `POST /api/signup/invites`
    Created,
    /// a revoke, approve or reject
    Changed,
}

/// What the app does after an `update`.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// nothing
    None,
    /// send these requests
    Send(Vec<Request>),
    /// copy this text (and say [`SignupWords::copied`])
    Copy(String),
    /// show this toast (`true`: an error, which stays with Copy)
    Toast(String, bool),
}

/// The words.
fn words() -> SignupWords {
    SignupWords::default()
}

// ---- the sign-up page ------------------------------------------------------------------------------------

/// The sign-up page's state.
#[derive(Clone, Debug, Default)]
pub struct SignUp {
    /// what the server allows (`None` while it is asked)
    pub policy: Option<Result<Policy, String>>,
    /// the invite field (a pasted link or token)
    pub invite: String,
    /// what a checked invite gives, or why it can't be used
    pub invite_check: Option<Result<String, String>>,
    /// the username
    pub username: String,
    /// the email
    pub email: String,
    /// the password
    pub password: String,
    /// the password again
    pub again: String,
    /// the form's error
    pub error: String,
    /// the server's answer once signed up
    pub done: Option<String>,
    /// a sign-up is on its way
    pub busy: bool,
}

/// The sign-up page's messages.
#[derive(Clone, Debug, PartialEq)]
pub enum SignUpMsg {
    /// the invite field
    Invite(String),
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
    /// a request's answer
    Answer(Kind, Result<Value, String>),
}

impl SignUp {
    /// The page opened (with an invite link, if the app was handed one): its requests.
    pub fn open(&mut self, invite: &str) -> Out {
        *self = SignUp { invite: invite.to_string(), ..SignUp::default() };
        let mut reqs = vec![Request::get("", Kind::Policy)];
        reqs.extend(self.check_invite());
        Out::Send(reqs)
    }

    fn check_invite(&mut self) -> Option<Request> {
        let t = token_from(&self.invite);
        self.invite_check = None;
        (!t.is_empty()).then(|| Request::post("/invite", json!({ "token": t }), Kind::Invite))
    }

    /// Applies a message.
    pub fn update(&mut self, msg: SignUpMsg) -> Out {
        match msg {
            SignUpMsg::Invite(s) => {
                self.invite = s;
                return self.check_invite().map(|r| Out::Send(vec![r])).unwrap_or(Out::None);
            }
            SignUpMsg::Username(s) => self.username = s,
            SignUpMsg::Email(s) => self.email = s,
            SignUpMsg::Password(s) => self.password = s,
            SignUpMsg::Again(s) => self.again = s,
            SignUpMsg::Submit => {
                if self.busy {
                    return Out::None;
                }
                let token = token_from(&self.invite);
                let required = self.policy.as_ref().and_then(|p| p.as_ref().ok()).is_some_and(|p| p.email_required(!token.is_empty()));
                if let Err(e) = check_form(&self.username, &self.password, &self.again, &self.email, required, &words()) {
                    self.error = e;
                    return Out::None;
                }
                self.error.clear();
                self.busy = true;
                return Out::Send(vec![Request::post(
                    "",
                    sign_up_body(&self.username, &self.password, &self.email, &token),
                    Kind::SignedUp,
                )]);
            }
            SignUpMsg::Answer(kind, r) => match kind {
                Kind::Policy => self.policy = Some(r.map(|v| Policy::from_json(&v))),
                Kind::Invite => {
                    self.invite_check = Some(r.map(|v| {
                        let role = v.get("role").and_then(Value::as_str).unwrap_or("").to_string();
                        fill(words().invite_gives, &[("role", &role)])
                    }))
                }
                Kind::SignedUp => {
                    self.busy = false;
                    match r {
                        Ok(v) => {
                            self.done = Some(v.get("message").and_then(Value::as_str).unwrap_or("").to_string());
                            self.password.clear();
                            self.again.clear();
                        }
                        Err(e) => self.error = e,
                    }
                }
                _ => {}
            },
        }
        Out::None
    }

    /// The page's body (the app puts it in its sign-in frame); `back` is its Back to sign in.
    pub fn view<'a, M: Clone + 'a>(&'a self, on: impl Fn(SignUpMsg) -> M + Clone + 'a, back: M) -> El<'a, M> {
        let w = words();
        let back_btn = settings::link(w.back, back);
        if let Some(msg) = &self.done {
            return column![widgets::t(msg.as_str()).size(15), back_btn].spacing(16).into();
        }
        let p = match &self.policy {
            None => return column![small_muted("…")].into(),
            Some(Err(e)) => return column![widgets::error_box(e.as_str()), back_btn].spacing(12).into(),
            Some(Ok(p)) => p,
        };
        let has_invite = !token_from(&self.invite).is_empty();
        let o = on.clone();
        let invite_field = field(w.invite_field, settings::input("https://…", &self.invite, move |s| o(SignUpMsg::Invite(s))));
        if let Err(why) = p.form(has_invite, &w) {
            let mut c = column![small_muted(why)].spacing(12);
            if p.who == "invite" && p.managed_by.is_none() {
                c = c.push(invite_field);
            }
            return c.push(back_btn).into();
        }
        let mut c = column![].spacing(12);
        if p.who == "invite" || has_invite {
            c = c.push(invite_field);
        }
        match &self.invite_check {
            Some(Ok(line)) => c = c.push(small_muted(line.as_str())),
            Some(Err(e)) => c = c.push(widgets::error_box(e.as_str())),
            None => {}
        }
        let email_label = if p.email_required(has_invite) { w.email.to_string() } else { format!("{} ({})", w.email, w.email_optional) };
        let (o1, o2, o3, o4, o5) = (on.clone(), on.clone(), on.clone(), on.clone(), on.clone());
        c = c
            .push(widgets::error_box(self.error.as_str()))
            .push(field("Username", settings::input("", &self.username, move |s| o1(SignUpMsg::Username(s)))))
            .push(field(email_label, settings::input("", &self.email, move |s| o2(SignUpMsg::Email(s)))))
            .push(field("Password", settings::input("", &self.password, move |s| o3(SignUpMsg::Password(s))).secure(true)))
            .push(field(
                w.confirm,
                settings::input("", &self.again, move |s| o4(SignUpMsg::Again(s))).secure(true).on_submit(o5(SignUpMsg::Submit)),
            ))
            .push(
                btn(if self.busy { w.signing_up } else { w.sign_up }, Btn::Primary, (!self.busy).then(|| on(SignUpMsg::Submit)))
                    .width(Length::Fill),
            )
            .push(back_btn);
        c.into()
    }
}

// ---- the admin's page ------------------------------------------------------------------------------------

/// What a confirm dialog of the admin page does on OK.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// revoke invite id
    Revoke(i64),
    /// reject waiting sign-up id
    Reject(i64),
}

/// How many people a new invite is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Uses {
    /// one person (a one-time link)
    #[default]
    One,
    /// up to a number
    Some,
    /// anyone with the link
    Any,
}

/// The admin's sign-up page's state.
#[derive(Clone, Debug, Default)]
pub struct Admin {
    /// the setting as edited (`None` while it loads), or the error
    pub settings: Option<Result<Settings, String>>,
    /// the roles the app offers
    pub roles: Vec<Choice>,
    /// the presets the app offers
    pub presets: Vec<Choice>,
    /// whether mail leaves the server
    pub mail_delivers: bool,
    /// the app that keeps the accounts, when not this one (the page is read-only)
    pub managed: Option<ManagedBy>,
    /// the per-hour field as typed
    pub per_hour: String,
    /// the per-day field as typed
    pub per_day: String,
    /// the invites
    pub invites: Vec<InviteRow>,
    /// the waiting sign-ups
    pub pending: Vec<PendingRow>,
    /// the new invite form is open
    pub form_open: bool,
    /// the new invite form
    pub form: InviteForm,
    /// who the new invite is for
    pub uses: Uses,
    /// the number typed for [`Uses::Some`]
    pub uses_n: String,
    /// a new invite's link, shown once
    pub link: Option<String>,
    /// the open confirm dialog
    pub confirm: Option<Confirm<Act>>,
    /// the server's address, put before links it gives without one
    pub server: String,
}

/// The admin page's messages.
#[derive(Clone, Debug, PartialEq)]
pub enum AdminMsg {
    /// who can sign up
    Who(String),
    /// the open sign-ups' role
    OpenRole(String),
    /// an open sign-up preset switched
    OpenPreset(String, bool),
    /// confirm the email first
    Verify(bool),
    /// an admin approves each one
    Approval(bool),
    /// per address per hour
    PerHour(String),
    /// per day
    PerDay(String),
    /// Save
    Save,
    /// New invite (opens or closes the form)
    ToggleForm,
    /// the invite's note
    Note(String),
    /// the invite's role
    Role(String),
    /// an invite preset switched
    Preset(String, bool),
    /// who it is for
    Uses(Uses),
    /// the number of people
    UsesN(String),
    /// how long it works, in hours
    Expiry(u32),
    /// Make invite
    Create,
    /// Copy link
    CopyLink,
    /// close the link dialog
    CloseLink,
    /// Revoke (asks first)
    Revoke(i64, String),
    /// Approve
    Approve(i64),
    /// Reject (asks first)
    Reject(i64, String),
    /// the confirm dialog
    Dialog(confirm::Msg),
    /// a request's answer
    Answer(Kind, Result<Value, String>),
}

impl Admin {
    /// The page opened on the server at `server`: its requests.
    pub fn open(&mut self, server: &str) -> Out {
        *self = Admin { server: server.to_string(), uses_n: "5".into(), ..Admin::default() };
        Out::Send(vec![Request::get("/settings", Kind::Settings)])
    }

    fn lists() -> Vec<Request> {
        vec![Request::get("/invites", Kind::Invites), Request::get("/pending", Kind::Pending)]
    }

    fn edit(&mut self, f: impl FnOnce(&mut Settings)) {
        if let Some(Ok(s)) = &mut self.settings {
            f(s);
        }
    }

    /// Applies a message.
    pub fn update(&mut self, msg: AdminMsg) -> Out {
        let w = words();
        match msg {
            AdminMsg::Who(v) => self.edit(|s| s.who = v),
            AdminMsg::OpenRole(v) => self.edit(|s| s.open_role = v),
            AdminMsg::OpenPreset(id, on) => self.edit(|s| toggle(&mut s.open_presets, id, on)),
            AdminMsg::Verify(on) => self.edit(|s| s.verify_email = on),
            AdminMsg::Approval(on) => self.edit(|s| s.approval = on),
            AdminMsg::PerHour(v) => self.per_hour = v,
            AdminMsg::PerDay(v) => self.per_day = v,
            AdminMsg::Save => {
                let Some(Ok(mut s)) = self.settings.clone() else { return Out::None };
                s.per_hour = self.per_hour.trim().parse().unwrap_or(s.per_hour);
                s.per_day = self.per_day.trim().parse().unwrap_or(s.per_day);
                return Out::Send(vec![Request { method: "PUT", path: format!("{BASE}/settings"), body: s.to_json(), kind: Kind::Saved }]);
            }
            AdminMsg::ToggleForm => {
                self.form_open = !self.form_open;
                if self.form.role.is_empty() {
                    self.form.role = self.roles.first().map(|r| r.id.clone()).unwrap_or_default();
                }
            }
            AdminMsg::Note(v) => self.form.note = v,
            AdminMsg::Role(v) => self.form.role = v,
            AdminMsg::Preset(id, on) => toggle(&mut self.form.presets, id, on),
            AdminMsg::Uses(u) => self.uses = u,
            AdminMsg::UsesN(v) => self.uses_n = v,
            AdminMsg::Expiry(h) => self.form.expires_in_hours = h,
            AdminMsg::Create => {
                let mut f = self.form.clone();
                f.max_uses = match self.uses {
                    Uses::One => 1,
                    Uses::Any => 0,
                    Uses::Some => match parse_uses(&self.uses_n) {
                        Some(n) => n,
                        None => return Out::Toast("Enter a number of people from 2 to 100000".into(), true),
                    },
                };
                return Out::Send(vec![Request::post("/invites", f.to_json(), Kind::Created)]);
            }
            AdminMsg::CopyLink => return self.link.clone().map(Out::Copy).unwrap_or(Out::None),
            AdminMsg::CloseLink => self.link = None,
            AdminMsg::Revoke(id, title) => {
                self.confirm =
                    Some(Confirm::new(fill(w.revoke_question, &[("note", &title)]), w.revoke_text, w.revoke, Act::Revoke(id)).danger())
            }
            AdminMsg::Reject(id, name) => {
                self.confirm =
                    Some(Confirm::new(fill(w.reject_question, &[("username", &name)]), w.reject_text, w.reject, Act::Reject(id)).danger())
            }
            AdminMsg::Approve(id) => return Out::Send(vec![Request::post(&format!("/pending/{id}/approve"), Value::Null, Kind::Changed)]),
            AdminMsg::Dialog(m) => {
                let act = self.confirm.take().and_then(|c| c.update(m));
                return match act {
                    Some(Act::Revoke(id)) => Out::Send(vec![Request::post(&format!("/invites/{id}/revoke"), Value::Null, Kind::Changed)]),
                    Some(Act::Reject(id)) => Out::Send(vec![Request::post(&format!("/pending/{id}/reject"), Value::Null, Kind::Changed)]),
                    None => Out::None,
                };
            }
            AdminMsg::Answer(kind, r) => return self.answer(kind, r),
        }
        Out::None
    }

    fn answer(&mut self, kind: Kind, r: Result<Value, String>) -> Out {
        let v = match r {
            Ok(v) => v,
            Err(e) if kind == Kind::Settings => {
                self.settings = Some(Err(e));
                return Out::None;
            }
            Err(e) => return Out::Toast(e, true),
        };
        match kind {
            Kind::Settings | Kind::Saved => {
                let s = Settings::from_json(v.get("settings").unwrap_or(&Value::Null));
                self.per_hour = s.per_hour.to_string();
                self.per_day = s.per_day.to_string();
                self.settings = Some(Ok(s));
                self.roles = choices(&v, "roles");
                self.presets = choices(&v, "presets");
                self.mail_delivers = v.get("mail_delivers").and_then(Value::as_bool).unwrap_or(false);
                self.managed = ManagedBy::from_json(&v);
                if kind == Kind::Saved {
                    return Out::Toast(words().saved.into(), false);
                }
                if self.managed.is_none() {
                    return Out::Send(Admin::lists());
                }
            }
            Kind::Invites => self.invites = InviteRow::list(&v),
            Kind::Pending => self.pending = PendingRow::list(&v),
            Kind::Created => {
                self.form_open = false;
                self.form = InviteForm { role: self.form.role.clone(), ..InviteForm::default() };
                self.uses = Uses::One;
                self.link = Some(full_link(v.get("link").and_then(Value::as_str).unwrap_or(""), &self.server));
                return Out::Send(Admin::lists());
            }
            Kind::Changed => return Out::Send(Admin::lists()),
            _ => {}
        }
        Out::None
    }

    /// The page; `now` (seconds since 1970) words the invites' expiry.
    pub fn view(&self, now: f64) -> El<'_, AdminMsg> {
        let w = words();
        let s = match &self.settings {
            None => return small_muted("…").into(),
            Some(Err(e)) => return widgets::error_box(e.as_str()),
            Some(Ok(s)) => s,
        };
        let read_only = self.managed.is_some();
        let mut page = column![].spacing(18).width(Length::Fill);
        if let Some(m) = &self.managed {
            let mut line = fill(w.read_only, &[("app", &m.name)]);
            if !m.reachable {
                line = format!("{line} {}", fill(w.unreachable, &[("app", &m.name)]));
            }
            page = page.push(container(small(line)).padding([10, 14]).width(Length::Fill).style(crate::theme::card));
        }
        page = page.push(self.settings_card(s, read_only));
        if !read_only {
            page = page.push(self.pending_card()).push(self.invites_card(now));
        }
        page.into()
    }

    /// The dialog over the page (the new link, or a question), if one is open.
    pub fn overlay(&self) -> Option<El<'_, AdminMsg>> {
        if let Some(c) = &self.confirm {
            return Some(confirm::view(c).map(AdminMsg::Dialog));
        }
        let link = self.link.as_ref()?;
        let w = words();
        let body = column![
            small_muted(w.link_once),
            container(widgets::t(link.as_str()).size(13).font(iced::Font::MONOSPACE)).padding(10).style(crate::theme::card)
        ]
        .spacing(10);
        let foot = row![
            btn(kit_rules::dialogs::CLOSE, Btn::Default, Some(AdminMsg::CloseLink)),
            btn(w.copy_link, Btn::Primary, Some(AdminMsg::CopyLink))
        ]
        .spacing(10);
        Some(modal(w.link_title, body, Some(foot.into()), false, AdminMsg::CloseLink))
    }

    fn settings_card<'a>(&'a self, s: &'a Settings, read_only: bool) -> El<'a, AdminMsg> {
        let w = words();
        let mut c = column![h2(w.who_title)].spacing(12);
        for (id, name, line) in WHO {
            let r = radio(name, id, Some(s.who.as_str()), |v: &str| AdminMsg::Who(v.to_string()));
            let r: El<'a, AdminMsg> = if read_only {
                container(widgets::t(format!("{} {name}", if s.who == id { "●" } else { "○" }))).into()
            } else {
                r.into()
            };
            c = c.push(column![r, small_muted(line)].spacing(2));
        }
        if s.who == "open" {
            let role_opts: Vec<Opt> = self.roles.iter().map(|r| Opt::new(r.id.clone(), r.label.clone())).collect();
            let role =
                if s.open_role.is_empty() { self.roles.first().map(|r| r.id.clone()).unwrap_or_default() } else { s.open_role.clone() };
            let mut open = column![h3(w.open_title)].spacing(10);
            open = open.push(field(w.open_role, settings::select(role_opts, &role, AdminMsg::OpenRole)));
            for p in &self.presets {
                let id = p.id.clone();
                let on = s.open_presets.contains(&p.id);
                let f = (!read_only).then_some(move |b| AdminMsg::OpenPreset(id.clone(), b));
                open = open.push(settings::switch_maybe(on, p.label.as_str(), f));
            }
            open = open.push(settings::switch_maybe(s.verify_email, w.verify_email, (!read_only).then_some(AdminMsg::Verify)));
            if !self.mail_delivers {
                open = open.push(small_muted(w.no_mail));
            }
            open = open.push(settings::switch_maybe(s.approval, w.approval, (!read_only).then_some(AdminMsg::Approval)));
            open = open.push(settings::two(
                field(w.per_hour, settings::input("5", &self.per_hour, AdminMsg::PerHour)),
                Some(field(w.per_day, settings::input("50", &self.per_day, AdminMsg::PerDay))),
            ));
            c = c.push(open);
        }
        if !read_only {
            c = c.push(btn(w.save, Btn::Primary, Some(AdminMsg::Save)));
        }
        card(c.width(Length::Fill)).width(Length::Fill).into()
    }

    fn pending_card(&self) -> El<'_, AdminMsg> {
        let w = words();
        let mut c = column![h2(w.pending_title)].spacing(10);
        if self.pending.is_empty() {
            c = c.push(small_muted(w.no_pending));
        }
        for p in &self.pending {
            let mut who = p.username.clone();
            if !p.email.is_empty() {
                who = format!("{who} · {}", p.email);
            }
            let line = format!("{} · {} · {}", p.waiting_line(&w), day(p.created_at), p.address);
            c = c.push(
                row![
                    column![widgets::t(who).size(14), small_muted(line)].spacing(2).width(Length::Fill),
                    btn(w.approve, Btn::Default, Some(AdminMsg::Approve(p.id))),
                    btn(w.reject, Btn::Danger, Some(AdminMsg::Reject(p.id, p.username.clone()))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        card(c.width(Length::Fill)).width(Length::Fill).into()
    }

    fn invites_card(&self, now: f64) -> El<'_, AdminMsg> {
        let w = words();
        let head = row![h2(w.invites_title).width(Length::Fill), btn(w.new_invite, Btn::Default, Some(AdminMsg::ToggleForm))]
            .align_y(Alignment::Center);
        let mut c = column![head].spacing(12);
        if self.form_open {
            c = c.push(self.invite_form());
        }
        if self.invites.is_empty() {
            c = c.push(small_muted(w.no_invites));
        }
        for i in &self.invites {
            let (label, class) = invite_status(&i.status);
            let kind = match class {
                "ok" => Badge::Ok,
                "err" => Badge::Err,
                "warn" => Badge::Warn,
                _ => Badge::Plain,
            };
            let title = i.title(&w);
            let gives = std::iter::once(label_of(&self.roles, &i.role))
                .chain(i.presets.iter().map(|p| label_of(&self.presets, p)))
                .collect::<Vec<_>>()
                .join(", ");
            let used = if i.used_by.is_empty() {
                w.unused.to_string()
            } else {
                format!("{}: {}", w.used_by, i.used_by.iter().map(|(n, at)| format!("{n} ({})", day(*at))).collect::<Vec<_>>().join(", "))
            };
            let mut r = row![column![
                row![widgets::t(title.clone()).size(14).font(crate::theme::semibold()), badge(label, kind)]
                    .spacing(8)
                    .align_y(Alignment::Center),
                small_muted(format!("{gives} · {} · {}", i.uses_line(), i.expiry_line(now))),
                small_muted(format!("{} · {}", i.created_by, day(i.created_at))),
                small(used),
            ]
            .spacing(2)
            .width(Length::Fill)]
            .spacing(10)
            .align_y(Alignment::Center);
            if i.can_revoke() {
                r = r.push(btn(w.revoke, Btn::Danger, Some(AdminMsg::Revoke(i.id, title))));
            }
            c = c.push(r);
        }
        card(c.width(Length::Fill)).width(Length::Fill).into()
    }

    fn invite_form(&self) -> El<'_, AdminMsg> {
        let w = words();
        let role_opts: Vec<Opt> = self.roles.iter().map(|r| Opt::new(r.id.clone(), r.label.clone())).collect();
        let expiry: Vec<Opt> = EXPIRY.iter().map(|(h, l)| Opt::new(h.to_string(), *l)).collect();
        let mut c = column![
            field(w.note, settings::input(w.note_placeholder, &self.form.note, AdminMsg::Note)),
            settings::two(
                field(w.role, settings::select(role_opts, &self.form.role, AdminMsg::Role)),
                Some(field(
                    w.expires,
                    settings::select(expiry, &self.form.expires_in_hours.to_string(), |h| AdminMsg::Expiry(h.parse().unwrap_or(0)))
                )),
            ),
        ]
        .spacing(10);
        for p in &self.presets {
            let id = p.id.clone();
            c = c.push(settings::switch(self.form.presets.contains(&p.id), p.label.as_str(), move |b| AdminMsg::Preset(id.clone(), b)));
        }
        let active = match self.uses {
            Uses::One => "one",
            Uses::Some => "some",
            Uses::Any => "any",
        };
        let mut uses =
            row![seg(&[("one", w.one_time), ("some", w.several), ("any", w.no_limit)], active, |v| AdminMsg::Uses(match v.as_str() {
                "one" => Uses::One,
                "some" => Uses::Some,
                _ => Uses::Any,
            }))]
            .spacing(10)
            .align_y(Alignment::Center);
        if self.uses == Uses::Some {
            uses = uses.push(text_input("5", &self.uses_n).on_input(AdminMsg::UsesN).width(90).padding([8, 10]).style(crate::theme::input));
        }
        c = c.push(field(w.uses, uses)).push(btn(w.create, Btn::Primary, Some(AdminMsg::Create)));
        widgets::card_flat(container(c).padding(14)).width(Length::Fill).into()
    }
}

/// `id` in or out of `list`.
fn toggle(list: &mut Vec<String>, id: String, on: bool) {
    list.retain(|x| x != &id);
    if on {
        list.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_answer() -> Value {
        json!({ "settings": { "who": "invite", "per_hour": 5, "per_day": 50 }, "roles": [{ "id": "member", "label": "Member" }],
            "presets": [{ "id": "shared", "label": "Shared folder" }], "mail_delivers": false, "managed_by": null, "read_only": false })
    }

    #[test]
    fn the_admin_page_loads_saves_and_makes_invites() {
        let mut a = Admin::default();
        assert_eq!(a.open("https://h.example"), Out::Send(vec![Request::get("/settings", Kind::Settings)]));
        let out = a.update(AdminMsg::Answer(Kind::Settings, Ok(settings_answer())));
        assert_eq!(out, Out::Send(Admin::lists()));
        a.update(AdminMsg::Who("open".into()));
        a.update(AdminMsg::Approval(true));
        a.update(AdminMsg::PerHour("3".into()));
        let Out::Send(r) = a.update(AdminMsg::Save) else { panic!() };
        assert_eq!((r[0].method, r[0].path.as_str()), ("PUT", "/api/signup/settings"));
        assert_eq!(
            (r[0].body["who"].clone(), r[0].body["approval"].clone(), r[0].body["per_hour"].clone()),
            (json!("open"), json!(true), json!(3))
        );
        // a new invite for up to 4 people
        a.update(AdminMsg::ToggleForm);
        a.update(AdminMsg::Uses(Uses::Some));
        a.update(AdminMsg::UsesN("1".into()));
        assert!(matches!(a.update(AdminMsg::Create), Out::Toast(_, true)));
        a.update(AdminMsg::UsesN("4".into()));
        let Out::Send(r) = a.update(AdminMsg::Create) else { panic!() };
        assert_eq!(
            (r[0].body["max_uses"].clone(), r[0].body["role"].clone(), r[0].body["expires_in_hours"].clone()),
            (json!(4), json!("member"), json!(168))
        );
        a.update(AdminMsg::Answer(Kind::Created, Ok(json!({ "link": "/#/sign-up?invite=abc" }))));
        assert_eq!(a.link.as_deref(), Some("https://h.example/#/sign-up?invite=abc"));
        assert_eq!(a.update(AdminMsg::CopyLink), Out::Copy("https://h.example/#/sign-up?invite=abc".into()));
        assert!(a.overlay().is_some());
        a.update(AdminMsg::CloseLink);
        assert!(a.overlay().is_none());
    }

    #[test]
    fn revoke_and_reject_ask_first() {
        let mut a = Admin::default();
        a.update(AdminMsg::Revoke(3, "team".into()));
        assert_eq!(a.confirm.as_ref().unwrap().title, "Revoke “team”?");
        assert_eq!(a.update(AdminMsg::Dialog(confirm::Msg::Cancel)), Out::None);
        a.update(AdminMsg::Reject(4, "vic".into()));
        let Out::Send(r) = a.update(AdminMsg::Dialog(confirm::Msg::Ok)) else { panic!() };
        assert_eq!(r[0].path, "/api/signup/pending/4/reject");
        let Out::Send(r) = a.update(AdminMsg::Approve(4)) else { panic!() };
        assert_eq!(r[0].path, "/api/signup/pending/4/approve");
    }

    #[test]
    fn a_managed_page_is_read_only_and_loads_no_lists() {
        let mut a = Admin::default();
        a.open("");
        let mut v = settings_answer();
        v["managed_by"] = json!({ "name": "Hub", "url": "https://hub", "reachable": true });
        assert_eq!(a.update(AdminMsg::Answer(Kind::Settings, Ok(v))), Out::None);
        let mut ui = iced_test::simulator(a.view(0.0));
        assert!(ui.find("Accounts are managed by Hub: this setting is changed there and applies to every linked app.").is_ok());
        assert!(ui.find("Save").is_err());
    }

    #[test]
    fn the_admin_page_draws_its_lists() {
        let mut a = Admin::default();
        a.open("");
        a.update(AdminMsg::Answer(Kind::Settings, Ok(settings_answer())));
        a.update(AdminMsg::Answer(Kind::Invites, Ok(json!({ "invites": [{ "id": 1, "note": "team", "role": "member", "max_uses": 0, "uses": 1, "status": "active", "used_by": [{ "username": "ana", "used_at": 0 }] }] }))));
        a.update(AdminMsg::Answer(Kind::Pending, Ok(json!({ "pending": [{ "id": 2, "username": "vic", "waiting_for": "approval" }] }))));
        let mut ui = iced_test::simulator(a.view(0.0));
        assert!(ui.find("team").is_ok() && ui.find("Used by: ana (—)").is_ok() && ui.find("vic").is_ok());
        assert!(ui.click("Approve").is_ok());
        assert_eq!(ui.into_messages().collect::<Vec<_>>(), [AdminMsg::Approve(2)]);
    }

    #[test]
    fn the_sign_up_page_checks_then_sends() {
        let mut s = SignUp::default();
        let t = "a".repeat(64);
        let Out::Send(r) = s.open(&format!("https://h/#/sign-up?invite={t}")) else { panic!() };
        assert_eq!(r.iter().map(|r| r.kind).collect::<Vec<_>>(), [Kind::Policy, Kind::Invite]);
        s.update(SignUpMsg::Answer(Kind::Policy, Ok(json!({ "who": "invite" }))));
        s.update(SignUpMsg::Answer(Kind::Invite, Ok(json!({ "role": "Member" }))));
        assert_eq!(s.invite_check, Some(Ok("This invite makes a Member account.".into())));
        s.update(SignUpMsg::Username("ana".into()));
        s.update(SignUpMsg::Password("password1".into()));
        s.update(SignUpMsg::Again("password2".into()));
        assert_eq!(s.update(SignUpMsg::Submit), Out::None);
        assert_eq!(s.error, "The two passwords aren't the same");
        s.update(SignUpMsg::Again("password1".into()));
        let Out::Send(r) = s.update(SignUpMsg::Submit) else { panic!() };
        assert_eq!(r[0].body["invite"], json!(t));
        s.update(SignUpMsg::Answer(Kind::SignedUp, Ok(json!({ "status": "active", "message": "Your account is ready. Sign in." }))));
        assert_eq!(s.done.as_deref(), Some("Your account is ready. Sign in."));
        let mut ui = iced_test::simulator(s.view(|m| m, SignUpMsg::Submit));
        assert!(ui.find("Your account is ready. Sign in.").is_ok());
    }
}
