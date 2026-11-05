//! The sign-up routes' handlers.

use crate::store::{Invite, InviteUse, NewInvite, NewSignupRequest, SignupRequest};
use crate::{fill, looks_like_email, settings, token, Cfg, Choice, NewAccount, SignupHost, SignupSettings, Who, Words};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use kit_server_core::db::now;
use kit_server_core::{err, Admin, Error, Result};
use serde::Deserialize;
use serde_json::{json, Value};

/// Characters of an invite's note.
const MAX_NOTE: usize = 200;
/// An hour, in seconds.
const HOUR: f64 = 3600.0;

/// The 409 for an app whose accounts another app keeps, with the source's address.
fn managed_error(words: &Words, name: &str, url: &str) -> Error {
    err(409, fill(&words.managed, &[("app", name)])).with(json!({ "managed_by": { "name": name, "url": url } }))
}

/// 409 when another app keeps the accounts.
async fn not_managed(st: &impl SignupHost, words: &Words) -> Result<()> {
    match st.account_source().await {
        Some(src) => Err(managed_error(words, &src.name, &src.url)),
        None => Ok(()),
    }
}

/// A link: the app's address (if known) and `pattern` with the token filled in.
fn link(st: &impl SignupHost, pattern: &str, token: &str) -> String {
    format!("{}{}", st.base_url().trim_end_matches('/'), fill(pattern, &[("token", token)]))
}

/// The public part of a setting.
fn public_policy(s: &SignupSettings) -> Value {
    json!({ "who": s.who, "verify_email": s.verify_email, "approval": s.approval })
}

/// `GET /api/signup`: who can sign up, for the sign-in page (anyone). With an account source, the source's
/// setting and `managed_by`.
pub(crate) async fn policy<S: SignupHost>(State(st): State<S>) -> Result<Json<Value>> {
    if let Some(src) = st.account_source().await {
        let mut p = src.policy.clone().filter(Value::is_object).unwrap_or_else(|| json!({ "who": null }));
        p["managed_by"] = json!({ "name": src.name, "url": src.url, "reachable": src.policy.is_some() });
        return Ok(Json(p));
    }
    let mut p = public_policy(&settings(&st).await?);
    p["managed_by"] = Value::Null;
    Ok(Json(p))
}

/// `POST /api/signup` body.
#[derive(Deserialize)]
pub(crate) struct SignUp {
    /// the name
    username: String,
    /// the password
    password: String,
    /// the mail address
    #[serde(default)]
    email: String,
    /// an invite token
    #[serde(default)]
    invite: String,
}

/// An invite by its token, or the words why it can't be used.
async fn usable_invite(st: &impl SignupHost, words: &Words, token: &str) -> Result<Invite> {
    let inv = st.signup().invite_by_token(&token::hash(token)).await?;
    let Some(inv) = inv else { return Err(err(404, words.invite_unknown.clone())) };
    match inv.unusable(now()) {
        Some("revoked") => Err(err(410, words.invite_revoked.clone())),
        Some("expired") => Err(err(410, words.invite_expired.clone())),
        Some(_) => Err(err(410, words.invite_used_up.clone())),
        None => Ok(inv),
    }
}

/// 429 when this address made too many tries this hour; each call counts as one.
fn tries(st: &impl SignupHost, cfg: &Cfg, address: &str) -> Result<()> {
    if st.limited(&format!("signup-try:{address}"), cfg.tries_per_hour, HOUR) {
        return Err(err(429, cfg.words.too_many.clone()));
    }
    Ok(())
}

