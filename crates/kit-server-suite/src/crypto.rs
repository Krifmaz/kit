//! The small pieces of cryptography the suite needs: random tokens, the keys apps link with
//! (`ks_<12 hex>.<secret>`, kept as the SHA-256 of the secret) and HS256 ID tokens (JWT, signed with the
//! client's secret as OIDC Core §10.1 says for symmetric signatures).

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use rand::Rng;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// `bytes` random bytes as URL-safe base64 without padding.
pub fn random_token(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::rng().fill(&mut b[..]);
    B64.encode(b)
}

/// `sha256:<hex>` of `s`.
pub fn hash(s: &str) -> String {
    let d = Sha256::digest(s.as_bytes());
    let mut out = String::with_capacity(7 + 64);
    out.push_str("sha256:");
    for b in d {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Whether two strings are equal, taking the same time wherever they differ.
pub fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// A new key: (its public id `ks_<12 hex>`, the whole key `<id>.<secret>`, the secret's hash).
pub fn new_key() -> (String, String, String) {
    let mut id = [0u8; 6];
    rand::rng().fill(&mut id[..]);
    let id = format!("ks_{}", id.iter().map(|b| format!("{b:02x}")).collect::<String>());
    let secret = random_token(32);
    let hash = hash(&secret);
    (id.clone(), format!("{id}.{secret}"), hash)
}

/// A key's two halves (`ks_…`, secret), when `key` is one.
pub fn split_key(key: &str) -> Option<(&str, &str)> {
    let (id, secret) = key.split_once('.')?;
    (id.starts_with("ks_") && id.len() > 3 && !secret.is_empty()).then_some((id, secret))
}

/// An HS256 JWT of `claims`, signed with `secret`.
pub fn sign(claims: &Value, secret: &str) -> String {
    let head = B64.encode(br#"{"alg":"HS256","typ":"JWT"}"#);
    let body = B64.encode(claims.to_string().as_bytes());
    let input = format!("{head}.{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC takes a key of any length");
    mac.update(input.as_bytes());
    format!("{input}.{}", B64.encode(mac.finalize().into_bytes()))
}

/// The claims of an HS256 JWT signed with `secret`; None for any other token (another algorithm, a wrong
/// signature, a token that is not three parts of base64 JSON).
pub fn verify(token: &str, secret: &str) -> Option<Value> {
    let mut parts = token.split('.');
    let (head, body, sig) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let header: Value = serde_json::from_slice(&B64.decode(head).ok()?).ok()?;
    if header.get("alg").and_then(Value::as_str) != Some("HS256") {
        return None;
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(format!("{head}.{body}").as_bytes());
    mac.verify_slice(&B64.decode(sig).ok()?).ok()?;
    serde_json::from_slice(&B64.decode(body).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_split_and_hash() {
        let (id, key, h) = new_key();
        assert!(id.starts_with("ks_") && id.len() == 15);
        let (i, s) = split_key(&key).unwrap();
        assert_eq!(i, id);
        assert!(same(&hash(s), &h));
        assert!(!same(&hash("other"), &h));
        assert_eq!(split_key("nope.x"), None);
        assert_eq!(split_key("ks_.x"), None);
        assert_eq!(split_key("ks_ab."), None);
        assert_ne!(random_token(16), random_token(16));
    }

    #[test]
    fn tokens_verify_only_with_their_secret() {
        let t = sign(&json!({ "sub": "1", "n": 2 }), "s3cret");
        assert_eq!(verify(&t, "s3cret").unwrap()["sub"], "1");
        assert!(verify(&t, "other").is_none());
        let mut bad = t.clone();
        bad.push('x');
        assert!(verify(&bad, "s3cret").is_none());
        let none = format!("{}.{}.", B64.encode(br#"{"alg":"none"}"#), B64.encode(b"{}"));
        assert!(verify(&none, "s3cret").is_none());
        assert!(verify("a.b", "s3cret").is_none());
    }
}
