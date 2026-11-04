//! The app's GitHub releases as an update feed for its in-app updaters (Encore's `UpdateRules.kt` on Android
//! and `clients/desktop/src/desktop/updater.rs`): the newest full release, its file for this device, the
//! release's SHA256SUMS, and the checks before anything is installed.
//!
//! - **Full release:** tag `v<MAJOR>.<MINOR>.<PATCH>`, not a draft, not a prerelease. It carries the apps.
//! - **Hotfix:** tag `hotfix-<version>`. It can be GitHub's "Latest" but carries no apps, so an app updater
//!   never picks it (the server's hotfix channel does: [`crate::server`]).
//! - A release made without rebuilding the apps carries the last APK over under its own version, so the
//!   APK's version is read from its file name, not from the tag.
//! - Only files under `https://github.com/<owner>/<app>/releases/download/` are used, and a file installs
//!   only when the release's `SHA256SUMS` (or `SHA256SUMS.txt`) lists it with the same hash.

use crate::{is_release_version, newer_release, HOTFIX_PREFIX, NOTES_MAX};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// The checksum files of a release (the release workflow writes `SHA256SUMS` over every file).
pub const SUMS: [&str; 2] = ["SHA256SUMS", "SHA256SUMS.txt"];

/// The most an APK download may take when the release does not give its size (300 MiB).
pub const MAX_APK_BYTES: u64 = 300 * 1024 * 1024;

/// How many releases the feed asks GitHub for (newest first).
pub const PER_PAGE: usize = 30;

/// The app's repository on GitHub (Encore: `Repo::new("Krifmaz", "Encore")`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// the account (`Krifmaz`)
    pub owner: String,
    /// the repository, which is also the app's name in its file names (`Encore`)
    pub app: String,
}

impl Repo {
    /// The repository `owner/app`.
    pub fn new(owner: impl Into<String>, app: impl Into<String>) -> Repo {
        Repo { owner: owner.into(), app: app.into() }
    }

