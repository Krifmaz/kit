//! "Update now", the server's part: which tags may be installed, the pull of the new image, and the start
//! of the helper container that swaps the server's own ([`crate::swap`]). Each step goes into the update's
//! [`Report`]. Picking the release, checks only the app knows and keeping the record are the app's.
use crate::docker::{self, Docker};
use crate::report::{self, Report};
use crate::swap;
use crate::App;
use std::time::Duration;

/// A pull of a new image (a few hundred MB) on a slow line.
pub const PULL_TIMEOUT: Duration = Duration::from_secs(20 * 60);

/// The tag to pull: `latest` or a version (a leading v is dropped: images are tagged `0.28.0`), letters,
/// digits, dots, dashes and underscores only; versions before [`App::first_self_updating`] are refused
/// (their image has no helper, so the swap could never finish).
pub fn check_tag(app: &App, tag: &str) -> Result<String, String> {
    let t = tag.trim();
    let valid = !t.is_empty() && t.len() <= 64 && t.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
    if !valid {
        return Err("Not a valid image tag".into());
    }
    let t = if t.len() > 1 && t.starts_with(['v', 'V']) && t.as_bytes()[1].is_ascii_digit() { &t[1..] } else { t };
    if let Some(first) = app.first_self_updating.as_deref() {
        if t.as_bytes()[0].is_ascii_digit() && kit_server_core::version::compare(t, first) == std::cmp::Ordering::Less {
            return Err(format!("{} {t} can't update itself; pick {first} or newer", app.name));
        }
    }
    Ok(t.to_string())
}

/// The words for a failed pull of `image`. The pull carries no registry login, so "unauthorized" or "denied"
/// means the image is private (or the tag doesn't exist there); on GitHub's registry the package must be
/// made public.
pub fn pull_error(image: &str, e: &str) -> String {
    let l = e.to_ascii_lowercase();
    if !(l.contains("unauthorized") || l.contains("denied")) {
        return format!("Could not pull {image}: {e}");
    }
    let (repo, _) = swap::parse_image(image);
    match repo.strip_prefix("ghcr.io/") {
        Some(path) => {
            let pkg = path.rsplit('/').next().unwrap_or(path);
            format!(
                "Could not pull {image}: {e}. GitHub refused an anonymous pull: the {pkg} package on ghcr.io must be public (GitHub → Packages → {pkg} → Package settings → Change visibility)."
            )
        }
        None => format!("Could not pull {image}: {e}. The registry refused an anonymous pull: the image must be public."),
    }
}

/// Pull `<app image>:tag`, then create and start the helper container from it, writing each step into `r`.
/// `self_id` is this server's container ([`crate::probe::SelfUpdate::id`]); `schema` the database schema the
/// running server keeps (the helper refuses an image that knows less). Ok(the helper's name); Err((the step
/// that failed, its error)).
pub async fn start_update(
    app: &App,
    d: &Docker,
    self_id: &str,
    tag: &str,
    schema: Option<i64>,
    r: &mut Report,
) -> Result<String, (&'static str, String)> {
    let image = format!("{}:{tag}", app.image);
    r.line("pull", &format!("pulling {image}"));
    d.pull(&app.image, tag, PULL_TIMEOUT).await.map_err(|e| ("pull", pull_error(&image, &e)))?;
    r.line("pull", &format!("{image} is on this host"));
    let me = d
        .inspect(self_id)
        .await
        .map_err(|e| ("helper", format!("The Docker socket refused the request: {e}")))?
        .ok_or_else(|| ("helper", "Not running in a container".to_string()))?;
    let body = swap::plan_helper_body(app, &me, &image, schema)
        .ok_or_else(|| ("helper", "The Docker socket is not mounted from the host (no bind for /var/run/docker.sock)".to_string()))?;
    // an ended helper of an earlier try would be read as this one's: it goes first
    if let Ok(old) = report::helpers(d, &app.label).await {
        for (id, running, _) in old {
            if running {
                return Err(("helper", "An update helper is still running: wait for it to end".into()));
            }
            let _ = d.delete(&format!("/containers/{id}?force=true")).await;
        }
    }
    let name = format!("{}-update-{}", me["Name"].as_str().unwrap_or(app.program()).trim_start_matches('/'), crate::now() as i64);
    match schema {
        Some(s) => r.line("helper", &format!("creating the helper container {name} (database schema {s})")),
        None => r.line("helper", &format!("creating the helper container {name}")),
    }
    let (code, v) = d
        .post(&format!("/containers/create?name={}", urlencoding::encode(&name)), Some(&body))
        .await
        .map_err(|e| ("helper", format!("Could not start the update helper: {e}")))?;
    if code != 201 {
        return Err(("helper", format!("Could not start the update helper: Docker answered {code}: {}", docker::message(&v))));
    }
    let id = v["Id"].as_str().unwrap_or(&name).to_string();
    let (code, v) = d
        .post(&format!("/containers/{id}/start"), None)
        .await
        .map_err(|e| ("helper", format!("Could not start the update helper: {e}")))?;
    if !(200..300).contains(&code) {
        let _ = d.delete(&format!("/containers/{id}?force=true")).await;
        return Err(("helper", format!("Could not start the update helper: Docker answered {code}: {}", docker::message(&v))));
    }
    r.line("helper", &format!("{name} started: it stops this server and swaps the container (the server is down for about a minute)"));
    Ok(name)
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod tests;
