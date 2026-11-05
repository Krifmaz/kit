//! Sign-up: who can sign up, invite links, and the sign-up form, as every client shows them.
//!
//! The server is kit-server-signup (`contracts/signup.md`). [`Policy`] reads `GET /api/signup` (what the sign-in
//! page offers); [`Settings`] is the admin's setting (`GET`/`PUT /api/signup/settings`); [`InviteRow`] and
//! [`PendingRow`] are the rows of the admin's lists. The words are [`SignupWords`]. Times are seconds since 1970,
//! passed in (`now`), like the server keeps them.

use serde_json::{json, Value};

/// Who can sign up: the value, its name and one sentence, in the order the admin's choice shows them.
pub const WHO: [(&str, &str, &str); 3] = [
    ("closed", "Closed", "Only admins make accounts."),
    ("invite", "Invite only", "People sign up with an invite link an admin made. This is the default."),
    ("open", "Open", "Anyone can sign up, within the limits below."),
];

/// How long a new invite works, in hours (0: never), and its words; the default is the second.
pub const EXPIRY: [(u32, &str); 4] = [(24, "1 day"), (168, "7 days"), (720, "30 days"), (0, "Never")];
/// The new invite form's expiry when nothing is picked: 7 days.
pub const DEFAULT_EXPIRY_HOURS: u32 = 168;

/// Every word the sign-up parts show. An app replaces any of them.
#[derive(Debug, Clone, PartialEq)]
pub struct SignupWords {
    /// the sign-up page's title
    pub title: &'static str,
    /// the sign-in page's link to the sign-up page (open sign-up)
    pub create_account: &'static str,
    /// the sign-in page's line while sign-up is by invite only
    pub invite_hint: &'static str,
    /// the sign-in page's line when another app keeps the accounts (`{app}`)
    pub managed_hint: &'static str,
    /// the sign-up page when sign-up is closed
    pub closed: &'static str,
    /// the sign-up page without an invite while sign-up is by invite only
    pub need_invite: &'static str,
    /// the invite field's label (desktop and Android: paste the link)
    pub invite_field: &'static str,
    /// the line under a checked invite (`{role}`)
    pub invite_gives: &'static str,
    /// the email field's label
    pub email: &'static str,
    /// the email field's hint when it is optional
    pub email_optional: &'static str,
    /// the password check field's label
    pub confirm: &'static str,
    /// the two passwords differ
    pub mismatch: &'static str,
    /// no name given
    pub need_name: &'static str,
    /// no email given while it is needed
    pub need_email: &'static str,
    /// the password is too short (`{n}`)
    pub short_password: &'static str,
    /// the sign-up button
    pub sign_up: &'static str,
    /// the sign-up button while it goes
    pub signing_up: &'static str,
    /// the link back to signing in
    pub back: &'static str,
    /// the confirmation page while it works
    pub verifying: &'static str,
    /// the admin page's title
    pub admin_title: &'static str,
    /// the setting's heading
    pub who_title: &'static str,
    /// the open sign-up block's heading
    pub open_title: &'static str,
    /// the role for open sign-ups
    pub open_role: &'static str,
    /// the presets for open sign-ups
    pub open_presets: &'static str,
    /// the email confirmation switch
    pub verify_email: &'static str,
    /// the line under it when mail isn't set up
    pub no_mail: &'static str,
    /// the approval switch
    pub approval: &'static str,
    /// sign-ups per address per hour
    pub per_hour: &'static str,
    /// sign-ups per day (0: no limit)
    pub per_day: &'static str,
    /// the save button
    pub save: &'static str,
    /// the toast once saved
    pub saved: &'static str,
    /// the read-only banner (`{app}`)
    pub read_only: &'static str,
    /// the read-only banner's addition when the source can't be reached
    pub unreachable: &'static str,
    /// the invites block's heading
    pub invites_title: &'static str,
    /// the new invite button
    pub new_invite: &'static str,
    /// the invite's note field
    pub note: &'static str,
    /// the invite's note placeholder
    pub note_placeholder: &'static str,
    /// the role field
    pub role: &'static str,
    /// the presets field
    pub presets: &'static str,
    /// the uses field
    pub uses: &'static str,
    /// one-time
    pub one_time: &'static str,
    /// no limit
    pub no_limit: &'static str,
    /// a number of uses
    pub several: &'static str,
    /// the expiry field
    pub expires: &'static str,
    /// the make button
    pub create: &'static str,
    /// the dialog that shows a new link once
    pub link_title: &'static str,
    /// the line in it
    pub link_once: &'static str,
    /// copy the link
    pub copy_link: &'static str,
    /// the toast once copied
    pub copied: &'static str,
    /// revoke
    pub revoke: &'static str,
    /// the revoke question (`{note}`)
    pub revoke_question: &'static str,
    /// the revoke question's text
    pub revoke_text: &'static str,
    /// no invites yet
    pub no_invites: &'static str,
    /// who used it
    pub used_by: &'static str,
    /// nobody used it yet
    pub unused: &'static str,
    /// an invite without a note
    pub untitled: &'static str,
    /// the waiting block's heading
    pub pending_title: &'static str,
    /// nobody waits
    pub no_pending: &'static str,
    /// approve
    pub approve: &'static str,
    /// reject
    pub reject: &'static str,
    /// the reject question (`{username}`)
    pub reject_question: &'static str,
    /// the reject question's text
    pub reject_text: &'static str,
    /// waiting for the email
    pub waiting_email: &'static str,
    /// waiting for an admin
    pub waiting_approval: &'static str,
}

