//! Whether "Update now" can replace this container, and in plain words why not: the socket (there, a
//! socket, writable, answering `_ping`), and this process in a container Docker knows that it may replace.
//! Whether the owner switched Update now off is the app's setting, checked before this.
use crate::docker::{self, Docker};
use crate::swap;
use crate::App;
use serde_json::{json, Value};
use std::path::Path;

/// Whether Update now can replace this container, and the facts it needs to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SelfUpdate {
    /// the container can update itself
    pub ready: bool,
    /// why not, when it can't
    pub reason: Option<String>,
    /// this container's full id (when ready)
    pub id: Option<String>,
    /// the container's name
    pub name: Option<String>,
    /// the image it was created from
    pub image: Option<String>,
}

impl SelfUpdate {
    /// A "cannot update" answer with `reason`.
    pub fn no(reason: impl Into<String>) -> SelfUpdate {
        SelfUpdate { reason: Some(reason.into()), ..Default::default() }
    }

    /// The answer as JSON (for a cache).
    pub fn json(&self) -> Value {
        json!({ "ready": self.ready, "reason": self.reason, "id": self.id, "name": self.name, "image": self.image })
    }

    /// The answer from its JSON.
    pub fn from_json(v: &Value) -> SelfUpdate {
        let s = |k: &str| v[k].as_str().map(String::from);
        SelfUpdate { ready: v["ready"] == true, reason: s("reason"), id: s("id"), name: s("name"), image: s("image") }
    }
}

/// Whether this process may write `path` (`access(W_OK)`).
fn writable(path: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(path.as_os_str().as_bytes()) else { return false };
    // SAFETY: a valid NUL-terminated path; access() only reads it
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

/// Whether Update now can work here, and in plain words why not: the socket at `sock`, and this process in a
/// container Docker knows (the id from `mountinfo`, or the `hostname`, which is the id's first 12 characters
/// unless the owner set one).
pub async fn probe_with(app: &App, sock: &Path, hostname: &str, mountinfo: &str) -> SelfUpdate {
    let meta = match std::fs::metadata(sock) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return SelfUpdate::no("The Docker socket is not mounted"),
        Err(e) => return SelfUpdate::no(format!("The Docker socket can't be read: {e}")),
    };
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    if !meta.file_type().is_socket() {
        return SelfUpdate::no(format!("The Docker socket is not mounted ({} is not a socket)", sock.display()));
    }
    if !writable(sock) {
        // SAFETY: plain getters without arguments
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        return SelfUpdate::no(format!(
            "The Docker socket is not writable by {} (it runs as uid {uid}, gid {gid}; the socket's group is {}): set PGID={} or PUID=0",
            app.name,
            meta.gid(),
            meta.gid()
        ));
    }
    let d = Docker::new(sock);
    match d.get("/_ping").await {
        Ok((200, _)) => {}
        Ok((code, v)) => {
            return SelfUpdate::no(format!("The Docker socket refused the request: Docker answered {code}: {}", docker::message(&v)))
        }
        Err(e) => return SelfUpdate::no(format!("The Docker socket refused the request: {e}")),
    }
    let mut candidates: Vec<String> = docker::container_id_from_mountinfo(mountinfo).into_iter().collect();
    if !hostname.is_empty() && !candidates.iter().any(|c| c == hostname) {
        candidates.push(hostname.to_string());
    }
    for c in candidates {
        match d.inspect(&c).await {
            Ok(Some(v)) => {
                let id = v["Id"].as_str().unwrap_or("");
                if id.starts_with(&c) || v["Config"]["Hostname"].as_str() == Some(hostname) {
                    if let Some(why) = swap::cannot_replace(&v) {
                        return SelfUpdate::no(why);
                    }
                    let s = |x: &Value| x.as_str().map(String::from);
                    return SelfUpdate {
                        ready: true,
                        reason: None,
                        id: s(&v["Id"]),
                        name: v["Name"].as_str().map(|n| n.trim_start_matches('/').to_string()),
                        image: s(&v["Config"]["Image"]),
                    };
                }
            }
            Ok(None) => {}
            Err(e) => return SelfUpdate::no(format!("The Docker socket refused the request: {e}")),
        }
    }
    SelfUpdate::no("Not running in a container")
}

/// [`probe_with`] for this process.
pub async fn probe(app: &App, sock: &Path) -> SelfUpdate {
    let mountinfo = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
    probe_with(app, sock, &docker::hostname(), &mountinfo).await
}
