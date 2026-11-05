//! First-run setup: the first person to open a new server makes its owner account on a "Create your account"
//! screen, instead of the server making one with a password printed in its log.
//!
//! So that nobody else on the network can claim a new server first, the screen asks for a **setup code**: a
//! one-time code the server makes at start and prints in its log ([`SetupCode::at_start`]). Only someone who can
//! read the server's log (who installed it) knows it. The code stops working the moment the owner exists, and a
//! restart before then prints a new one. Wrong codes are limited per address and in all ([`SetupConfig`]).
//!
//! An unattended install still makes its owner from settings at start (in the template: `APP_ADMIN_USER` and
//! `APP_ADMIN_PASSWORD`); the app then passes [`SetupCode::closed`] and the screen never shows.
//!
//! ```text
//! GET  /api/setup  → {needed}                                        (anyone: show the screen?)
//! POST /api/setup  {code, username, password, email?} → {ok, username} (anyone, with the code; then sign in)
//! ```
//!
//! The app keeps its accounts: the kit never reads or writes its tables. It asks the app whether there is an
//! owner ([`SetupHost::has_owner`]) and to make one ([`SetupHost::create_owner`]); the app checks the name and
//! password with its own rules and words. The clients sign in with the new name and password once it is made.
//!
//! An app adds the routes with `.merge(kit_server_setup::router())` or `router_with(SetupConfig { … })`.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Extension, Json, Router};
use kit_server_core::{err, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

/// The letters a setup code is made of: digits and capitals without the ones that look alike (0 O 1 I L U).
pub const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters in a setup code (shown in groups of four: `ABCD-EFGH-JKMN`, about 59 bits).
pub const CODE_LEN: usize = 12;

/// Seconds since 1970.
fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// `typed` as a code is compared: capitals, with spaces, dashes and anything else that isn't a letter or digit
/// left out (so `abcd efgh-jkmn` is `ABCDEFGHJKMN`).
pub fn normalize(typed: &str) -> String {
    typed.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

/// `code` in groups of four, joined by dashes, as people read it.
pub fn grouped(code: &str) -> String {
    code.as_bytes().chunks(4).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join("-")
}

/// Whether `a` and `b` are equal, in time that does not depend on where they differ.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// A new random code of [`CODE_LEN`] characters of [`ALPHABET`].
fn new_code() -> String {
    let mut out = String::with_capacity(CODE_LEN);
    // rejection sampling: only bytes below the largest multiple of the alphabet's size, so every letter is as likely
    let limit = (256 / ALPHABET.len() * ALPHABET.len()) as u8;
    while out.len() < CODE_LEN {
        let mut b = [0u8; 32];
        getrandom::fill(&mut b).expect("the system's random numbers");
        for x in b {
            if x < limit && out.len() < CODE_LEN {
                out.push(ALPHABET[x as usize % ALPHABET.len()] as char);
            }
        }
    }
    out
}

/// The server's setup code: open (a code that makes the owner) until the owner exists, or closed.
#[derive(Default)]
pub struct SetupCode {
    /// the code, without dashes; `None` once closed (or never opened)
    code: Mutex<Option<String>>,
    /// the times of recent wrong codes, per address
    wrong: Mutex<HashMap<String, Vec<f64>>>,
}

impl std::fmt::Debug for SetupCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // never the code itself: a debug print could end up in a log or a bug report
        f.debug_struct("SetupCode").field("open", &self.is_open()).finish()
    }
}

impl SetupCode {
    /// A new, random, open code.
    pub fn new() -> SetupCode {
        SetupCode::with_code(&new_code())
    }

    /// An open code that is `code` (tests, or an app that shows its own).
    pub fn with_code(code: &str) -> SetupCode {
        SetupCode { code: Mutex::new(Some(normalize(code))), wrong: Mutex::default() }
    }

    /// No setup: the owner exists already, or the app makes it from its settings.
    pub fn closed() -> SetupCode {
        SetupCode::default()
    }

    /// The code at start: when `needed` (no owner, and none made from settings) a new one, printed in the log
    /// with [`SetupCode::banner`] so whoever installed the server can read it; otherwise [`SetupCode::closed`].
    pub fn at_start(needed: bool, app: &str) -> SetupCode {
        let code = SetupCode::closed();
        if needed {
            code.open(app);
        }
        code
    }

    /// Opens setup with a new random code and prints it in the log ([`SetupCode::banner`]), for an app that keeps
    /// its code in its state from the start and finds out later that it has no owner.
    pub fn open(&self, app: &str) {
        *self.code.lock().unwrap_or_else(|e| e.into_inner()) = Some(new_code());
        if let Some(b) = self.banner(app) {
            // stdout, like the rest of the server's log, so `docker logs` shows it
            println!("{b}");
        }
        tracing::info!("{app} has no account yet: first-run setup is open, with the setup code printed above.");
    }

