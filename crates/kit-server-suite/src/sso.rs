//! The linked app's side of single sign-on: people sign in through the link marked as the account source.
//!
//! - `GET /api/kit/sso` (public): `{enabled, source: {name, icon, base_url}, start}`, so a sign-in page can
//!   offer "Sign in with …".
//! - `GET /api/kit/sso/start?next=/path`: off to the account source's authorization endpoint (from its
//!   discovery document) with a fresh `state` and `nonce`.
//! - `GET /api/kit/sso/callback?code&state`: the code for an ID token (checked: signature with the key's
//!   secret, issuer, audience, expiry, nonce), then [`crate::SuiteHost::sso_sign_in`] makes the app's own
//!   session. A failure goes back to the sign-in page with the reason in `sso_error`.

use crate::{crypto, remote, Pending, SsoUser, SuiteHost};
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use kit_rules::suite::{encode, fill, join};
use kit_server_core::{err, Result};
use serde::Deserialize;
use serde_json::{json, Value};

/// Where this app takes the account source's answer.
pub const CALLBACK_PATH: &str = "/api/kit/sso/callback";

/// The routes of the linked app.
pub(crate) fn router<S: SuiteHost>() -> Router<S> {
    Router::new()
        .route("/api/kit/sso", get(info::<S>))
        .route("/api/kit/sso/start", get(start::<S>))
        .route(CALLBACK_PATH, get(callback::<S>))
}

/// `GET /api/kit/sso` (public): whether people can sign in through an account source, and which.
async fn info<S: SuiteHost>(State(st): State<S>) -> Result<Json<Value>> {
    let source = st.suite().account_source(st.suite_store()).await?;
    Ok(Json(match source {
        Some(l) => {
            let m = l.manifest.clone().unwrap_or_default();
            json!({ "enabled": true, "source": { "name": l.name(), "icon": m.icon, "base_url": l.base_url }, "start": "/api/kit/sso/start" })
        }
        None => json!({ "enabled": false, "source": null, "start": null }),
    }))
}

/// The query of a sign-in start.
#[derive(Debug, Deserialize)]
struct StartQuery {
    /// where to go after
    next: Option<String>,
}

/// The account source's discovery document.
async fn discover<S: SuiteHost>(st: &S, issuer: &str) -> std::result::Result<Value, String> {
    let url = join(issuer, "/.well-known/openid-configuration");
    remote::json(st.suite().http().get(&url), &url).await
}

/// `GET /api/kit/sso/start?next=`: off to the account source to sign in.
async fn start<S: SuiteHost>(State(st): State<S>, headers: HeaderMap, Query(q): Query<StartQuery>) -> Result<Response> {
    let next = crate::local_path(q.next.as_deref().unwrap_or("/"));
    let link = st.suite().account_source(st.suite_store()).await?.ok_or_else(|| err(404, "No account source is linked to this app."))?;
    let (client_id, _) =
        crypto::split_key(&link.api_key).ok_or_else(|| err(500, "The account source's key can't be read. Link it again."))?;
    let issuer = link.manifest.as_ref().and_then(|m| m.sso.as_ref()).map(|s| s.issuer.clone()).unwrap_or_else(|| link.base_url.clone());
    let doc = discover(&st, &issuer).await.map_err(|e| err(502, e))?;
    let endpoint =
        doc.get("authorization_endpoint").and_then(Value::as_str).ok_or_else(|| err(502, format!("{issuer} gave no place to sign in.")))?;
    let state = crypto::random_token(24);
    let nonce = crypto::random_token(24);
    let redirect = join(&st.suite().base_url(&headers), CALLBACK_PATH);
    st.suite().mem().pending.insert(state.clone(), Pending { nonce: nonce.clone(), next, link: link.id, at: kit_server_core::db::now() });
    let sep = if endpoint.contains('?') { '&' } else { '?' };
    let url = format!(
        "{endpoint}{sep}response_type=code&scope=openid%20profile%20email&client_id={}&redirect_uri={}&state={}&nonce={}",
        encode(client_id),
        encode(&redirect),
        encode(&state),
        encode(&nonce)
    );
    Ok(Redirect::to(&url).into_response())
}

/// The query of the account source's answer.
#[derive(Debug, Deserialize)]
struct CallbackQuery {
    /// the code, when it worked
    code: Option<String>,
    /// the state this app sent
    #[serde(default)]
    state: String,
    /// the provider's error, when it did not
    error: Option<String>,
}