/// `POST /api/signup`: make an account with an invite, or an open sign-up within the setting's limits.
pub(crate) async fn sign_up<S: SignupHost>(
    State(st): State<S>,
    cfg: Cfg,
    headers: HeaderMap,
    Json(b): Json<SignUp>,
) -> Result<Json<Value>> {
    let words = &cfg.words;
    not_managed(&st, words).await?;
    let address = st.client_address(&headers);
    tries(&st, &cfg, &address)?;
    let s = settings(&st).await?;
    if s.who == Who::Closed {
        return Err(err(403, words.closed.clone()));
    }
    let email = b.email.trim().to_string();
    if !email.is_empty() && !looks_like_email(&email) {
        return Err(err(400, words.email_bad.clone()));
    }
    let username = b.username.trim().to_string();
    if !b.invite.trim().is_empty() {
        return with_invite(&st, words, &b.invite, username, b.password, email).await;
    }
    if s.who == Who::Invite {
        return Err(err(403, words.invite_only.clone()));
    }
    // open
    if s.verify_email && email.is_empty() {
        return Err(err(400, words.email_needed.clone()));
    }
    if st.limited(&format!("signup:{address}"), s.per_hour.max(1) as usize, HOUR) {
        return Err(err(429, words.too_many.clone()));
    }
    if s.per_day > 0 && st.limited("signup-day", s.per_day as usize, 24.0 * HOUR) {
        return Err(err(429, words.full_today.clone()));
    }
    let roles = st.roles();
    let role = if s.open_role.is_empty() || !roles.iter().any(|r| r.id == s.open_role) {
        roles.first().map(|r| r.id.clone()).unwrap_or_default()
    } else {
        s.open_role.clone()
    };
    let presets = known(&st.presets(), &s.open_presets);
    let waits = s.verify_email || s.approval;
    let account = NewAccount { username: username.clone(), password: b.password, email: email.clone(), role, presets, enabled: !waits };
    let id = st.create_account(account).await?;
    if !waits {
        st.log("info", &format!("Account '{username}' signed up"));
        return Ok(Json(json!({ "status": "active", "message": words.ready, "account_id": id })));
    }
    let tok = s.verify_email.then(token::new_token);
    let req = NewSignupRequest {
        account_id: id,
        username: username.clone(),
        email: email.clone(),
        verify_hash: tok.as_deref().map(token::hash).unwrap_or_default(),
        verify_expires_at: if tok.is_some() { now() + f64::from(cfg.verify_hours) * HOUR } else { 0.0 },
        needs_approval: s.approval,
        address,
    };
    st.signup().add_request(&req).await?;
    if let Some(tok) = tok {
        let app = st.app_name();
        let link = link(&st, &cfg.verify_link, &tok);
        let hours = cfg.verify_hours.to_string();
        let values = [("app", app.as_str()), ("username", username.as_str()), ("link", link.as_str()), ("hours", hours.as_str())];
        let (subject, text) = (fill(&words.verify_subject, &values), fill(&words.verify_text, &values));
        if let Err(e) = st.mailer().send(&email, &subject, &text).await {
            st.log("warning", &format!("Sign-up confirmation mail to {email} failed: {e}"));
        }
        st.log("info", &format!("Account '{username}' signed up and waits for its email"));
        return Ok(Json(json!({ "status": "verify_email", "message": words.waiting_email, "account_id": id })));
    }
    st.log("info", &format!("Account '{username}' signed up and waits for an admin"));
    Ok(Json(json!({ "status": "approval", "message": words.waiting_approval, "account_id": id })))
}

/// The ids of `wanted` the app offers, in its order.
fn known(offered: &[Choice], wanted: &[String]) -> Vec<String> {
    offered.iter().filter(|c| wanted.contains(&c.id)).map(|c| c.id.clone()).collect()
}

/// An invite's sign-up: take one use, make the account (switched on: the admin vouched for it), record who used it.
async fn with_invite(
    st: &impl SignupHost,
    words: &Words,
    tok: &str,
    username: String,
    password: String,
    email: String,
) -> Result<Json<Value>> {
    let inv = usable_invite(st, words, tok).await?;
    if !st.signup().take_invite(inv.id, now()).await? {
        // revoked or used up between the check and now
        return Err(err(410, words.invite_used_up.clone()));
    }
    let account =
        NewAccount { username: username.clone(), password, email, role: inv.role.clone(), presets: inv.presets.clone(), enabled: true };
    let id = match st.create_account(account).await {
        Ok(id) => id,
        Err(e) => {
            st.signup().give_back_invite(inv.id).await?;
            return Err(e);
        }
    };
    st.signup().record_use(&InviteUse { invite_id: inv.id, account_id: id, username: username.clone(), used_at: now() }).await?;
    st.log("info", &format!("Account '{username}' signed up with invite {}", inv.id));
    Ok(Json(json!({ "status": "active", "message": words.ready, "account_id": id })))
}