impl Default for SignupWords {
    fn default() -> Self {
        SignupWords {
            title: "Create your account",
            create_account: "Create an account",
            invite_hint: "New here? Open the invite link an admin sent you.",
            managed_hint: "Accounts are managed by {app}. Sign up there: the same account works here.",
            closed: "Sign-up is closed. Ask an admin to make you an account.",
            need_invite: "Sign-up is by invite only. Open the invite link an admin sent you, or paste it below.",
            invite_field: "Invite link",
            invite_gives: "This invite makes a {role} account.",
            email: "Email",
            email_optional: "Optional",
            confirm: "Password again",
            mismatch: "The two passwords aren't the same",
            need_name: "Choose a username",
            need_email: "Enter your email address: we send a link to confirm it.",
            short_password: "Use a password of at least {n} characters",
            sign_up: "Create account",
            signing_up: "Creating…",
            back: "Back to sign in",
            verifying: "Confirming your email…",
            admin_title: "Sign-up",
            who_title: "Who can sign up",
            open_title: "Open sign-up",
            open_role: "New accounts get the role",
            open_presets: "and these presets",
            verify_email: "Confirm their email first",
            no_mail: "Mail isn't set up on this server: confirmation links only go to the server's log.",
            approval: "An admin approves each one",
            per_hour: "Sign-ups per address per hour",
            per_day: "Sign-ups per day (0: no limit)",
            save: "Save",
            saved: "Saved",
            read_only: "Accounts are managed by {app}: this setting is changed there and applies to every linked app.",
            unreachable: "{app} can't be reached right now, so its setting isn't shown.",
            invites_title: "Invites",
            new_invite: "New invite",
            note: "Note",
            note_placeholder: "Who it's for (only admins see this)",
            role: "Role",
            presets: "Presets",
            uses: "Can be used by",
            one_time: "One person",
            no_limit: "Anyone with the link",
            several: "Up to",
            expires: "Works for",
            create: "Make invite",
            link_title: "Invite link",
            link_once: "Copy it now: for safety, the link is shown only this once.",
            copy_link: "Copy link",
            copied: "Link copied",
            revoke: "Revoke",
            revoke_question: "Revoke “{note}”?",
            revoke_text: "The link stops working at once. Accounts already made with it stay.",
            no_invites: "No invites yet. New invite makes a link to send.",
            used_by: "Used by",
            unused: "Not used yet",
            untitled: "Invite",
            pending_title: "Waiting sign-ups",
            no_pending: "Nobody is waiting.",
            approve: "Approve",
            reject: "Reject",
            reject_question: "Reject {username}?",
            reject_text: "Their account is removed. They can sign up again.",
            waiting_email: "Waiting for their email",
            waiting_approval: "Waiting for approval",
        }
    }
}

