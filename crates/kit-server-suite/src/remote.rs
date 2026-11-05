//! Asking another app: its manifest, whether a key works, its search, a sign-in code exchange. Every call has
//! a time limit ([`TIMEOUT`]) and a size limit ([`MAX_BYTES`]) and fails with one plain sentence that names
//! the address.

use kit_rules::suite::{join, Manifest, SearchHit};
use serde_json::Value;
use std::time::Duration;

/// How long one call to another app may take.
pub const TIMEOUT: Duration = Duration::from_secs(5);
/// The largest answer read from another app.
pub const MAX_BYTES: usize = 1 << 20;
/// Where every app serves its manifest.
pub const MANIFEST_PATH: &str = "/api/kit/manifest";
/// Where a key is checked (`Authorization: Bearer <key>` → `{id, name}` of the app that made it).
pub const PING_PATH: &str = "/api/kit/suite/ping";
/// The header a search on behalf of a person names them in (their user name in the asking app; a hint the
/// answering app may use or ignore).
pub const USER_HEADER: &str = "x-kit-suite-user";

/// The HTTP client the suite uses: the time limit above, no redirects followed, the system's proxy settings.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("kit-server-suite/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("an HTTP client")
}

/// Why a call to `url` failed, in one sentence.
fn failed(url: &str, e: &reqwest::Error) -> String {
    if e.is_timeout() {
        format!("{url} did not answer within {} seconds.", TIMEOUT.as_secs())
    } else if e.is_connect() {
        format!("{url} can't be reached. Check the address and that the app is running.")
    } else {
        format!("{url} can't be read: {e}")
    }
}

/// The JSON answer of `req` to `url`: the body when the status is 2xx; otherwise its `detail`, or the status.
pub async fn json(req: reqwest::RequestBuilder, url: &str) -> Result<Value, String> {
    let mut resp = req.send().await.map_err(|e| failed(url, &e))?;
    let status = resp.status();
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| failed(url, &e))? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_BYTES {
            return Err(format!("{url} answered with more than {} MB.", MAX_BYTES >> 20));
        }
    }
    let v: Option<Value> = serde_json::from_slice(&body).ok();
    if !status.is_success() {
        let detail = v.as_ref().and_then(|v| v.get("detail")).and_then(Value::as_str).map(str::to_string);
        return Err(match (status.as_u16(), detail) {
            (401 | 403, _) => {
                format!("{url} did not accept the key (HTTP {}). Make a new key in that app and paste it here.", status.as_u16())
            }
            (_, Some(d)) => format!("{url} answered: {d}"),
            (s, None) => format!("{url} answered HTTP {s}."),
        });
    }
    v.ok_or_else(|| format!("{url} did not answer with JSON. Is it an app on the kit?"))
}

/// The manifest of the app at `base`, checked: one sentence for each problem otherwise.
pub async fn manifest(http: &reqwest::Client, base: &str) -> Result<Manifest, String> {
    let url = join(base, MANIFEST_PATH);
    let v = json(http.get(&url), &url).await?;
    let m: Manifest = serde_json::from_value(v).map_err(|e| format!("{url} is not a kit manifest: {e}"))?;
    let problems = m.problems();
    if !problems.is_empty() {
        return Err(format!("{url}: {}", problems.join(" ")));
    }
    Ok(m)
}

/// Whether the app at `base` takes `key`: its `{id, name}` when it does.
pub async fn ping(http: &reqwest::Client, base: &str, key: &str) -> Result<Value, String> {
    let url = join(base, PING_PATH);
    json(http.get(&url).bearer_auth(key), &url).await
}

/// GET `path` of the app at `base` with `key`: its JSON answer.
pub async fn get(http: &reqwest::Client, base: &str, path: &str, key: &str) -> Result<Value, String> {
    let url = join(base, path);
    json(http.get(&url).bearer_auth(key), &url).await
}

/// The search of the app `m` (with `key`) for `q`, on behalf of `user`: at most `limit` hits.
pub async fn search(
    http: &reqwest::Client,
    m: &Manifest,
    key: &str,
    q: &str,
    limit: usize,
    user: Option<&str>,
) -> Result<Vec<SearchHit>, String> {
    let Some(path) = m.search.as_deref() else { return Ok(vec![]) };
    let url = join(&m.base_url, path);
    let mut req = http.get(&url).bearer_auth(key).query(&[("q", q), ("limit", &limit.to_string())]);
    if let Some(u) = user {
        req = req.header(USER_HEADER, u);
    }
    let v = json(req, &url).await?;
    let hits: Vec<SearchHit> = serde_json::from_value(v.get("results").cloned().unwrap_or(Value::Array(vec![])))
        .map_err(|e| format!("{url} answered results that can't be read: {e}"))?;
    Ok(hits.into_iter().take(limit).collect())
}