    /// `owner/app`.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.owner, self.app)
    }

    /// GitHub's list of the newest releases (`https://api.github.com/repos/<owner>/<app>/releases?per_page=30`).
    pub fn releases_api(&self) -> String {
        format!("https://api.github.com/repos/{}/{}/releases?per_page={PER_PAGE}", self.owner, self.app)
    }

    /// The releases page (`https://github.com/<owner>/<app>/releases`).
    pub fn releases_page(&self) -> String {
        format!("https://github.com/{}/{}/releases", self.owner, self.app)
    }

    /// Every file an updater fetches is under this (`https://github.com/<owner>/<app>/releases/download/`).
    pub fn downloads(&self) -> String {
        format!("https://github.com/{}/{}/releases/download/", self.owner, self.app)
    }

    /// Whether `url` is a file of the project's releases (`<downloads><tag>/<file>`), read as an HTTP client
    /// will fetch it: exactly a tag and a file name after the prefix, neither empty, `.` or `..` (also when
    /// written as `%2e%2e`), and neither holding a slash or backslash.
    pub fn is_release_file(&self, url: &str) -> bool {
        let Some(rest) = url.strip_prefix(&self.downloads()) else { return false };
        let rest = rest.split(['?', '#']).next().unwrap_or("");
        let parts: Vec<&str> = rest.split('/').collect();
        parts.len() == 2
            && parts.iter().all(|p| match percent_decode(p) {
                Some(d) => !d.is_empty() && d != "." && d != ".." && !d.contains(['/', '\\']),
                None => false,
            })
    }

    /// The version in an Android app's file name (`<App>-android-<version>.apk`), `None` for any other file.
    pub fn apk_version(&self, name: &str) -> Option<String> {
        let v = name.strip_prefix(&format!("{}-android-", self.app))?.strip_suffix(".apk")?;
        is_release_version(v).then(|| v.to_string())
    }

    /// The release file for `platform` at `version` (the names the release workflow gives them).
    pub fn asset_name(&self, version: &str, platform: Platform) -> String {
        let app = &self.app;
        match platform {
            Platform::Android => format!("{app}-android-{version}.apk"),
            Platform::Windows => format!("{app}-{version}-windows-x86_64.exe"),
            Platform::MacOs => format!("{app}-{version}-macos-universal.dmg"),
            Platform::AppImage => format!("{app}-{version}-linux-x86_64.AppImage"),
            Platform::Deb => format!("{}_{version}_amd64.deb", app.to_lowercase()),
        }
    }

    /// The newer release in a release of GitHub's answer for `platform` (`Ok(None)` when there is none for
    /// this device), with its file and checksum file, both only from the project's release downloads. The
    /// Android app's version is the newest APK's file name; a desktop app's is the tag's, and a release
    /// without this device's file (a server-only release) is not offered. An answer without a tag (GitHub's
    /// `{"message": …}`) is an error in the updaters' words.
    pub fn newest(&self, release: &Value, current: &str, platform: Platform) -> Result<Option<AppRelease>, String> {
        let tag = release["tag_name"].as_str().unwrap_or("").trim();
        if tag.is_empty() {
            let why = release["message"].as_str().filter(|m| !m.trim().is_empty()).unwrap_or("no release found");
            return Err(format!("Couldn't check for updates: {why}"));
        }
        if release["draft"] == true || release["prerelease"] == true {
            return Ok(None);
        }
        let assets: Vec<Asset> = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| Asset {
                name: a["name"].as_str().unwrap_or("").to_string(),
                url: a["browser_download_url"].as_str().unwrap_or("").to_string(),
                size: a["size"].as_u64().unwrap_or(0),
                sha256: digest_hex(a["digest"].as_str().unwrap_or("")),
            })
            .filter(|a| !a.name.is_empty() && self.is_release_file(&a.url))
            .collect();
        let (version, asset) = match platform {
            // the newest APK the release carries (a release without the apps carries the last one over)
            Platform::Android => {
                let Some((v, a)) = assets
                    .iter()
                    .filter_map(|a| Some((self.apk_version(&a.name)?, a)))
                    .max_by(|x, y| kit_server_core::version::compare(&x.0, &y.0))
                else {
                    return Ok(None);
                };
                (v, a.clone())
            }
            _ => {
                let v = tag.trim_start_matches(['v', 'V']).to_string();
                let want = self.asset_name(&v, platform);
                let Some(a) = assets.iter().find(|a| a.name == want) else { return Ok(None) };
                (v, a.clone())
            }
        };
        if !newer_release(&version, current) {
            return Ok(None);
        }
        let (notes, notes_cut) = notes(release["body"].as_str().unwrap_or(""));
        Ok(Some(AppRelease {
            version,
            tag: tag.to_string(),
            page: release["html_url"]
                .as_str()
                .filter(|u| u.starts_with("https://github.com/"))
                .map(String::from)
                .unwrap_or_else(|| self.releases_page()),
            notes,
            notes_cut,
            published_at: release["published_at"].as_str().unwrap_or("").to_string(),
            asset,
            sums: assets.iter().find(|a| SUMS.contains(&a.name.as_str())).cloned(),
        }))
    }
}

/// Which app's file a release carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// the Android app (`<App>-android-<version>.apk`)
    Android,
    /// the Windows installer (`<App>-<version>-windows-x86_64.exe`)
    Windows,
    /// the macOS disk image (`<App>-<version>-macos-universal.dmg`)
    MacOs,
    /// the Linux AppImage (`<App>-<version>-linux-x86_64.AppImage`)
    AppImage,
    /// the Debian package (`<app>_<version>_amd64.deb`)
    Deb,
}

/// A release file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    /// its file name
    pub name: String,
    /// where it is downloaded from (always under [`Repo::downloads`])
    pub url: String,
    /// its size in bytes (0 when GitHub does not say)
    pub size: u64,
    /// GitHub's own SHA-256 of the upload (lowercase hex), "" when it has none
    pub sha256: String,
}