/// The shortest password the form takes before asking the server (the server has the last word).
pub const MIN_PASSWORD: usize = 8;

/// The app that keeps the accounts when apps share them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManagedBy {
    /// its name
    pub name: String,
    /// its address
    pub url: String,
    /// whether this app could reach it just now
    pub reachable: bool,
}

impl ManagedBy {
    /// The `managed_by` object of an answer; `None` for null or missing.
    pub fn from_json(v: &Value) -> Option<ManagedBy> {
        let m = v.get("managed_by").filter(|m| m.is_object())?;
        Some(ManagedBy {
            name: text(m, "name"),
            url: text(m, "url"),
            reachable: m.get("reachable").and_then(Value::as_bool).unwrap_or(true),
        })
    }
}

/// What `GET /api/signup` says: who can sign up and how.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Policy {
    /// `closed`, `invite` or `open`; "" when unknown (the account source can't be reached)
    pub who: String,
    /// open sign-ups confirm their email
    pub verify_email: bool,
    /// open sign-ups wait for an admin
    pub approval: bool,
    /// the app that keeps the accounts, if not this one
    pub managed_by: Option<ManagedBy>,
}

impl Policy {
    /// The answer read; an unknown or missing `who` stays "".
    pub fn from_json(v: &Value) -> Policy {
        Policy {
            who: text(v, "who"),
            verify_email: flag(v, "verify_email"),
            approval: flag(v, "approval"),
            managed_by: ManagedBy::from_json(v),
        }
    }

    /// The line under the sign-in form: a link to sign up (open), how to get in (invite only), where accounts
    /// are made (another app keeps them), or nothing (closed). `(text, link)`: `link` says the text opens the
    /// sign-up page, or (with an account source) the source's address.
    pub fn sign_in_line(&self, w: &SignupWords) -> Option<(String, SignInLink)> {
        if let Some(m) = &self.managed_by {
            return Some((crate::app::fill(w.managed_hint, &[("app", &m.name)]), SignInLink::Source(m.url.clone())));
        }
        match self.who.as_str() {
            "open" => Some((w.create_account.to_string(), SignInLink::SignUp)),
            "invite" => Some((w.invite_hint.to_string(), SignInLink::None)),
            _ => None,
        }
    }

    /// Whether the sign-up form shows, with or without an invite, and the sentence instead when it doesn't.
    pub fn form(&self, has_invite: bool, w: &SignupWords) -> Result<(), String> {
        if let Some(m) = &self.managed_by {
            return Err(crate::app::fill(w.managed_hint, &[("app", &m.name)]));
        }
        match self.who.as_str() {
            "closed" => Err(w.closed.to_string()),
            "invite" if !has_invite => Err(w.need_invite.to_string()),
            _ => Ok(()),
        }
    }

    /// Whether the email field must be filled: open sign-up with confirmation, and no invite.
    pub fn email_required(&self, has_invite: bool) -> bool {
        self.who == "open" && self.verify_email && !has_invite
    }
}

/// What the sign-in page's line opens.
#[derive(Debug, Clone, PartialEq)]
pub enum SignInLink {
    /// nothing: the line is only words
    None,
    /// this app's sign-up page
    SignUp,
    /// the account source's address
    Source(String),
}

/// The sign-up form's checks before it is sent; the sentence of the first that fails.
pub fn check_form(username: &str, password: &str, again: &str, email: &str, email_required: bool, w: &SignupWords) -> Result<(), String> {
    if username.trim().is_empty() {
        return Err(w.need_name.to_string());
    }
    if password.chars().count() < MIN_PASSWORD {
        return Err(crate::app::fill(w.short_password, &[("n", &MIN_PASSWORD.to_string())]));
    }
    if password != again {
        return Err(w.mismatch.to_string());
    }
    if email_required && email.trim().is_empty() {
        return Err(w.need_email.to_string());
    }
    Ok(())
}

