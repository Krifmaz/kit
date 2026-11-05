//! The account source's side of single sign-on: a small OpenID Connect provider (authorization code flow).
//!
//! - `GET /.well-known/openid-configuration`: the discovery document.
//! - `GET /api/kit/oidc/authorize`: a signed-in person is sent back to the linked app with a code; anyone else
//!   goes to the app's sign-in page first ([`crate::SuiteConfig::login_path`]) and comes back here after.
//!   The client is a key this app made, and the key's `redirect_url` says where people may be sent back.
//!   Linked apps are trusted by the admin who made the key, so no consent page is shown.
//! - `POST /api/kit/oidc/token`: the code for an access token and an ID token (HS256, signed with the key's
//!   secret as the client sent it), client authentication `client_secret_post` or `client_secret_basic`.
//! - `GET /api/kit/oidc/userinfo`: the claims for an access token.
//!
//! Every route answers 404 unless the app is an account source.

use crate::{crypto, Grant, SuiteHost};
use axum::extract::{Query, State};
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use base64::Engine;
use kit_rules::suite::{encode, fill, join};
use kit_server_core::{err, Account, Result};
use serde::Deserialize;
use serde_json::{json, Value};

/// The routes of the account source.
pub(crate) fn router<S: SuiteHost>() -> Router<S> {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery::<S>))
        .route("/api/kit/oidc/authorize", get(authorize::<S>))
        .route("/api/kit/oidc/token", post(token::<S>))
        .route("/api/kit/oidc/userinfo", get(userinfo::<S>))
}

/// 404 unless the app is an account source.
fn enabled<S: SuiteHost>(st: &S) -> Result<()> {
    if !st.suite().config().account_source {
        return Err(err(404, "This app is not an account source."));
    }
    Ok(())
}

/// `GET /.well-known/openid-configuration`: where the provider's endpoints are and what it supports.
async fn discovery<S: SuiteHost>(State(st): State<S>, headers: HeaderMap) -> Result<Json<Value>> {
    enabled(&st)?;
    let base = st.suite().base_url(&headers);
    Ok(Json(json!({
        "issuer": base,
        "authorization_endpoint": join(&base, "/api/kit/oidc/authorize"),
        "token_endpoint": join(&base, "/api/kit/oidc/token"),
        "userinfo_endpoint": join(&base, "/api/kit/oidc/userinfo"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["HS256"],
        "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
        "scopes_supported": ["openid", "profile", "email"],
        "claims_supported": ["sub", "iss", "aud", "exp", "iat", "nonce", "preferred_username", "name", "email", "kit_admin", "kit_owner"],
    })))
}

/// The query of an authorization request.
#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    /// must be `code`
    #[serde(default)]
    response_type: String,
    /// the key's id
    #[serde(default)]
    client_id: String,
    /// where to send the code
    #[serde(default)]
    redirect_uri: String,
    /// handed back as is
    state: Option<String>,
    /// put in the ID token
    nonce: Option<String>,
}

/// Whether `uri` is under the key's `redirect_url` (the same address, or a path below it).
fn redirect_allowed(uri: &str, base: &str) -> bool {
    uri == base || uri.strip_prefix(base).is_some_and(|rest| rest.starts_with('/') || rest.starts_with('?'))
}

/// `GET /api/kit/oidc/authorize`: a code for the signed-in person, sent to the linked app; the sign-in page
/// first for anyone not signed in. 400 (no redirect) for an unknown client or a redirect it may not use.
async fn authorize<S: SuiteHost>(State(st): State<S>, mut parts: Parts, Query(q): Query<AuthorizeQuery>) -> Result<Response> {
    enabled(&st)?;
    let key = st.suite_store().key(&q.client_id).await?.ok_or_else(|| err(400, "The app asking to sign you in is not linked here."))?;
    let base = key.redirect_url.clone().ok_or_else(|| {
        err(
            400,
            format!("The key \u{201c}{}\u{201d} has no address to sign people in to. Make a key with the other app's address.", key.name),
        )
    })?;
    if !redirect_allowed(&q.redirect_uri, &base) {
        return Err(err(400, format!("Sign-in may only return to {base}; the app asked for {}.", q.redirect_uri)));
    }
    let back = |params: &[(&str, &str)]| {
        let mut url = q.redirect_uri.clone();
        let mut sep = if url.contains('?') { '&' } else { '?' };
        for (k, v) in params {
            url.push(sep);
            url.push_str(&format!("{k}={}", encode(v)));
            sep = '&';
        }
        if let Some(s) = &q.state {
            url.push_str(&format!("{sep}state={}", encode(s)));
        }
        Redirect::to(&url).into_response()
    };
    if q.response_type != "code" {
        return Ok(back(&[("error", "unsupported_response_type")]));
    }
    let account = match st.signed_in(&mut parts).await {
        Ok(a) => a,
        Err(e) if e.status == StatusCode::UNAUTHORIZED => {
            let here = parts.uri.path_and_query().map(|p| p.as_str().to_string()).unwrap_or_else(|| parts.uri.path().to_string());
            let login = fill(&st.suite().config().login_path, &[("next", &here)]);
            return Ok(Redirect::to(&login).into_response());
        }
        Err(e) => return Err(e),
    };
    let code = crypto::random_token(32);
    let grant =
        Grant { client_id: key.id, redirect_uri: q.redirect_uri.clone(), account, nonce: q.nonce.clone(), at: kit_server_core::db::now() };
    st.suite().mem().codes.insert(code.clone(), grant);
    Ok(back(&[("code", &code)]))
}

