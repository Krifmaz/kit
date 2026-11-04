//! In-app updates for a desktop app: the newest full release of the app on GitHub, compared with
//! this build by semver; the file for this computer; checked against the release's SHA256SUMS; and
//! the Updates block of the app's settings with the words for each step. The app's name, its GitHub
//! repository ([`Feed`]) and its file names are the app's.
//!
//! Hotfix releases (`hotfix-<version>`) carry no desktop files: [`full_release`]
//! passes over them, so they never offer a desktop update (contracts/update-feed.md). A file whose
//! SHA-256 is not the one SHA256SUMS lists is never used ([`verify`]).
//!
//! The kit holds the rules and the words; downloading, installing and restarting stay in the app,
//! which knows its installers.

use crate::settings::{err_text, help, link, switch};
use crate::theme::Btn;
use crate::widgets::{btn_sm, El};
use iced::widget::row;
use iced::Alignment;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// The checksum file of a release.
pub const SUMS: [&str; 2] = ["SHA256SUMS", "SHA256SUMS.txt"];

/// Where an app's releases live on GitHub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Feed {
    /// The app's name in the words ("Demo").
    pub app: &'static str,
    /// The releases page (`https://github.com/<owner>/<repo>/releases`).
    pub page: &'static str,
    /// Every file the updater fetches is under this (`https://github.com/<owner>/<repo>/releases/download/`).
    pub downloads: &'static str,
}

/// A release file.
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    /// Its file name.
    pub name: String,
    /// Where to download it.
    pub url: String,
    /// Its size in bytes.
    pub size: u64,
}

/// A newer release.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Its version, without the `v`.
    pub version: String,
    /// The release page.
    pub page: String,
    /// Its notes.
    pub notes: String,
    /// The file for this computer, when the release has one.
    pub asset: Option<Asset>,
    /// The release's SHA256SUMS, when it has one.
    pub sums: Option<String>,
}

/// Where the updater is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Phase {
    /// Nothing happened yet.
    #[default]
    Idle,
    /// A check runs.
    Checking,
    /// This build is the newest.
    UpToDate,
    /// A newer release is ready to install.
    Available(Release),
    /// Its file is coming in.
    Downloading {
        /// The version.
        version: String,
        /// Bytes so far.
        got: u64,
        /// All bytes (0 when unknown).
        total: u64,
    },
    /// The download is checked against SHA256SUMS.
    Verifying(String),
    /// It is being put in place; the app starts again by itself.
    Installing(String),
    /// It failed, in these words.
    Failed(String),
}

impl Phase {
    /// The updater is working: Check for updates and Install wait.
    pub fn busy(&self) -> bool {
        matches!(self, Phase::Checking | Phase::Downloading { .. } | Phase::Verifying(_) | Phase::Installing(_))
    }
}

/// A byte count in the web's words (`1.5 MB`, decimal units).
pub fn size(bytes: f64) -> String {
    let units = ["B", "kB", "MB", "GB", "TB"];
    let mut v = bytes.max(0.0);
    let mut i = 0;
    while v >= 1000.0 && i < units.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{} B", v as u64)
    } else if v >= 100.0 {
        format!("{} {}", v.round() as u64, units[i])
    } else {
        format!("{v:.1} {}", units[i])
    }
}

/// The words for a phase.
pub fn describe(p: &Phase, app: &str) -> String {
    match p {
        Phase::Idle => String::new(),
        Phase::Checking => "Looking for a new version…".into(),
        Phase::UpToDate => format!("{app} is up to date."),
        Phase::Available(r) => format!("{app} {} is ready to install.", r.version),
        Phase::Downloading { version, got, total } if *total > 0 => {
            format!("Downloading {app} {version}: {} of {}…", size(*got as f64), size(*total as f64))
        }
        Phase::Downloading { version, got, .. } => format!("Downloading {app} {version}: {}…", size(*got as f64)),
        Phase::Verifying(_) => "Checking the download…".into(),
        Phase::Installing(v) => format!("Installing {app} {v}. {app} starts again by itself."),
        Phase::Failed(e) => e.clone(),
    }
}

