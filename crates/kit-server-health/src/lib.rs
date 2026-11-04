//! `GET /api/health` (contracts/health.md): the server's version and the build of the web app served at `/`.
//! No sign-in needed. Clients ask it to notice that the server updated (a new `web` or `version`) or is being
//! restarted (no answer), as kit-rules `update` decides. The route's path is [`HealthPaths`] (default
//! `/api/health`).
//!
//! ```
//! let web = kit_server_health::web_build(std::path::Path::new("static"), "demo-web-");
//! let health = kit_server_health::Health::new("1.4.0", web);
//! let app: axum::Router = axum::Router::new().merge(kit_server_health::router(health));
//! ```

use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Map, Value};
use std::path::Path;

/// What `/api/health` answers. Read once at start: a new web build only arrives with a new server process.
#[derive(Debug, Clone, PartialEq)]
pub struct Health {
    /// the server's version: `MAJOR.MINOR.PATCH`, or `MAJOR.MINOR.PATCH.N` for a hotfix build
    pub version: String,
    /// the server's implementation (`"rust"`)
    pub server: String,
    /// the build of the web app served at `/` (the hash in its script's name); `None` without a web build
    pub web: Option<String>,
    /// more fields the app adds to the answer (clients ignore fields they do not know)
    pub extra: Map<String, Value>,
}

impl Health {
    /// The answer of a Rust server at `version` serving the web build `web`.
    pub fn new(version: impl Into<String>, web: Option<String>) -> Health {
        Health { version: version.into(), server: "rust".into(), web, extra: Map::new() }
    }

    /// The answer as JSON: `{ok: true, version, server, web}` and the app's extra fields.
    pub fn json(&self) -> Value {
        let mut v = json!({ "ok": true, "version": self.version, "server": self.server, "web": self.web });
        if let Value::Object(o) = &mut v {
            for (k, x) in &self.extra {
                o.entry(k.clone()).or_insert_with(|| x.clone());
            }
        }
        v
    }
}

/// Where the health route is served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthPaths {
    /// `GET`: the health answer (default `/api/health`)
    pub health: String,
}

impl Default for HealthPaths {
    fn default() -> Self {
        HealthPaths { health: "/api/health".into() }
    }
}

/// The health route at its default path ([`HealthPaths::default`]), answering `health`.
pub fn router<S: Clone + Send + Sync + 'static>(health: Health) -> Router<S> {
    router_with(health, HealthPaths::default())
}

/// The health route at `paths`, answering `health` (made once, so every request gets the same bytes).
pub fn router_with<S: Clone + Send + Sync + 'static>(health: Health, paths: HealthPaths) -> Router<S> {
    let body = health.json();
    Router::new().route(&paths.health, get(move || async move { Json(body) }))
}

/// The build id of the web app in `static_dir`: the content hash in the name of trunk's script,
/// `<prefix><hash>.js` (`demo-web-f92ab5b84b305a59.js` for the prefix `demo-web-`; the same hash a service
/// worker can make its cache name from). The prefix is the app's own: its web crate's name and a `-`. `None` when the folder has no such file (a server without the
/// web app, the tests).
pub fn web_build(static_dir: &Path, prefix: &str) -> Option<String> {
    let mut found: Vec<String> = std::fs::read_dir(static_dir)
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|name| web_build_of(&name, prefix).map(str::to_string))
        .collect();
    // one build per folder; should an old script linger next to the new one, the answer is still stable
    found.sort();
    found.pop()
}

/// The hash of a file name `<prefix><hash>.js` (not the `_bg.wasm`, not a `.js.br` copy); `None` for any other
/// name. Trunk drops leading zeros of the hash, so 12 hex digits or more count.
pub fn web_build_of<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    let hash = name.strip_prefix(prefix)?.strip_suffix(".js")?;
    (hash.len() >= 12 && hash.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[test]
    fn the_web_build_is_the_scripts_hash() {
        let p = "demo-web-";
        assert_eq!(web_build_of("demo-web-f92ab5b84b305a59.js", p), Some("f92ab5b84b305a59"));
        assert_eq!(web_build_of("demo-web-59d563296e62da3.js", p), Some("59d563296e62da3"));
        for other in [
            "demo-web-f92ab5b84b305a59_bg.wasm",
            "demo-web-f92ab5b84b305a59.js.br",
            "demo-web-abc.js",
            "app.js",
            "demo-web-zzzzzzzzzzzz.js",
        ] {
            assert_eq!(web_build_of(other, p), None, "{other}");
        }
        let dir = std::env::temp_dir().join(format!("kit-health-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(web_build(&dir, p), None, "an empty folder");
        std::fs::write(dir.join("demo-web-f92ab5b84b305a59.js"), "").unwrap();
        std::fs::write(dir.join("demo-web-f92ab5b84b305a59_bg.wasm"), "").unwrap();
        assert_eq!(web_build(&dir, p).as_deref(), Some("f92ab5b84b305a59"));
        assert_eq!(web_build(&dir.join("missing"), p), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn answers_version_and_build() {
        let mut h = Health::new("1.4.0", Some("3f9a0c1b2d4e".into()));
        h.extra.insert("ok".into(), json!(false));
        h.extra.insert("name".into(), json!("Demo"));
        let app: Router = router(h);
        let r = app.oneshot(Request::get("/api/health").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(r.status(), 200);
        let b = axum::body::to_bytes(r.into_body(), 1 << 16).await.unwrap();
        let v: Value = serde_json::from_slice(&b).unwrap();
        // an extra field never overrides the contract's own
        assert_eq!(v, json!({ "ok": true, "version": "1.4.0", "server": "rust", "web": "3f9a0c1b2d4e", "name": "Demo" }));
        assert_eq!(Health::new("1.0.0", None).json()["web"], Value::Null);
    }

    #[tokio::test]
    async fn the_path_is_the_apps() {
        let paths = HealthPaths { health: "/status".into() };
        let app: Router = router_with(Health::new("1.0.0", None), paths);
        let r = app.clone().oneshot(Request::get("/status").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(r.status(), 200);
        let r = app.oneshot(Request::get("/api/health").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(r.status(), 404);
    }
}