/// The `POST /api/signup` body.
pub fn sign_up_body(username: &str, password: &str, email: &str, invite: &str) -> Value {
    json!({ "username": username.trim(), "password": password, "email": email.trim(), "invite": invite.trim() })
}

/// The invite token in what was pasted: an invite link (`…invite=<token>…`) or the token alone; "" for neither.
pub fn token_from(pasted: &str) -> String {
    let p = pasted.trim();
    let from = match p.find("invite=").or_else(|| p.find("token=")) {
        Some(at) => &p[at + p[at..].find('=').unwrap_or(0) + 1..],
        None => p,
    };
    let t: String = from.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    if t.len() >= 16 {
        t
    } else {
        String::new()
    }
}

/// A link from the server: as it is when it has an address, else after `server` (`https://host`).
pub fn full_link(link: &str, server: &str) -> String {
    if link.starts_with("http://") || link.starts_with("https://") {
        link.to_string()
    } else {
        format!("{}{link}", server.trim_end_matches('/'))
    }
}

/// The admin's setting, as `GET`/`PUT /api/signup/settings` carry it.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// `closed`, `invite` or `open`
    pub who: String,
    /// the role of open sign-ups ("" for the app's first)
    pub open_role: String,
    /// the presets of open sign-ups
    pub open_presets: Vec<String>,
    /// open sign-ups confirm their email
    pub verify_email: bool,
    /// open sign-ups wait for an admin
    pub approval: bool,
    /// sign-ups per address per hour
    pub per_hour: u32,
    /// sign-ups per day, 0 for no limit
    pub per_day: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            who: "invite".into(),
            open_role: String::new(),
            open_presets: Vec::new(),
            verify_email: false,
            approval: false,
            per_hour: 5,
            per_day: 50,
        }
    }
}

impl Settings {
    /// The `settings` object read, defaults for what is missing.
    pub fn from_json(v: &Value) -> Settings {
        let d = Settings::default();
        let n = |k: &str, d: u32| v.get(k).and_then(Value::as_u64).map(|n| n.min(u64::from(u32::MAX)) as u32).unwrap_or(d);
        Settings {
            who: Some(text(v, "who")).filter(|w| !w.is_empty()).unwrap_or(d.who),
            open_role: text(v, "open_role"),
            open_presets: strings(v, "open_presets"),
            verify_email: flag(v, "verify_email"),
            approval: flag(v, "approval"),
            per_hour: n("per_hour", d.per_hour),
            per_day: n("per_day", d.per_day),
        }
    }

    /// The `PUT` body.
    pub fn to_json(&self) -> Value {
        json!({
            "who": self.who, "open_role": self.open_role, "open_presets": self.open_presets,
            "verify_email": self.verify_email, "approval": self.approval, "per_hour": self.per_hour, "per_day": self.per_day,
        })
    }
}

/// A role or preset the app offers.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// its id
    pub id: String,
    /// its name
    pub label: String,
}

/// The `roles` or `presets` list of the settings answer.
pub fn choices(v: &Value, key: &str) -> Vec<Choice> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().map(|c| Choice { id: text(c, "id"), label: text(c, "label") }).collect())
        .unwrap_or_default()
}

/// The label of `id` among `list`; `id` itself when the app doesn't offer it any more.
pub fn label_of(list: &[Choice], id: &str) -> String {
    list.iter().find(|c| c.id == id).map(|c| c.label.clone()).unwrap_or_else(|| id.to_string())
}

/// The new invite form, as the admin filled it.
#[derive(Debug, Clone, PartialEq)]
pub struct InviteForm {
    /// the note
    pub note: String,
    /// the role ("" for the app's first)
    pub role: String,
    /// the presets
    pub presets: Vec<String>,
    /// 1 one-time, 0 no limit, or a number
    pub max_uses: i64,
    /// hours it works, 0 never
    pub expires_in_hours: u32,
}

impl Default for InviteForm {
    fn default() -> Self {
        InviteForm { note: String::new(), role: String::new(), presets: Vec::new(), max_uses: 1, expires_in_hours: DEFAULT_EXPIRY_HOURS }
    }
}