/// `POST /api/signup/invite` and `/verify` body.
#[derive(Deserialize)]
pub(crate) struct TokenBody {
    /// the token from the link
    token: String,
}

/// `POST /api/signup/invite`: whether an invite link works, and what it gives (anyone; counts as a try).
pub(crate) async fn check_invite<S: SignupHost>(
    State(st): State<S>,
    cfg: Cfg,
    headers: HeaderMap,
    Json(b): Json<TokenBody>,
) -> Result<Json<Value>> {
    not_managed(&st, &cfg.words).await?;
    tries(&st, &cfg, &st.client_address(&headers))?;
    if settings(&st).await?.who == Who::Closed {
        return Err(err(403, cfg.words.closed.clone()));
    }
    let inv = usable_invite(&st, &cfg.words, &b.token).await?;
    let label = |list: &[Choice], id: &str| list.iter().find(|c| c.id == id).map(|c| c.label.clone()).unwrap_or_else(|| id.to_string());
    let presets = st.presets();
    Ok(Json(json!({
        "ok": true,
        "role": label(&st.roles(), &inv.role),
        "presets": inv.presets.iter().map(|p| label(&presets, p)).collect::<Vec<_>>(),
        "expires_at": inv.expires_at,
    })))
}

/// Switch the account of `r` on and forget it when nothing is left to wait for; its status.
async fn settle(st: &impl SignupHost, r: &SignupRequest) -> Result<&'static str> {
    let Some(r) = st.signup().request(r.id).await? else { return Ok("active") };
    match r.waiting_for() {
        Some(w) => Ok(w),
        None => {
            st.enable_account(r.account_id).await?;
            st.signup().remove_request(r.id).await?;
            st.log("info", &format!("Account '{}' can sign in now", r.username));
            Ok("active")
        }
    }
}

/// The words of a status.
fn status_words<'a>(words: &'a Words, status: &str) -> &'a str {
    match status {
        "verify_email" => &words.waiting_email,
        "approval" => &words.waiting_approval,
        _ => &words.ready,
    }
}

/// `POST /api/signup/verify`: the confirmation mail's link (anyone; counts as a try).
pub(crate) async fn verify<S: SignupHost>(
    State(st): State<S>,
    cfg: Cfg,
    headers: HeaderMap,
    Json(b): Json<TokenBody>,
) -> Result<Json<Value>> {
    tries(&st, &cfg, &st.client_address(&headers))?;
    let r = st.signup().request_by_token(&token::hash(&b.token)).await?;
    let Some(r) = r.filter(|r| r.verify_expires_at == 0.0 || r.verify_expires_at > now()) else {
        return Err(err(400, cfg.words.verify_bad.clone()));
    };
    st.signup().verify_request(r.id).await?;
    let status = settle(&st, &r).await?;
    Ok(Json(json!({ "status": status, "message": status_words(&cfg.words, status) })))
}

/// The admin's view of the setting, with what the forms offer.
async fn settings_answer(st: &impl SignupHost) -> Result<Value> {
    let (managed_by, read_only, current) = match st.account_source().await {
        Some(src) => {
            let s = src.policy.clone().and_then(|p| serde_json::from_value::<SignupSettings>(p).ok()).unwrap_or_default();
            (json!({ "name": src.name, "url": src.url, "reachable": src.policy.is_some() }), true, s)
        }
        None => (Value::Null, false, settings(st).await?),
    };
    Ok(json!({
        "settings": current,
        "roles": st.roles(),
        "presets": st.presets(),
        "mail_delivers": st.mailer().delivers(),
        "managed_by": managed_by,
        "read_only": read_only,
    }))
}