/// The form of a token request.
#[derive(Debug, Deserialize)]
struct TokenForm {
    /// must be `authorization_code`
    #[serde(default)]
    grant_type: String,
    /// the code
    #[serde(default)]
    code: String,
    /// the same redirect as in the authorization request
    #[serde(default)]
    redirect_uri: String,
    /// the key's id (`client_secret_post`)
    client_id: Option<String>,
    /// the key's secret half (`client_secret_post`)
    client_secret: Option<String>,
}

/// An OAuth error answer (`error`, and the sentence in both `error_description` and the kit's `detail`).
fn oauth_error(status: u16, code: &str, sentence: &str) -> Response {
    let s = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST);
    (s, [(header::CACHE_CONTROL, "no-store")], Json(json!({ "error": code, "error_description": sentence, "detail": sentence })))
        .into_response()
}

/// The client id and secret of a token request: `Authorization: Basic`, else the form's.
fn client_credentials(headers: &HeaderMap, f: &TokenForm) -> Option<(String, String)> {
    let basic = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).and_then(|v| {
        let (scheme, rest) = v.trim().split_once(' ')?;
        if !scheme.eq_ignore_ascii_case("basic") {
            return None;
        }
        let raw = base64::engine::general_purpose::STANDARD.decode(rest.trim()).ok()?;
        let text = String::from_utf8(raw).ok()?;
        let (id, secret) = text.split_once(':')?;
        Some((urlencoding::decode(id).ok()?.into_owned(), urlencoding::decode(secret).ok()?.into_owned()))
    });
    basic.or_else(|| Some((f.client_id.clone()?, f.client_secret.clone()?)))
}

/// The claims of `account` for client `client_id`: the app's ([`SuiteHost::oidc_claims`]) and `sub`.
async fn claims_of<S: SuiteHost>(st: &S, account: &Account) -> Result<Value> {
    let mut c = st.oidc_claims(account.clone()).await?;
    if !c.is_object() {
        c = json!({});
    }
    c["sub"] = json!(account.id.to_string());
    Ok(c)
}

/// `POST /api/kit/oidc/token`: an authorization code for `{access_token, token_type, expires_in, id_token}`.
async fn token<S: SuiteHost>(State(st): State<S>, headers: HeaderMap, Form(f): Form<TokenForm>) -> Response {
    if let Err(e) = enabled(&st) {
        return e.into_response();
    }
    if f.grant_type != "authorization_code" {
        return oauth_error(400, "unsupported_grant_type", "Only the authorization code grant is supported.");
    }
    let Some((client_id, secret)) = client_credentials(&headers, &f) else {
        return oauth_error(401, "invalid_client", "The app did not say who it is.");
    };
    let whole = format!("{client_id}.{secret}");
    let key = match st.suite().check_key(st.suite_store(), &whole).await {
        Ok(k) => k,
        Err(_) => return oauth_error(401, "invalid_client", "The app's key is not one this app made, or it was revoked."),
    };
    let grant = st.suite().mem().codes.remove(&f.code);
    let Some(grant) = grant.filter(|g| g.client_id == key.id && g.redirect_uri == f.redirect_uri) else {
        return oauth_error(400, "invalid_grant", "The sign-in code is wrong, used or too old. Sign in again.");
    };
    let mut claims = match claims_of(&st, &grant.account).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    let now = kit_server_core::db::now() as u64;
    claims["iss"] = json!(st.suite().base_url(&headers));
    claims["aud"] = json!(key.id);
    claims["iat"] = json!(now);
    claims["exp"] = json!(now + crate::ID_TOKEN_SECS);
    if let Some(n) = grant.nonce {
        claims["nonce"] = json!(n);
    }
    let id_token = crypto::sign(&claims, &secret);
    let access = crypto::random_token(32);
    st.suite().mem().access.insert(access.clone(), (grant.account, key.id, kit_server_core::db::now() + crate::ACCESS_SECS));
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({ "access_token": access, "token_type": "Bearer", "expires_in": crate::ACCESS_SECS as u64, "id_token": id_token })),
    )
        .into_response()
}

/// `GET /api/kit/oidc/userinfo` (`Authorization: Bearer <access token>`): the person's claims.
async fn userinfo<S: SuiteHost>(State(st): State<S>, headers: HeaderMap) -> Result<Json<Value>> {
    enabled(&st)?;
    let token = crate::bearer(&headers).ok_or_else(|| err(401, "No access token was sent."))?;
    let found = st.suite().mem().access.get(&token).cloned();
    let (account, _, _) = found.ok_or_else(|| err(401, "The access token is wrong or too old."))?;
    Ok(Json(claims_of(&st, &account).await?))
}

#[cfg(test)]
mod tests {
    use super::redirect_allowed;

    #[test]
    fn redirects_stay_under_the_keys_address() {
        assert!(redirect_allowed("https://b.test/api/kit/sso/callback", "https://b.test"));
        assert!(redirect_allowed("https://b.test", "https://b.test"));
        assert!(!redirect_allowed("https://b.test.evil/cb", "https://b.test"));
        assert!(!redirect_allowed("https://evil.test/https://b.test", "https://b.test"));
        assert!(redirect_allowed("https://b.test/p/cb", "https://b.test/p"));
        assert!(!redirect_allowed("https://b.test/pp/cb", "https://b.test/p"));
    }
}
