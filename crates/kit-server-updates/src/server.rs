//! The server's own update (Encore's `encore_system::update`, without the Docker part): the newest release
//! from GitHub's answer, the update channel, and the parts of the `GET /api/system/update` answer every app
//! shares.
//!
//! The `update_channel` setting picks what counts: `stable` = the newest full release (GitHub's latest),
//! `hotfix` = the newest of those and the hotfix prereleases CI publishes (tag `hotfix-<last release>.<run>`,
//! e.g. `hotfix-0.35.2.17`); a full release is newer than every hotfix before it (0.35.3 > 0.35.2.17).

use crate::feed::{notes, Repo};
use crate::HOTFIX_PREFIX;
use serde_json::{json, Value};

/// Which builds the server's update offers (the `update_channel` setting).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// tagged releases
    Stable,
    /// also the builds of every push to main (`hotfix-…` tags)
    Hotfix,
}

impl Channel {
    /// The channel a setting value names (anything but `hotfix` is stable).
    pub fn parse(s: &str) -> Channel {
        if s.trim().eq_ignore_ascii_case("hotfix") {
            Channel::Hotfix
        } else {
            Channel::Stable
        }
    }

    /// The channel's name in the API.
    pub fn name(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Hotfix => "hotfix",
        }
    }
}

/// GitHub's release JSON → what is kept: `{tag, latest, hotfix, notes, notes_cut, url, published_at,
/// checked_at}` (`now`: unix seconds); `None` for an answer without a tag.
pub fn parse_release(v: &Value, now: f64, repo: &Repo) -> Option<Value> {
    let tag = v["tag_name"].as_str().map(str::trim).filter(|t| !t.is_empty())?;
    let hotfix = tag.starts_with(HOTFIX_PREFIX);
    let (notes, cut) = notes(v["body"].as_str().unwrap_or(""));
    Some(json!({
        "tag": tag,
        "latest": tag.trim_start_matches(HOTFIX_PREFIX).trim_start_matches(['v', 'V']),
        "hotfix": hotfix,
        "notes": notes,
        "notes_cut": cut,
        "url": v["html_url"].as_str().map(String::from).unwrap_or_else(|| format!("https://github.com/{}/releases/tag/{tag}", repo.slug())),
        "published_at": v["published_at"],
        "checked_at": now as i64,
    }))
}

/// The hotfix channel's pick from GitHub's release list: the highest version among the full releases and the
/// hotfix prereleases (drafts and other prereleases don't count).
pub fn pick_newest(list: &Value, now: f64, repo: &Repo) -> Option<Value> {
    list.as_array()?
        .iter()
        .filter(|v| v["draft"] != true)
        .filter(|v| {
            let hotfix = v["tag_name"].as_str().is_some_and(|t| t.starts_with(HOTFIX_PREFIX));
            if v["prerelease"] == true {
                hotfix
            } else {
                !hotfix
            }
        })
        .filter_map(|v| parse_release(v, now, repo))
        .filter(|r| !kit_server_core::version::parts(r["latest"].as_str().unwrap_or("")).is_empty())
        .max_by(|a, b| kit_server_core::version::compare(a["latest"].as_str().unwrap_or(""), b["latest"].as_str().unwrap_or("")))
}

/// Whether `latest` is newer than the running version `current` (a `dev` build, one without a release
/// number: any release is).
pub fn is_newer(latest: &str, current: &str, dev: bool) -> bool {
    if latest.trim().is_empty() {
        return false;
    }
    dev || kit_server_core::version::newer(latest, current)
}

/// The shared part of `GET /api/system/update` from the kept `release`: `{current, dev, latest, tag, hotfix,
/// is_newer, notes, notes_cut, url, published_at, checked_at, restart: "ready"}`. The app adds its own
/// (Encore: `self_update`, `self_update_reason`, `image`, `container_image`, `error`, `check_updates`,
/// `channel`).
pub fn answer(release: Option<&Value>, current: &str, dev: bool, repo: &Repo) -> Value {
    let r = release.cloned().unwrap_or(Value::Null);
    let latest = r["latest"].as_str().map(String::from);
    json!({
        "current": current,
        "dev": dev,
        "latest": latest,
        "tag": r["tag"],
        "hotfix": r["hotfix"] == true,
        "is_newer": latest.as_deref().is_some_and(|l| is_newer(l, current, dev)),
        "notes": r["notes"].as_str().unwrap_or(""),
        "notes_cut": r["notes_cut"] == true,
        "url": r["url"].as_str().map(String::from).unwrap_or_else(|| repo.releases_page()),
        "published_at": r["published_at"],
        "checked_at": r["checked_at"],
        "restart": "ready",
    })
}