impl InviteForm {
    /// The `POST /api/signup/invites` body.
    pub fn to_json(&self) -> Value {
        json!({ "note": self.note.trim(), "role": self.role, "presets": self.presets, "max_uses": self.max_uses, "expires_in_hours": self.expires_in_hours })
    }
}

/// The uses field's number typed in, between 2 and 100 000; `None` for anything else.
pub fn parse_uses(text: &str) -> Option<i64> {
    text.trim().parse::<i64>().ok().filter(|n| (2..=100_000).contains(n))
}

/// An invite of the admin's list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InviteRow {
    /// its id
    pub id: i64,
    /// the note ("" for none)
    pub note: String,
    /// the role's id
    pub role: String,
    /// the presets' ids
    pub presets: Vec<String>,
    /// 1 one-time, 0 no limit
    pub max_uses: i64,
    /// accounts made
    pub uses: i64,
    /// when it stops working, 0 never
    pub expires_at: f64,
    /// `active`, `revoked`, `expired` or `used_up`
    pub status: String,
    /// the admin who made it
    pub created_by: String,
    /// when
    pub created_at: f64,
    /// who used it: (name, when)
    pub used_by: Vec<(String, f64)>,
}

impl InviteRow {
    /// One invite of `invites`.
    pub fn from_json(v: &Value) -> InviteRow {
        InviteRow {
            id: v.get("id").and_then(Value::as_i64).unwrap_or(0),
            note: text(v, "note"),
            role: text(v, "role"),
            presets: strings(v, "presets"),
            max_uses: v.get("max_uses").and_then(Value::as_i64).unwrap_or(1),
            uses: v.get("uses").and_then(Value::as_i64).unwrap_or(0),
            expires_at: v.get("expires_at").and_then(Value::as_f64).unwrap_or(0.0),
            status: text(v, "status"),
            created_by: text(v, "created_by_name"),
            created_at: v.get("created_at").and_then(Value::as_f64).unwrap_or(0.0),
            used_by: v
                .get("used_by")
                .and_then(Value::as_array)
                .map(|a| a.iter().map(|u| (text(u, "username"), u.get("used_at").and_then(Value::as_f64).unwrap_or(0.0))).collect())
                .unwrap_or_default(),
        }
    }

    /// The list of `{"invites": […]}`.
    pub fn list(v: &Value) -> Vec<InviteRow> {
        v.get("invites").and_then(Value::as_array).map(|a| a.iter().map(InviteRow::from_json).collect()).unwrap_or_default()
    }

    /// Its title: the note, else "Invite <id>".
    pub fn title(&self, w: &SignupWords) -> String {
        if self.note.trim().is_empty() {
            format!("{} {}", w.untitled, self.id)
        } else {
            self.note.clone()
        }
    }

    /// "1 of 1 use", "2 of 5 uses", "3 uses, no limit".
    pub fn uses_line(&self) -> String {
        let s = |n: i64| if n == 1 { "" } else { "s" };
        if self.max_uses > 0 {
            format!("{} of {} use{}", self.uses, self.max_uses, s(self.max_uses))
        } else {
            format!("{} use{}, no limit", self.uses, s(self.uses))
        }
    }

    /// "Never expires", "Expires in 3 days", "Expires in 5 hours", "Expired".
    pub fn expiry_line(&self, now: f64) -> String {
        if self.expires_at <= 0.0 {
            return "Never expires".into();
        }
        let left = self.expires_at - now;
        if left <= 0.0 {
            return "Expired".into();
        }
        let hours = (left / 3600.0).ceil() as i64;
        if hours < 48 {
            format!("Expires in {hours} hour{}", if hours == 1 { "" } else { "s" })
        } else {
            format!("Expires in {} days", hours / 24)
        }
    }

    /// Whether Revoke shows: only while it works.
    pub fn can_revoke(&self) -> bool {
        self.status == "active"
    }
}

/// A status's words and badge class.
pub fn invite_status(status: &str) -> (&'static str, &'static str) {
    match status {
        "active" => ("Active", "ok"),
        "revoked" => ("Revoked", "err"),
        "expired" => ("Expired", ""),
        "used_up" => ("Used up", "warn"),
        _ => ("Unknown", ""),
    }
}