/// `GET /api/signup/settings` (admin).
pub(crate) async fn get_settings<S: SignupHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    Ok(Json(settings_answer(&st).await?))
}

/// `PUT /api/signup/settings` (admin): the whole setting; 409 when another app keeps the accounts.
pub(crate) async fn put_settings<S: SignupHost>(
    State(st): State<S>,
    cfg: Cfg,
    Admin(u): Admin,
    Json(b): Json<Value>,
) -> Result<Json<Value>> {
    not_managed(&st, &cfg.words).await?;
    let mut s: SignupSettings = serde_json::from_value(b).map_err(|e| err(400, format!("The sign-up setting isn't valid: {e}")))?;
    if !s.open_role.is_empty() && !st.roles().iter().any(|r| r.id == s.open_role) {
        return Err(err(400, format!("There is no role '{}'", s.open_role)));
    }
    let presets = st.presets();
    if let Some(p) = s.open_presets.iter().find(|p| !presets.iter().any(|c| &c.id == *p)) {
        return Err(err(400, format!("There is no preset '{p}'")));
    }
    s.per_hour = s.per_hour.clamp(1, 1000);
    st.signup().save_settings(&serde_json::to_value(&s).unwrap_or_default()).await?;
    st.log("info", &format!("{} set who can sign up to {:?}", u.username, s.who));
    Ok(Json(settings_answer(&st).await?))
}

/// An invite as the admin sees it: with its status and who used it.
async fn invite_json(st: &impl SignupHost, inv: &Invite) -> Result<Value> {
    let mut v = serde_json::to_value(inv).unwrap_or_default();
    v["status"] = json!(inv.status(now()));
    v["used_by"] = json!(st.signup().uses(inv.id).await?);
    Ok(v)
}

/// `GET /api/signup/invites` (admin): every invite, newest first.
pub(crate) async fn list_invites<S: SignupHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let mut out = Vec::new();
    for inv in st.signup().invites().await? {
        out.push(invite_json(&st, &inv).await?);
    }
    Ok(Json(json!({ "invites": out })))
}

/// `POST /api/signup/invites` body.
#[derive(Deserialize)]
pub(crate) struct NewInviteBody {
    /// the admin's note
    #[serde(default)]
    note: String,
    /// the role; "" for the app's first
    #[serde(default)]
    role: String,
    /// the presets
    #[serde(default)]
    presets: Vec<String>,
    /// 1 for one-time (the default), 0 for no limit
    #[serde(default = "one")]
    max_uses: i64,
    /// hours until it stops working; 0 (the default) for never
    #[serde(default)]
    expires_in_hours: f64,
}

/// 1.
fn one() -> i64 {
    1
}

