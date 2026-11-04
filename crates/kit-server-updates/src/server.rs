//! The server's own update: the newest release from GitHub's answer, the update channel, and the parts of
//! the update answer every app shares. How the server replaces itself (a new image, a package, a binary) is
//! the app's.
//!
//! The channel picks what counts: `stable` = the newest full release, `hotfix` = the newest of those and the
//! hotfix prereleases (tag `<hotfix prefix><last release>.<build>`, e.g. `hotfix-0.35.2.17`); a full release
//! is newer than every hotfix before it (0.35.3 > 0.35.2.17). An app without hotfixes
//! ([`Feed::hotfix_prefix`] unset) only ever has stable releases.

use crate::feed::{notes, Feed};
use serde_json::{json, Value};

/// Which builds the server's update offers (an app setting, say `update_channel`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// tagged releases
    Stable,
    /// also the hotfix builds (tags with the feed's hotfix prefix)
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
pub fn parse_release(v: &Value, now: f64, feed: &Feed) -> Option<Value> {
    let tag = v["tag_name"].as_str().map(str::trim).filter(|t| !t.is_empty())?;
    let hotfix = feed.is_hotfix_tag(tag);
    let (notes, cut) = notes(v["body"].as_str().unwrap_or(""));
    Some(json!({
        "tag": tag,
        "latest": feed.tag_version(tag),
        "hotfix": hotfix,
        "notes": notes,
        "notes_cut": cut,
        "url": v["html_url"].as_str().map(String::from).unwrap_or_else(|| format!("https://github.com/{}/releases/tag/{tag}", feed.slug())),
        "published_at": v["published_at"],
        "checked_at": now as i64,
    }))
}

/// Whether `channel` counts the release `v` (one entry of GitHub's release list): never a draft or a
/// prerelease that is not a hotfix (a beta); a hotfix published as a normal release on both channels, a
/// hotfix prerelease on the hotfix channel only.
pub fn counts(v: &Value, channel: Channel, feed: &Feed) -> bool {
    let hotfix = v["tag_name"].as_str().is_some_and(|t| feed.is_hotfix_tag(t));
    let prerelease = v["prerelease"] == true;
    v["draft"] != true && (!prerelease || (hotfix && channel == Channel::Hotfix))
}

/// The channel's pick from GitHub's release list: the highest version among the releases it counts
/// ([`counts`]), whichever of them GitHub marks "Latest" (a hotfix built while a full release was on its way
/// carries the older number yet can become "Latest").
pub fn pick_newest(list: &Value, channel: Channel, now: f64, feed: &Feed) -> Option<Value> {
    list.as_array()?
        .iter()
        .filter(|v| counts(v, channel, feed))
        .filter_map(|v| parse_release(v, now, feed))
        // a tag without a number (a stray name) can't be compared
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

/// The shared part of `GET /api/update` from the kept `release`: `{current, dev, latest, tag, hotfix,
/// is_newer, notes, notes_cut, url, published_at, checked_at, restart: "ready"}`. The app adds fields of its
/// own (whether and how it can update itself, the channel, the last error, …).
pub fn answer(release: Option<&Value>, current: &str, dev: bool, feed: &Feed) -> Value {
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
        "url": r["url"].as_str().map(String::from).unwrap_or_else(|| feed.releases_page()),
        "published_at": r["published_at"],
        "checked_at": r["checked_at"],
        "restart": "ready",
    })
}