/// A sign-up that waits.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PendingRow {
    /// its id (for approve and reject)
    pub id: i64,
    /// the account's name
    pub username: String,
    /// its email
    pub email: String,
    /// `verify_email` or `approval`
    pub waiting_for: String,
    /// where it came from
    pub address: String,
    /// when
    pub created_at: f64,
}

impl PendingRow {
    /// The list of `{"pending": […]}`.
    pub fn list(v: &Value) -> Vec<PendingRow> {
        let row = |p: &Value| PendingRow {
            id: p.get("id").and_then(Value::as_i64).unwrap_or(0),
            username: text(p, "username"),
            email: text(p, "email"),
            waiting_for: text(p, "waiting_for"),
            address: text(p, "address"),
            created_at: p.get("created_at").and_then(Value::as_f64).unwrap_or(0.0),
        };
        v.get("pending").and_then(Value::as_array).map(|a| a.iter().map(row).collect()).unwrap_or_default()
    }

    /// What it waits for, in words.
    pub fn waiting_line<'a>(&self, w: &'a SignupWords) -> &'a str {
        if self.waiting_for == "verify_email" {
            w.waiting_email
        } else {
            w.waiting_approval
        }
    }
}

/// A time as its UTC day, `2026-11-05` ("—" for none): the lists' dates where a client has no locale formatting.
pub fn day(ts: f64) -> String {
    if ts <= 0.0 {
        return "—".into();
    }
    // days since 1970 to a civil date (Howard Hinnant's algorithm)
    let z = (ts / 86400.0).floor() as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// A text field ("" when missing).
fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// A true/false field (false when missing).
fn flag(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}

/// A list of texts (empty when missing).
fn strings(v: &Value, k: &str) -> Vec<String> {
    v.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sign_in_line_follows_the_policy() {
        let w = SignupWords::default();
        let p = |v: Value| Policy::from_json(&v);
        assert_eq!(p(json!({ "who": "open" })).sign_in_line(&w), Some(("Create an account".into(), SignInLink::SignUp)));
        assert_eq!(p(json!({ "who": "invite" })).sign_in_line(&w).unwrap().1, SignInLink::None);
        assert_eq!(p(json!({ "who": "closed" })).sign_in_line(&w), None);
        let m = p(json!({ "who": "open", "managed_by": { "name": "Hub", "url": "https://hub.example" } }));
        assert!(m.managed_by.as_ref().unwrap().reachable);
        let (t, l) = m.sign_in_line(&w).unwrap();
        assert!(t.starts_with("Accounts are managed by Hub."));
        assert_eq!(l, SignInLink::Source("https://hub.example".into()));
        assert_eq!(p(json!({ "who": "open", "managed_by": null })).managed_by, None);
    }

    #[test]
    fn the_form_shows_when_it_can_be_used() {
        let w = SignupWords::default();
        let p = |v: Value| Policy::from_json(&v);
        assert_eq!(p(json!({ "who": "closed" })).form(true, &w), Err(w.closed.into()));
        assert_eq!(p(json!({ "who": "invite" })).form(false, &w), Err(w.need_invite.into()));
        assert!(p(json!({ "who": "invite" })).form(true, &w).is_ok());
        assert!(p(json!({ "who": "open" })).form(false, &w).is_ok());
        assert!(p(json!({ "who": "open", "managed_by": { "name": "Hub", "url": "u" } })).form(true, &w).is_err());
        let v = p(json!({ "who": "open", "verify_email": true }));
        assert!(v.email_required(false) && !v.email_required(true));
        assert!(!p(json!({ "who": "invite", "verify_email": true })).email_required(false));
    }

    #[test]
    fn form_checks() {
        let w = SignupWords::default();
        assert_eq!(check_form(" ", "password1", "password1", "", false, &w), Err(w.need_name.into()));
        assert_eq!(check_form("ana", "short", "short", "", false, &w), Err("Use a password of at least 8 characters".into()));
        assert_eq!(check_form("ana", "password1", "password2", "", false, &w), Err(w.mismatch.into()));
        assert!(check_form("ana", "password1", "password1", "", true, &w).is_err());
        assert!(check_form("ana", "password1", "password1", "a@b.co", true, &w).is_ok());
        assert_eq!(
            sign_up_body(" ana ", "pw", " a@b.co", "t"),
            json!({ "username": "ana", "password": "pw", "email": "a@b.co", "invite": "t" })
        );
    }

    #[test]
    fn tokens_and_links() {
        let t = "a".repeat(64);
        assert_eq!(token_from(&format!("https://x.example/#/sign-up?invite={t}")), t);
        assert_eq!(token_from(&format!("  {t}\n")), t);
        assert_eq!(token_from(&format!("/#/sign-up/verify?token={t}&x=1")), t);
        assert_eq!(token_from("hello"), "");
        assert_eq!(full_link("/#/sign-up?invite=1", "https://h.example/"), "https://h.example/#/sign-up?invite=1");
        assert_eq!(full_link("https://a.example/x", "https://h"), "https://a.example/x");
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings::from_json(&json!({ "who": "open", "approval": true, "per_hour": 3, "open_presets": ["a"] }));
        assert_eq!((s.who.as_str(), s.approval, s.verify_email, s.per_hour, s.per_day), ("open", true, false, 3, 50));
        assert_eq!(Settings::from_json(&s.to_json()), s);
        assert_eq!(Settings::from_json(&json!({})), Settings::default());
        let v = json!({ "roles": [{ "id": "m", "label": "Member" }] });
        let roles = choices(&v, "roles");
        assert_eq!((label_of(&roles, "m"), label_of(&roles, "x")), ("Member".into(), "x".into()));
    }

    #[test]
    fn invite_rows() {
        let w = SignupWords::default();
        let v = json!({ "invites": [{ "id": 3, "note": "", "role": "member", "presets": ["a"], "max_uses": 5, "uses": 2,
            "expires_at": 1000.0 + 3.0 * 86400.0, "status": "active", "created_by_name": "admin", "created_at": 10.0,
            "used_by": [{ "username": "ana", "used_at": 20.0 }] }] });
        let r = &InviteRow::list(&v)[0];
        assert_eq!(
            (r.title(&w), r.uses_line(), r.expiry_line(1000.0)),
            ("Invite 3".into(), "2 of 5 uses".into(), "Expires in 3 days".into())
        );
        assert_eq!(r.used_by, vec![("ana".to_string(), 20.0)]);
        assert!(r.can_revoke());
        let one = InviteRow { max_uses: 1, uses: 1, status: "used_up".into(), ..Default::default() };
        assert_eq!((one.uses_line(), one.can_revoke()), ("1 of 1 use".into(), false));
        let open = InviteRow { max_uses: 0, uses: 1, ..Default::default() };
        assert_eq!((open.uses_line(), open.expiry_line(5.0)), ("1 use, no limit".into(), "Never expires".into()));
        let soon = InviteRow { expires_at: 3600.0, ..Default::default() };
        assert_eq!((soon.expiry_line(10.0), soon.expiry_line(4000.0)), ("Expires in 1 hour".into(), "Expired".into()));
        assert_eq!(invite_status("used_up"), ("Used up", "warn"));
        assert_eq!((parse_uses("5"), parse_uses("1"), parse_uses("x")), (Some(5), None, None));
        assert_eq!(InviteForm::default().to_json()["expires_in_hours"], 168);
    }

    #[test]
    fn days() {
        assert_eq!(
            (day(0.0), day(1.0), day(1_793_836_800.0), day(951_782_400.0)),
            ("—".into(), "1970-01-01".into(), "2026-11-05".into(), "2000-02-29".into())
        );
    }

    #[test]
    fn pending_rows() {
        let w = SignupWords::default();
        let l = PendingRow::list(
            &json!({ "pending": [{ "id": 1, "username": "vic", "waiting_for": "verify_email" }, { "id": 2, "waiting_for": "approval" }] }),
        );
        assert_eq!((l[0].waiting_line(&w), l[1].waiting_line(&w)), (w.waiting_email, w.waiting_approval));
    }
}