/// Back to the sign-in page with the reason (`sso_error`).
fn fail<S: SuiteHost>(st: &S, next: &str, why: &str) -> Response {
    let login = fill(&st.suite().config().login_path, &[("next", next)]);
    let sep = if login.contains('?') { '&' } else { '?' };
    Redirect::to(&format!("{login}{sep}sso_error={}", encode(why))).into_response()
}

/// Whether the token's `aud` is `client_id` (a string, or an array holding it).
fn audience_ok(aud: Option<&Value>, client_id: &str) -> bool {
    match aud {
        Some(Value::String(s)) => s == client_id,
        Some(Value::Array(a)) => a.iter().any(|v| v.as_str() == Some(client_id)),
        _ => false,
    }
}

/// `GET /api/kit/sso/callback`: the account source's answer; the app's session, or the sign-in page with the
/// reason.
async fn callback<S: SuiteHost>(State(st): State<S>, headers: HeaderMap, Query(q): Query<CallbackQuery>) -> Response {
    let pending = st.suite().mem().pending.remove(&q.state);
    let Some(p) = pending else {
        return fail(&st, "/", "The sign-in took too long or was already used. Try again.");
    };
    if let Some(e) = q.error {
        return fail(&st, &p.next, &format!("The account source said no ({e})."));
    }
    let Some(code) = q.code else { return fail(&st, &p.next, "The account source sent no sign-in code. Try again.") };
    match finish(&st, &headers, &p, &code).await {
        Ok(user) => match st.sso_sign_in(user, p.next.clone()).await {
            Ok(r) => r,
            Err(e) => fail(&st, &p.next, &e.detail),
        },
        Err(why) => fail(&st, &p.next, &why),
    }
}

/// The code for the person the account source vouches for.
async fn finish<S: SuiteHost>(st: &S, headers: &HeaderMap, p: &Pending, code: &str) -> std::result::Result<SsoUser, String> {
    let link = st.suite_store().link(p.link).await.map_err(|e| e.detail)?.filter(|l| l.account_source);
    let link = link.ok_or("The account source was unlinked during the sign-in.")?;
    let (client_id, secret) = crypto::split_key(&link.api_key).ok_or("The account source's key can't be read. Link it again.")?;
    let issuer = link.manifest.as_ref().and_then(|m| m.sso.as_ref()).map(|s| s.issuer.clone()).unwrap_or_else(|| link.base_url.clone());
    let doc = discover(st, &issuer).await?;
    let endpoint = doc.get("token_endpoint").and_then(Value::as_str).ok_or(format!("{issuer} gave no place to finish signing in."))?;
    let redirect = join(&st.suite().base_url(headers), CALLBACK_PATH);
    let form = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", &redirect),
        ("client_id", client_id),
        ("client_secret", secret),
    ];
    let tokens = remote::json(st.suite().http().post(endpoint).form(&form), endpoint).await?;
    let id_token = tokens.get("id_token").and_then(Value::as_str).ok_or("The account source sent no ID token.")?;
    let claims = crypto::verify(id_token, secret).ok_or("The account source's ID token is not signed with this app's key.")?;
    let s = |k: &str| claims.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    if s("iss") != issuer {
        return Err(format!("The ID token comes from {}, not {issuer}.", s("iss")));
    }
    if !audience_ok(claims.get("aud"), client_id) {
        return Err("The ID token is for another app.".into());
    }
    if claims.get("exp").and_then(Value::as_f64).unwrap_or(0.0) < kit_server_core::db::now() {
        return Err("The ID token is too old. Sign in again.".into());
    }
    if s("nonce") != p.nonce {
        return Err("The ID token is not the one this sign-in asked for. Sign in again.".into());
    }
    if s("sub").is_empty() {
        return Err("The ID token names nobody.".into());
    }
    let b = |k: &str| claims.get(k).and_then(Value::as_bool).unwrap_or(false);
    let username = Some(s("preferred_username")).filter(|u| !u.is_empty()).unwrap_or_else(|| s("sub"));
    Ok(SsoUser {
        issuer,
        subject: s("sub"),
        name: Some(s("name")).filter(|n| !n.is_empty()).unwrap_or_else(|| username.clone()),
        username,
        email: Some(s("email")).filter(|e| !e.is_empty()),
        admin: b("kit_admin"),
        owner: b("kit_owner"),
        claims,
    })
}