/// The toast when a new version was found (once per version). `place` says where the app's Updates
/// block is ("Settings → Updates").
pub fn ready_words(app: &str, version: &str, place: &str) -> String {
    format!("{app} {version} is ready to install. {place}.")
}

/// The newest full release (tag v<major>.<minor>.<patch>, not a draft or prerelease) of GitHub's release list,
/// which is newest first. Hotfixes (hotfix-<version>) are the server's only.
pub fn full_release(list: &Value) -> Option<Value> {
    list.as_array()?
        .iter()
        .find(|r| {
            let tag = r.get("tag_name").and_then(Value::as_str).unwrap_or("").trim();
            r.get("draft").and_then(Value::as_bool) != Some(true)
                && r.get("prerelease").and_then(Value::as_bool) != Some(true)
                && tag.strip_prefix('v').is_some_and(|v| v.split('.').count() == 3 && semver::Version::parse(v).is_ok())
        })
        .cloned()
}

/// A newer release than `current` in the GitHub answer (None when there is none), with the file
/// for this computer (`want(version)` names it; None when this computer has no file) and the
/// checksum file, both only from the feed's release downloads.
pub fn newest(release: &Value, current: &str, feed: &Feed, want: impl Fn(&str) -> Option<String>) -> Result<Option<Release>, String> {
    let tag = release.get("tag_name").and_then(Value::as_str).unwrap_or("");
    if tag.is_empty() {
        let why = release.get("message").and_then(Value::as_str).unwrap_or("no release found");
        return Err(format!("Couldn't check for updates: {why}"));
    }
    if release.get("draft").and_then(Value::as_bool) == Some(true) || release.get("prerelease").and_then(Value::as_bool) == Some(true) {
        return Ok(None);
    }
    let version = tag.trim().trim_start_matches(['v', 'V']).to_string();
    let newer = match (semver::Version::parse(&version), semver::Version::parse(current)) {
        (Ok(a), Ok(b)) => a > b,
        _ => false,
    };
    if !newer {
        return Ok(None);
    }
    let assets: Vec<Asset> = release
        .get("assets")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .map(|x| Asset {
                    name: x.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                    url: x.get("browser_download_url").and_then(Value::as_str).unwrap_or("").to_string(),
                    size: x.get("size").and_then(Value::as_u64).unwrap_or(0),
                })
                .filter(|x| !x.name.is_empty() && x.url.starts_with(feed.downloads))
                .collect()
        })
        .unwrap_or_default();
    let want = want(&version);
    let asset = want.as_ref().and_then(|w| assets.iter().find(|a| &a.name == w).cloned());
    // A release published without the desktop apps (a server-only release) carries no file
    // for this computer: there is nothing to update to yet.
    if want.is_some() && asset.is_none() {
        return Ok(None);
    }
    Ok(Some(Release {
        page: release
            .get("html_url")
            .and_then(Value::as_str)
            .filter(|u| u.starts_with("https://github.com/"))
            .unwrap_or(feed.page)
            .to_string(),
        notes: release.get("body").and_then(Value::as_str).unwrap_or("").to_string(),
        asset,
        sums: assets.iter().find(|a| SUMS.contains(&a.name.as_str())).map(|a| a.url.clone()),
        version,
    }))
}

/// `SHA256SUMS` (sha256sum format: `<hex>  <name>`, or `<hex> *<name>`) → name → hex.
pub fn parse_sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|l| {
            let (hash, name) = l.trim().split_once(char::is_whitespace)?;
            let name = name.trim_start().trim_start_matches('*').trim();
            (hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()) && !name.is_empty())
                .then(|| (name.trim_start_matches("./").to_string(), hash.to_ascii_lowercase()))
        })
        .collect()
}

/// Whether a downloaded file may be used: it must be listed in the release's SHA256SUMS (`sums`)
/// with exactly this SHA-256. A release without one is never installed (the release workflow always
/// writes it, so its absence means a broken or hand-edited release).
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