    /// Whether the code still works (no owner yet).
    pub fn is_open(&self) -> bool {
        self.code.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// The code as people read it (`ABCD-EFGH-JKMN`); `None` once closed.
    pub fn code(&self) -> Option<String> {
        self.code.lock().unwrap_or_else(|e| e.into_inner()).as_deref().map(grouped)
    }

    /// The code stops working: the owner exists.
    pub fn close(&self) {
        *self.code.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// The lines the log shows at start, with the code; `None` once closed.
    pub fn banner(&self, app: &str) -> Option<String> {
        let code = self.code()?;
        let rule = "=".repeat(70);
        Some(format!(
            "{rule}\n{app} has no account yet. Open it in a browser or its app, and create your account\n\
             with this one-time setup code:\n\n    {code}\n\n\
             The code works until your account exists. A restart before then prints a new one.\n{rule}"
        ))
    }

    /// Whether `address` (or everyone together) had too many wrong codes in the last `window` seconds.
    fn limited(&self, address: &str, cfg: &SetupConfig) -> bool {
        let t = now();
        let mut m = self.wrong.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = 0;
        for v in m.values_mut() {
            v.retain(|&at| t - at < cfg.window_secs);
            all += v.len();
        }
        m.retain(|_, v| !v.is_empty());
        m.get(address).map_or(0, Vec::len) >= cfg.tries_per_address || all >= cfg.tries_in_all
    }

    /// Count one wrong code from `address` now.
    fn record_wrong(&self, address: &str) {
        self.wrong.lock().unwrap_or_else(|e| e.into_inner()).entry(address.to_string()).or_default().push(now());
    }

    /// Takes the code out when `typed` is it, so only one owner can be made with it at a time; `None` when it
    /// isn't (or the code is closed). The [`Claim`] puts it back unless [`Claim::done`].
    fn claim(&self, typed: &str) -> Option<Claim<'_>> {
        let mut c = self.code.lock().unwrap_or_else(|e| e.into_inner());
        let ok = c.as_deref().is_some_and(|code| same(code, &normalize(typed)));
        ok.then(|| Claim { owner: self, code: c.take() })
    }
}

/// The code while the owner is being made: put back when that fails (a name taken, a short password), so the
/// person can try again.
struct Claim<'a> {
    /// the code it came from
    owner: &'a SetupCode,
    /// the code taken out; `None` once done
    code: Option<String>,
}

impl Claim<'_> {
    /// The owner was made: the code stays closed.
    fn done(mut self) {
        self.code = None;
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        if let Some(code) = self.code.take() {
            let mut c = self.owner.code.lock().unwrap_or_else(|e| e.into_inner());
            if c.is_none() {
                *c = Some(code);
            }
        }
    }
}

/// The owner to make, as typed on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOwner {
    /// the name (trimmed; the app checks it)
    pub username: String,
    /// the password (the app checks and hashes it)
    pub password: String,
    /// the mail address; may be empty
    pub email: String,
}

/// The words people see. Each has a plain English default.
#[derive(Debug, Clone, PartialEq)]
pub struct Words {
    /// a setup when the server already has its owner
    pub done_already: String,
    /// a setup while the server has no setup code (it makes its owner from its settings)
    pub closed: String,
    /// no code typed
    pub need_code: String,
    /// a wrong code
    pub wrong_code: String,
    /// too many wrong codes
    pub too_many: String,
    /// an email that doesn't look like one
    pub email_bad: String,
}

impl Default for Words {
    fn default() -> Self {
        Words {
            done_already: "This server already has its owner. Sign in instead.".into(),
            closed: "This server doesn't take a setup code. Its owner is made from its settings: sign in with them.".into(),
            need_code: "Enter the setup code from the server's log.".into(),
            wrong_code: "That setup code isn't right. Copy it from the server's log: it is printed each time the server starts.".into(),
            too_many: "Too many wrong setup codes. Wait 15 minutes, or restart the server to get a new code.".into(),
            email_bad: "That email address doesn't look right.".into(),
        }
    }
}

/// Everything about setup an app may change: where it is served, the limits and the words.
#[derive(Debug, Clone, PartialEq)]
pub struct SetupConfig {
    /// the route (default `/api/setup`)
    pub base: String,
    /// wrong codes one address may send in the window (default 10)
    pub tries_per_address: usize,
    /// wrong codes everyone together may send in the window, so a spoofed address doesn't help (default 50)
    pub tries_in_all: usize,
    /// the window, in seconds (default 15 minutes)
    pub window_secs: f64,
    /// the words people see
    pub words: Words,
}