/// A newer release for this device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRelease {
    /// the version the file installs (the APK's from its file name)
    pub version: String,
    /// the release's tag
    pub tag: String,
    /// the release page
    pub page: String,
    /// the release notes, cut to [`NOTES_MAX`] characters
    pub notes: String,
    /// whether the notes were cut
    pub notes_cut: bool,
    /// when it was published (GitHub's time, "" when it does not say)
    pub published_at: String,
    /// the file to install
    pub asset: Asset,
    /// the release's checksum file, when it has one
    pub sums: Option<Asset>,
}

/// Whether a release tag is a full release's (`v<MAJOR>.<MINOR>.<PATCH>`).
pub fn is_full_tag(tag: &str) -> bool {
    tag.trim().strip_prefix('v').is_some_and(is_release_version)
}

/// Whether a release tag is a hotfix's (`hotfix-<version>`).
pub fn is_hotfix_tag(tag: &str) -> bool {
    tag.trim().starts_with(HOTFIX_PREFIX)
}

/// The newest full release (tag `v<MAJOR>.<MINOR>.<PATCH>`, not a draft or prerelease) of GitHub's release
/// list, which is newest first. Hotfixes are passed over: they carry no apps.
pub fn full_release(list: &Value) -> Option<&Value> {
    list.as_array()?.iter().find(|r| r["draft"] != true && r["prerelease"] != true && is_full_tag(r["tag_name"].as_str().unwrap_or("")))
}

/// Release notes as kept: CRLF as LF, cut to [`NOTES_MAX`] characters, trailing space trimmed; and whether
/// they were cut.
pub fn notes(body: &str) -> (String, bool) {
    let body = body.replace("\r\n", "\n");
    let cut = body.chars().count() > NOTES_MAX;
    (body.chars().take(NOTES_MAX).collect::<String>().trim_end().to_string(), cut)
}

/// GitHub's asset digest (`sha256:<hex>`) as lowercase hex, "" when it is missing or another kind.
pub fn digest_hex(digest: &str) -> String {
    match digest.trim().strip_prefix("sha256:") {
        Some(h) if is_hex64(h) => h.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// `SHA256SUMS` (sha256sum format: `<hex>  <name>`, or `<hex> *<name>`, a leading `./` dropped) → name →
/// lowercase hex. Lines that are not one are skipped.
pub fn parse_sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let (hash, name) = l.trim().split_once(char::is_whitespace)?;
            let name = name.trim_start().trim_start_matches('*').trim();
            let name = name.strip_prefix("./").unwrap_or(name);
            (is_hex64(hash) && !name.is_empty()).then(|| (name.to_string(), hash.to_ascii_lowercase()))
        })
        .collect()
}

/// Whether the downloaded file `name` with SHA-256 `sha256_hex` may be installed: the release's `sums` must
/// list it with exactly this hash. A release without a checksum file is never installed. The error is the
/// sentence the updater shows.
pub fn verify(sums: Option<&str>, name: &str, sha256_hex: &str) -> Result<(), String> {
    let Some(text) = sums else {
        return Err(format!("The release has no SHA256SUMS, so {name} could not be checked and was not installed."));
    };
    let want = parse_sums(text)
        .remove(name)
        .ok_or_else(|| format!("{name} is not listed in the release's SHA256SUMS, so it was not installed."))?;
    if !want.eq_ignore_ascii_case(sha256_hex) {
        return Err(format!(
            "The download does not match the release's SHA256SUMS, so it was not installed (got {sha256_hex}, expected {want})."
        ));
    }
    Ok(())
}

/// Whether the downloaded file matches GitHub's own digest of the upload; `Ok` when it does or there is none.
pub fn verify_digest(asset: &Asset, sha256_hex: &str) -> Result<(), String> {
    if asset.sha256.is_empty() || asset.sha256.eq_ignore_ascii_case(sha256_hex) {
        return Ok(());
    }
    Err(format!("The download does not match the checksum GitHub lists for {}, so it was not installed.", asset.name))
}

/// The SHA-256 of `bytes` as lowercase hex (what `sha256sum` prints).
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// 64 hex digits.
fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A URL path part with its `%XX` escapes decoded; `None` for a broken escape or bytes that are not UTF-8.
fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(h, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