/// The SHA-256 of a file, as lowercase hex.
pub fn hash_file(p: &Path) -> Result<String, String> {
    use sha2::Digest;
    use std::io::Read;
    let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut h = sha2::Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// What the Updates block sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateMsg {
    /// The switch "Look for a new version when <app> starts".
    CheckAtStart(bool),
    /// Check for updates.
    Check,
    /// Install <version> and restart.
    Install,
    /// The releases page.
    OpenReleases,
}

/// The Updates block of the settings (its lines, for the app's settings card): this build's version,
/// the switch, the phase's words (red when it failed), Check for updates, Install … and restart when a
/// newer version was found, and the releases page.
pub fn lines<'a>(app: &str, current: &str, check_at_start: bool, phase: &Phase, found: Option<&str>) -> Vec<El<'a, UpdateMsg>> {
    let busy = phase.busy();
    let line = describe(phase, app);
    let mut out: Vec<El<'a, UpdateMsg>> = vec![
        help(format!("Version {current}")).into(),
        switch(check_at_start, format!("Look for a new version when {app} starts"), UpdateMsg::CheckAtStart),
    ];
    if !line.is_empty() {
        out.push(if matches!(phase, Phase::Failed(_)) { err_text(line).into() } else { help(line).into() });
    }
    let mut actions = row![btn_sm("Check for updates", Btn::Default, (!busy).then_some(UpdateMsg::Check))].spacing(8);
    if let Some(version) = found.filter(|v| !v.is_empty()) {
        actions = actions.push(btn_sm(format!("Install {version} and restart"), Btn::Primary, (!busy).then_some(UpdateMsg::Install)));
    }
    actions = actions.push(link("Releases page", UpdateMsg::OpenReleases));
    out.push(actions.align_y(Alignment::Center).into());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FEED: Feed = Feed {
        app: "Demo",
        page: "https://github.com/example/demo/releases",
        downloads: "https://github.com/example/demo/releases/download/",
    };

    fn release(tag: &str, names: &[&str]) -> Value {
        json!({
            "tag_name": tag,
            "html_url": "https://github.com/example/demo/releases/tag/v0.28.0",
            "body": "notes",
            "assets": names.iter().map(|n| json!({
                "name": n,
                "browser_download_url": format!("{}{tag}/{n}", FEED.downloads),
                "size": 10,
            })).collect::<Vec<_>>(),
        })
    }

    fn appimage(v: &str) -> Option<String> {
        Some(format!("Demo-{v}-linux-x86_64.AppImage"))
    }

    #[test]
    fn hotfixes_and_prereleases_are_passed_over_for_the_newest_full_release() {
        let mut pre = release("v0.36.0", &[]);
        pre["prerelease"] = json!(true);
        let list = json!([release("hotfix-0.35.2.17", &[]), pre, release("v0.35.2", &[]), release("v0.35.1", &[])]);
        assert_eq!(full_release(&list).unwrap()["tag_name"], "v0.35.2");
        assert!(full_release(&json!([release("hotfix-0.35.2.17", &[])])).is_none());
        assert!(full_release(&json!({"message": "Not Found"})).is_none());
    }

    #[test]
    fn a_newer_release_brings_this_computers_file_and_the_sums() {
        let names = ["Demo-0.28.0-windows-x86_64.exe", "Demo-0.28.0-linux-x86_64.AppImage", "demo_0.28.0_amd64.deb", "SHA256SUMS"];
        let r = newest(&release("v0.28.0", &names), "0.27.0", &FEED, appimage).unwrap().unwrap();
        assert_eq!(r.version, "0.28.0");
        assert_eq!(r.notes, "notes");
        assert!(r.sums.as_deref().unwrap().ends_with("/SHA256SUMS"));
        assert_eq!(r.asset.unwrap().name, "Demo-0.28.0-linux-x86_64.AppImage");
        // no file for this computer at all: the release is still offered (the page opens)
        let none = newest(&release("v0.28.0", &names), "0.27.0", &FEED, |_| None).unwrap().unwrap();
        assert_eq!(none.asset, None);
    }

    #[test]
    fn same_older_and_odd_releases_are_not_offered() {
        assert_eq!(newest(&release("v0.27.0", &[]), "0.27.0", &FEED, appimage), Ok(None));
        assert_eq!(newest(&release("0.26.9", &[]), "0.27.0", &FEED, appimage), Ok(None));
        assert_eq!(newest(&release("junk", &[]), "0.27.0", &FEED, appimage), Ok(None));
        let mut pre = release("v0.30.0", &[]);
        pre["prerelease"] = json!(true);
        assert_eq!(newest(&pre, "0.27.0", &FEED, appimage), Ok(None));
        assert!(newest(&json!({ "message": "Not Found" }), "0.27.0", &FEED, appimage).unwrap_err().contains("Not Found"));
        // a newer release without this computer's file (a server-only release) is not offered
        let old_files = release("v0.28.0", &["Demo-0.27.0-linux-x86_64.AppImage", "SHA256SUMS"]);
        assert_eq!(newest(&old_files, "0.27.0", &FEED, appimage), Ok(None));
        // files from anywhere else are ignored
        let mut r = release("v0.28.0", &["SHA256SUMS", "Demo-0.28.0-linux-x86_64.AppImage"]);
        r["assets"][0]["browser_download_url"] = json!("https://evil.example/SHA256SUMS");
        assert_eq!(newest(&r, "0.27.0", &FEED, appimage).unwrap().unwrap().sums, None);
    }

    #[test]
    fn sums_are_read_in_both_sha256sum_styles() {
        let h1 = "a".repeat(64);
        let h2 = "B".repeat(64);
        let text = format!("{h1}  Demo-1-linux-x86_64.AppImage\n{h2} *./demo_1_amd64.deb\nnot a line\n");
        let m = parse_sums(&text);
        assert_eq!(m["Demo-1-linux-x86_64.AppImage"], h1);
        assert_eq!(m["demo_1_amd64.deb"], "b".repeat(64));
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn a_file_that_does_not_match_its_sum_is_refused() {
        let good = "c".repeat(64);
        let sums = format!("{good}  Demo-1.exe\n");
        assert!(verify(Some(&sums), "Demo-1.exe", &good).is_ok());
        assert!(verify(Some(&sums), "Demo-1.exe", &good.to_uppercase()).is_ok());
        assert!(verify(Some(&sums), "Demo-1.exe", &"d".repeat(64)).unwrap_err().contains("does not match"));
        assert!(verify(Some(&sums), "Demo-2.exe", &good).unwrap_err().contains("not listed"));
        assert!(verify(Some(""), "Demo-1.exe", &good).is_err(), "an empty SHA256SUMS lists nothing");
        assert!(verify(None, "Demo-1.exe", "anything").is_err(), "a release without sums is not installed");
    }

    #[test]
    fn files_hash_as_sha256sum_does() {
        let d = std::env::temp_dir().join(format!("kit-iced-hash-{}", std::process::id()));
        std::fs::write(&d, b"abc").unwrap();
        assert_eq!(hash_file(&d).unwrap(), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        std::fs::remove_file(&d).unwrap();
        assert!(hash_file(&d).is_err());
    }

    #[test]
    fn the_words() {
        assert_eq!(describe(&Phase::UpToDate, "Demo"), "Demo is up to date.");
        assert_eq!(
            describe(&Phase::Downloading { version: "0.28.0".into(), got: 2_000_000, total: 40_000_000 }, "Demo"),
            "Downloading Demo 0.28.0: 2.0 MB of 40.0 MB…"
        );
        assert_eq!(describe(&Phase::Installing("1.0.0".into()), "Demo"), "Installing Demo 1.0.0. Demo starts again by itself.");
        assert_eq!(ready_words("Demo", "1.0.0", "Settings → Updates"), "Demo 1.0.0 is ready to install. Settings → Updates.");
        assert_eq!(size(999.0), "999 B");
        assert_eq!(size(1_500_000.0), "1.5 MB");
        assert!(Phase::Checking.busy() && !Phase::UpToDate.busy());
        assert_eq!(lines("Demo", "0.27.0", true, &Phase::Idle, Some("0.28.0")).len(), 3);
        assert_eq!(lines("Demo", "0.27.0", true, &Phase::UpToDate, None).len(), 4);
    }
}