/// `POST /api/signup/invites` (admin): a new invite; its token and link are shown this once.
pub(crate) async fn create_invite<S: SignupHost>(
    State(st): State<S>,
    cfg: Cfg,
    Admin(u): Admin,
    Json(b): Json<NewInviteBody>,
) -> Result<Json<Value>> {
    not_managed(&st, &cfg.words).await?;
    let roles = st.roles();
    let role = if b.role.trim().is_empty() { roles.first().map(|r| r.id.clone()).unwrap_or_default() } else { b.role.trim().to_string() };
    if !roles.iter().any(|r| r.id == role) {
        return Err(err(400, format!("There is no role '{role}'")));
    }
    let presets = st.presets();
    if let Some(p) = b.presets.iter().find(|p| !presets.iter().any(|c| &c.id == *p)) {
        return Err(err(400, format!("There is no preset '{p}'")));
    }
    if !(0..=100_000).contains(&b.max_uses) {
        return Err(err(400, "Uses must be 0 (no limit) to 100000"));
    }
    if !b.expires_in_hours.is_finite() || !(0.0..=24.0 * 3650.0).contains(&b.expires_in_hours) {
        return Err(err(400, "The expiry must be 0 (never) to ten years, in hours"));
    }
    let tok = token::new_token();
    let new = NewInvite {
        token_hash: token::hash(&tok),
        note: b.note.trim().chars().take(MAX_NOTE).collect(),
        role,
        presets: known(&presets, &b.presets),
        max_uses: b.max_uses,
        expires_at: if b.expires_in_hours > 0.0 { now() + b.expires_in_hours * HOUR } else { 0.0 },
        created_by: u.id,
        created_by_name: u.username.clone(),
    };
    let id = st.signup().create_invite(&new).await?;
    let inv = st.signup().invite(id).await?.ok_or_else(|| err(500, "The invite was not saved"))?;
    st.log("info", &format!("{} made invite {id}", u.username));
    Ok(Json(json!({ "invite": invite_json(&st, &inv).await?, "token": tok, "link": link(&st, &cfg.invite_link, &tok) })))
}

/// Invite `id`, or 404.
async fn find_invite(st: &impl SignupHost, id: i64) -> Result<Invite> {
    st.signup().invite(id).await?.ok_or_else(|| err(404, "There is no such invite"))
}

/// `GET /api/signup/invites/{id}` (admin).
pub(crate) async fn get_invite<S: SignupHost>(State(st): State<S>, Admin(_): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    let inv = find_invite(&st, id).await?;
    Ok(Json(json!({ "invite": invite_json(&st, &inv).await? })))
}

/// `POST /api/signup/invites/{id}/revoke` (admin): the invite stops working at once; its record and uses stay.
pub(crate) async fn revoke_invite<S: SignupHost>(State(st): State<S>, Admin(u): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    if !st.signup().revoke_invite(id, now()).await? {
        return Err(err(404, "There is no such invite"));
    }
    st.log("info", &format!("{} revoked invite {id}", u.username));
    let inv = find_invite(&st, id).await?;
    Ok(Json(json!({ "invite": invite_json(&st, &inv).await? })))
}

/// `GET /api/signup/pending` (admin): the sign-ups that wait, oldest first.
pub(crate) async fn list_pending<S: SignupHost>(State(st): State<S>, Admin(_): Admin) -> Result<Json<Value>> {
    let list: Vec<Value> = st
        .signup()
        .requests()
        .await?
        .iter()
        .map(|r| {
            let mut v = serde_json::to_value(r).unwrap_or_default();
            v["waiting_for"] = json!(r.waiting_for());
            v
        })
        .collect();
    Ok(Json(json!({ "pending": list })))
}

/// A waiting sign-up, or 404.
async fn find_request(st: &impl SignupHost, id: i64) -> Result<SignupRequest> {
    st.signup().request(id).await?.ok_or_else(|| err(404, "There is no such sign-up waiting"))
}

/// `POST /api/signup/pending/{id}/approve` (admin): let it in. An admin's yes also stands for the email: they
/// chose to let this account in.
pub(crate) async fn approve<S: SignupHost>(State(st): State<S>, Admin(u): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    let r = find_request(&st, id).await?;
    st.signup().approve_request(id).await?;
    if !r.email_verified {
        st.signup().verify_request(id).await?;
    }
    let status = settle(&st, &r).await?;
    st.log("info", &format!("{} let '{}' in", u.username, r.username));
    Ok(Json(json!({ "ok": true, "status": status })))
}

/// `POST /api/signup/pending/{id}/reject` (admin): turn it away; the waiting account (never signed in) is removed.
pub(crate) async fn reject<S: SignupHost>(State(st): State<S>, Admin(u): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    let r = find_request(&st, id).await?;
    st.remove_account(r.account_id).await?;
    st.signup().remove_request(id).await?;
    st.log("info", &format!("{} turned '{}' away", u.username, r.username));
    Ok(Json(json!({ "ok": true })))
}