impl Default for SetupConfig {
    fn default() -> Self {
        SetupConfig { base: "/api/setup".into(), tries_per_address: 10, tries_in_all: 50, window_secs: 900.0, words: Words::default() }
    }
}

/// What setup needs from the app: its setup code, whether it has an owner, and how to make one.
pub trait SetupHost: Clone + Send + Sync + 'static {
    /// The setup code the app made at start ([`SetupCode::at_start`]) and keeps in its state.
    fn setup_code(&self) -> &SetupCode;

    /// Whether the app has its owner account.
    fn has_owner(&self) -> impl Future<Output = Result<bool>> + Send;

    /// Make the owner. The app checks the name and password (answering 400 / 409 with its own words) and hashes
    /// the password. Once this succeeds the setup code stops working.
    fn create_owner(&self, owner: NewOwner) -> impl Future<Output = Result<()>> + Send;

    /// The address a request came from, for the limits. Default: the first `X-Forwarded-For` hop, else "local".
    fn client_address(&self, headers: &HeaderMap) -> String {
        headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "local".into())
    }

    /// A line for the app's activity log (`info` or `warning`). The default writes it to the tracing log.
    fn log(&self, level: &str, message: &str) {
        match level {
            "warning" => tracing::warn!("{message}"),
            _ => tracing::info!("{message}"),
        }
    }
}

/// The setup route with the default path, limits and words.
pub fn router<S: SetupHost>() -> Router<S> {
    router_with(SetupConfig::default())
}

/// The setup route with the app's path, limits and words.
pub fn router_with<S: SetupHost>(config: SetupConfig) -> Router<S> {
    let base = config.base.trim_end_matches('/').to_string();
    Router::new().route(&base, get(status::<S>).post(create::<S>)).layer(Extension(Arc::new(config)))
}

/// Whether the screen shows: no owner yet and the code is open.
pub async fn needed(st: &impl SetupHost) -> Result<bool> {
    Ok(st.setup_code().is_open() && !st.has_owner().await?)
}

/// `GET /api/setup`: whether the "Create your account" screen shows (anyone). It never gives the code.
async fn status<S: SetupHost>(State(st): State<S>) -> Result<Json<Value>> {
    Ok(Json(json!({ "needed": needed(&st).await? })))
}

/// `POST /api/setup` body.
#[derive(Deserialize)]
struct Create {
    /// the setup code from the log
    #[serde(default)]
    code: String,
    /// the owner's name
    #[serde(default)]
    username: String,
    /// the owner's password
    #[serde(default)]
    password: String,
    /// the owner's mail address
    #[serde(default)]
    email: String,
}

/// Whether `email` looks like a mail address: one `@`, something before it, a dot after it, no spaces.
fn looks_like_email(email: &str) -> bool {
    let Some((user, domain)) = email.split_once('@') else { return false };
    email.len() <= 254
        && !user.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// `POST /api/setup`: make the owner with the setup code (anyone). 409 once there is an owner, 403 for a wrong
/// code, 429 after too many; the app's 400 / 409 for a name or password it doesn't take (the code then still works).
async fn create<S: SetupHost>(
    State(st): State<S>,
    Extension(cfg): Extension<Arc<SetupConfig>>,
    headers: HeaderMap,
    Json(b): Json<Create>,
) -> Result<Json<Value>> {
    let w = &cfg.words;
    let code = st.setup_code();
    if st.has_owner().await? {
        code.close();
        return Err(err(409, w.done_already.clone()));
    }
    if !code.is_open() {
        return Err(err(409, w.closed.clone()));
    }
    let address = st.client_address(&headers);
    if code.limited(&address, &cfg) {
        return Err(err(429, w.too_many.clone()));
    }
    if normalize(&b.code).is_empty() {
        return Err(err(400, w.need_code.clone()));
    }
    let email = b.email.trim().to_string();
    if !email.is_empty() && !looks_like_email(&email) {
        return Err(err(400, w.email_bad.clone()));
    }
    let Some(claim) = code.claim(&b.code) else {
        code.record_wrong(&address);
        st.log("warning", &format!("First-run setup: a wrong setup code from {address}."));
        // the code may have been closed by a setup that finished just now
        if !code.is_open() && st.has_owner().await? {
            return Err(err(409, w.done_already.clone()));
        }
        return Err(err(403, w.wrong_code.clone()));
    };
    let username = b.username.trim().to_string();
    st.create_owner(NewOwner { username: username.clone(), password: b.password, email }).await?;
    claim.done();
    st.log("info", &format!("First-run setup: the owner account '{username}' was made from {address}."));
    Ok(Json(json!({ "ok": true, "username": username })))
}

#[cfg(test)]
mod tests;
