//! The feed rules (Encore's Android and desktop updater tests), the server's release rules (Encore's
//! `update_tests.rs`), the restart and the routes on the kit's test app.

use super::feed::*;
use super::server::*;
use super::*;
use axum::body::Body;
use axum::http::request::Parts;
use axum::http::Request;
use kit_server_core::testing::TestHost;
use kit_server_core::{err, Db};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tower::ServiceExt;

fn encore() -> Repo {
    Repo::new("Krifmaz", "Encore")
}

/// A release of `tag` with these files, all from the release downloads.
fn release(tag: &str, names: &[&str]) -> Value {
    json!({
        "tag_name": tag,
        "html_url": "https://github.com/Krifmaz/Encore/releases/tag/v0.28.0",
        "body": "notes",
        "published_at": "2026-11-02T10:00:00Z",
        "assets": names.iter().map(|n| json!({
            "name": n,
            "browser_download_url": format!("{}{tag}/{n}", encore().downloads()),
            "size": 10,
        })).collect::<Vec<_>>(),
    })
}

#[test]
fn the_repositorys_addresses() {
    let r = encore();
    assert_eq!(r.releases_api(), "https://api.github.com/repos/Krifmaz/Encore/releases?per_page=30");
    assert_eq!(r.releases_page(), "https://github.com/Krifmaz/Encore/releases");
    assert_eq!(r.downloads(), "https://github.com/Krifmaz/Encore/releases/download/");
    assert_eq!(r.asset_name("0.28.0", Platform::Android), "Encore-android-0.28.0.apk");
    assert_eq!(r.asset_name("0.28.0", Platform::Windows), "Encore-0.28.0-windows-x86_64.exe");
    assert_eq!(r.asset_name("0.28.0", Platform::MacOs), "Encore-0.28.0-macos-universal.dmg");
    assert_eq!(r.asset_name("0.28.0", Platform::AppImage), "Encore-0.28.0-linux-x86_64.AppImage");
    assert_eq!(r.asset_name("0.28.0", Platform::Deb), "encore_0.28.0_amd64.deb");
}

#[test]
fn hotfixes_and_prereleases_are_passed_over_for_the_newest_full_release() {
    let mut pre = release("v0.36.0", &[]);
    pre["prerelease"] = json!(true);
    let mut draft = release("v0.36.1", &[]);
    draft["draft"] = json!(true);
    let list = json!([release("hotfix-0.35.2.17", &[]), draft, pre, release("v0.35.2", &[]), release("v0.35.1", &[])]);
    assert_eq!(full_release(&list).unwrap()["tag_name"], "v0.35.2");
    assert!(full_release(&json!([release("hotfix-0.35.2.17", &[])])).is_none());
    assert!(full_release(&json!([release("v0.36.0-beta", &[]), release("0.35.0", &[])])).is_none());
    assert!(full_release(&json!({ "message": "Not Found" })).is_none());
    assert!(is_full_tag("v1.2.3") && !is_full_tag("v1.2") && !is_full_tag("1.2.3") && !is_full_tag("v1.2.3.4"));
    assert!(is_hotfix_tag("hotfix-0.36.0.17") && !is_hotfix_tag("v0.36.0"));
}

#[test]
fn versions_compare_by_number_and_a_dev_build_takes_every_release() {
    assert!(newer_release("0.30.1", "0.30.0"));
    assert!(newer_release("0.30.10", "0.30.9"), "numbers, not text");
    assert!(!newer_release("0.30.0", "0.30.0"));
    assert!(!newer_release("0.29.9", "0.30.0"));
    assert!(newer_release("0.1.0", "0.0.1-dev"));
    assert!(newer_release("0.1.0", "dev.12"));
    assert!(!newer_release("junk", "0.0.1-dev"));
    assert!(is_release_version(" 1.2.3 ") && !is_release_version("1.2.3-dev") && !is_release_version("1..3"));
}

#[test]
fn the_phone_takes_the_newest_apk_by_its_file_name() {
    let r = encore();
    let names = ["Encore-android-0.30.0.apk", "Encore-android-0.29.0.apk", "SHA256SUMS", "Encore-0.31.0-windows-x86_64.exe"];
    // a release made without the apps carries the last APK over under its own tag
    let rel = r.newest(&release("v0.31.0", &names), "0.29.0", Platform::Android).unwrap().unwrap();
    assert_eq!((rel.version.as_str(), rel.tag.as_str(), rel.asset.name.as_str()), ("0.30.0", "v0.31.0", "Encore-android-0.30.0.apk"));
    assert_eq!(rel.sums.as_ref().unwrap().name, "SHA256SUMS");
    assert_eq!((rel.notes.as_str(), rel.notes_cut, rel.published_at.as_str()), ("notes", false, "2026-11-02T10:00:00Z"));
    assert_eq!(rel.page, "https://github.com/Krifmaz/Encore/releases/tag/v0.28.0");
    // already installed, or no APK at all
    assert_eq!(r.newest(&release("v0.31.0", &names), "0.30.0", Platform::Android), Ok(None));
    assert_eq!(r.newest(&release("v0.31.0", &["SHA256SUMS"]), "0.1.0", Platform::Android), Ok(None));
    assert_eq!(r.apk_version("Encore-android-0.30.0.apk").as_deref(), Some("0.30.0"));
    for bad in ["Encore-android-0.30.apk", "Other-android-0.30.0.apk", "Encore-android-0.30.0.apk.sig", "Encore-android-v0.30.0.apk"] {
        assert_eq!(r.apk_version(bad), None, "{bad}");
    }
    // GitHub's digest of the upload, lowercase
    let mut v = release("v0.31.0", &["Encore-android-0.31.0.apk"]);
    v["assets"][0]["digest"] = json!(format!("sha256:{}", "AB".repeat(32)));
    assert_eq!(r.newest(&v, "0.30.0", Platform::Android).unwrap().unwrap().asset.sha256, "ab".repeat(32));
}

#[test]
fn a_newer_release_brings_this_computers_file_and_the_sums() {
    let r = encore();
    let names = [
        "Encore-0.28.0-windows-x86_64.exe",
        "Encore-0.28.0-macos-universal.dmg",
        "Encore-0.28.0-linux-x86_64.AppImage",
        "encore_0.28.0_amd64.deb",
        "Encore_0.28.0_x64-setup.exe",
        "SHA256SUMS",
    ];
    for p in [Platform::Windows, Platform::MacOs, Platform::AppImage, Platform::Deb] {
        let rel = r.newest(&release("v0.28.0", &names), "0.27.0", p).unwrap().unwrap();
        assert_eq!((rel.version.as_str(), rel.asset.name.clone()), ("0.28.0", r.asset_name("0.28.0", p)));
        assert!(rel.sums.unwrap().url.ends_with("/SHA256SUMS"));
    }
}

#[test]
fn same_older_and_odd_releases_are_not_offered() {
    let r = encore();
    let p = Platform::Deb;
    assert_eq!(r.newest(&release("v0.27.0", &["encore_0.27.0_amd64.deb"]), "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&release("0.26.9", &[]), "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&release("junk", &[]), "0.27.0", p), Ok(None));
    let mut pre = release("v0.30.0", &["encore_0.30.0_amd64.deb"]);
    pre["prerelease"] = json!(true);
    assert_eq!(r.newest(&pre, "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&json!({ "message": "Not Found" }), "0.27.0", p), Err("Couldn't check for updates: Not Found".into()));
    assert_eq!(r.newest(&json!({}), "0.27.0", p), Err("Couldn't check for updates: no release found".into()));
    // a newer release without this computer's file (a server-only release) is not offered
    assert_eq!(
        r.newest(&release("v0.28.0", &["Encore-0.27.0-linux-x86_64.AppImage", "SHA256SUMS"]), "0.27.0", Platform::AppImage),
        Ok(None)
    );
    // files from anywhere else are ignored
    let mut v = release("v0.28.0", &["SHA256SUMS", "encore_0.28.0_amd64.deb"]);
    v["assets"][0]["browser_download_url"] = json!("https://evil.example/SHA256SUMS");
    v["html_url"] = json!("https://evil.example/");
    let rel = r.newest(&v, "0.27.0", p).unwrap().unwrap();
    assert_eq!((rel.sums, rel.page.as_str()), (None, "https://github.com/Krifmaz/Encore/releases"));
}

#[test]
fn only_files_of_the_projects_releases() {
    let r = encore();
    let d = r.downloads();
    assert!(r.is_release_file(&format!("{d}v0.30.0/Encore-android-0.30.0.apk")));
    assert!(r.is_release_file(&format!("{d}v0.30.0/SHA256SUMS?x=1")));
    for bad in [
        "https://github.com/Krifmaz/Other/releases/download/v1/x.apk".to_string(),
        "http://github.com/Krifmaz/Encore/releases/download/v1/x.apk".into(),
        "https://github.com:8443/Krifmaz/Encore/releases/download/v1/x.apk".into(),
        "https://user@github.com/Krifmaz/Encore/releases/download/v1/x.apk".into(),
        format!("{d}v1/../../../Other/x.apk"),
        format!("{d}v1/%2e%2e/x.apk"),
        format!("{d}%2E%2E/x.apk"),
        format!("{d}v1/a%2fb.apk"),
        format!("{d}v1/a%5cb.apk"),
        format!("{d}v1/x.apk/more"),
        format!("{d}v1/"),
        format!("{d}x.apk"),
        format!("{d}v1/x%zz.apk"),
    ] {
        assert!(!r.is_release_file(&bad), "{bad}");
    }
}

#[test]
fn sums_are_read_in_both_sha256sum_styles_and_checked() {
    let (h1, h2) = ("a".repeat(64), "B".repeat(64));
    let m = parse_sums(&format!("{h1}  Encore-1-linux-x86_64.AppImage\n{h2} *./encore_1_amd64.deb\nnot a line\n{h1}\n"));
    assert_eq!(m["Encore-1-linux-x86_64.AppImage"], h1);
    assert_eq!(m["encore_1_amd64.deb"], "b".repeat(64));
    assert_eq!(m.len(), 2);
    let good = "c".repeat(64);
    let sums = format!("{good}  Encore-1.exe\n");
    assert!(verify(Some(&sums), "Encore-1.exe", &good).is_ok());
    assert!(verify(Some(&sums), "Encore-1.exe", &good.to_uppercase()).is_ok());
    let bad = "d".repeat(64);
    assert_eq!(
        verify(Some(&sums), "Encore-1.exe", &bad),
        Err(format!("The download does not match the release's SHA256SUMS, so it was not installed (got {bad}, expected {good})."))
    );
    assert_eq!(
        verify(Some(&sums), "Encore-2.exe", &good),
        Err("Encore-2.exe is not listed in the release's SHA256SUMS, so it was not installed.".into())
    );
    assert!(verify(Some(""), "Encore-1.exe", &good).is_err(), "an empty SHA256SUMS lists nothing");
    assert_eq!(
        verify(None, "Encore-1.exe", "x"),
        Err("The release has no SHA256SUMS, so Encore-1.exe could not be checked and was not installed.".into())
    );
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    assert_eq!(digest_hex("sha1:abc"), "");
    assert_eq!(digest_hex(&format!("sha256:{}", "x".repeat(64))), "");
    let a = Asset { name: "x.apk".into(), url: String::new(), size: 0, sha256: good.clone() };
    assert!(verify_digest(&a, &good.to_uppercase()).is_ok());
    assert_eq!(
        verify_digest(&a, &bad),
        Err("The download does not match the checksum GitHub lists for x.apk, so it was not installed.".into())
    );
    assert!(verify_digest(&Asset { sha256: String::new(), ..a }, &bad).is_ok(), "no digest from GitHub");
    assert!(SUMS.contains(&"SHA256SUMS.txt") && MAX_APK_BYTES == 314_572_800);
}

#[test]
fn a_release_is_kept_with_its_notes_cut_at_4000_characters() {
    let repo = encore();
    let body = format!("## What changed\r\n\r\n{}", "ü".repeat(5000));
    let v = json!({ "tag_name": "v0.29.0", "body": body, "html_url": "https://github.com/Krifmaz/Encore/releases/tag/v0.29.0", "published_at": "2026-11-02T10:00:00Z" });
    let r = parse_release(&v, 1_800_000_000.0, &repo).unwrap();
    assert_eq!((r["tag"].as_str(), r["latest"].as_str(), r["hotfix"].as_bool()), (Some("v0.29.0"), Some("0.29.0"), Some(false)));
    assert_eq!(r["notes"].as_str().unwrap().chars().count(), 4000);
    assert!(r["notes"].as_str().unwrap().starts_with("## What changed\n\n"), "CRLF becomes LF");
    assert_eq!((r["notes_cut"].as_bool(), r["checked_at"].as_i64()), (Some(true), Some(1_800_000_000)));
    assert!(parse_release(&json!({ "message": "Not Found" }), 0.0, &repo).is_none());
    let r = parse_release(&json!({ "tag_name": "0.29.1" }), 0.0, &repo).unwrap();
    assert_eq!((r["notes"].as_str(), r["notes_cut"].as_bool()), (Some(""), Some(false)));
    assert_eq!(r["url"], json!("https://github.com/Krifmaz/Encore/releases/tag/0.29.1"));
    assert_eq!(parse_release(&json!({ "tag_name": "hotfix-0.35.2.17" }), 0.0, &repo).unwrap()["latest"], json!("0.35.2.17"));
}

#[test]
fn the_hotfix_channel_takes_the_highest_of_releases_and_hotfixes() {
    let repo = encore();
    assert_eq!(Channel::parse(" Hotfix "), Channel::Hotfix);
    for s in ["", "stable", "nightly"] {
        assert_eq!(Channel::parse(s), Channel::Stable, "{s:?}");
    }
    assert_eq!((Channel::Stable.name(), Channel::Hotfix.name()), ("stable", "hotfix"));
    let list = json!([
        { "tag_name": "hotfix-0.35.2.18", "prerelease": true, "draft": true },
        { "tag_name": "hotfix-0.35.2.17", "prerelease": true },
        { "tag_name": "v0.36.0-beta", "prerelease": true },
        { "tag_name": "v0.35.2" },
        { "tag_name": "hotfix-0.35.1.9", "prerelease": true },
    ]);
    let r = pick_newest(&list, 0.0, &repo).unwrap();
    assert_eq!((r["tag"].as_str(), r["latest"].as_str(), r["hotfix"].as_bool()), (Some("hotfix-0.35.2.17"), Some("0.35.2.17"), Some(true)));
    let list = json!([{ "tag_name": "v0.35.3" }, { "tag_name": "hotfix-0.35.2.17", "prerelease": true }]);
    assert_eq!(pick_newest(&list, 0.0, &repo).unwrap()["latest"], json!("0.35.3"));
    assert!(pick_newest(&json!([]), 0.0, &repo).is_none());
    assert!(is_newer("0.35.3", "0.35.2.17", false));
    assert!(is_newer("0.35.2.17", "0.35.2", false));
    assert!(!is_newer("0.35.2", "0.35.2.17", false), "the stable channel does not go back from a hotfix");
    assert!(is_newer("0.26.1", "2.0.0", true), "a dev build");
    assert!(!is_newer("", "0.28.0", true));
}

#[test]
fn the_shared_part_of_the_update_answer() {
    let repo = encore();
    let rel =
        parse_release(&json!({ "tag_name": "v0.29.0", "body": "notes", "published_at": "2026-11-02T10:00:00Z" }), 1_800_000_000.0, &repo)
            .unwrap();
    let a = answer(Some(&rel), "0.28.0", false, &repo);
    assert_eq!(
        a,
        json!({ "current": "0.28.0", "dev": false, "latest": "0.29.0", "tag": "v0.29.0", "hotfix": false, "is_newer": true, "notes": "notes",
                "notes_cut": false, "url": "https://github.com/Krifmaz/Encore/releases/tag/v0.29.0", "published_at": "2026-11-02T10:00:00Z",
                "checked_at": 1_800_000_000, "restart": "ready" })
    );
    let a = answer(None, "0.28.0", false, &repo);
    assert_eq!(
        (a["latest"].clone(), a["is_newer"].clone(), a["url"].clone()),
        (Value::Null, json!(false), json!("https://github.com/Krifmaz/Encore/releases"))
    );
}

/// "Restart server" plans the restart and does not exit: the app's server loop does that.
#[tokio::test]
async fn a_restart_is_scheduled_once() {
    let r = Restart::default();
    assert!(!r.is_requested());
    assert!(r.schedule(Duration::from_millis(30)));
    assert!(!r.schedule(Duration::from_millis(30)), "one restart at a time");
    assert!(!r.is_requested(), "not before the delay: the answer gets out first");
    tokio::time::timeout(Duration::from_secs(2), r.requested()).await.expect("the restart comes after the delay");
    assert!(r.is_requested());
    tokio::time::timeout(Duration::from_millis(100), r.requested()).await.unwrap();
    assert_eq!(EXIT_RESTART, 75);
}

/// The test app: what was asked for.
#[derive(Clone)]
struct App {
    host: TestHost,
    asked: Arc<Mutex<Vec<String>>>,
    restart: Restart,
}

impl Host for App {
    fn db(&self) -> &dyn Db {
        self.host.db()
    }
    async fn signed_in(&self, parts: &mut Parts) -> Result<Account> {
        self.host.signed_in(parts).await
    }
}

impl Updates for App {
    async fn update_status(&self, force: bool) -> Result<Value> {
        self.asked.lock().unwrap().push(format!("status force={force}"));
        Ok(answer(None, "0.28.0", false, &encore()))
    }
    async fn update_now(&self, who: &Account, tag: Option<String>) -> Result<Value> {
        self.asked.lock().unwrap().push(format!("update {} {tag:?}", who.username));
        match tag.as_deref() {
            Some("0.27.0") => Err(err(400, "Encore 0.27.0 can't update itself; pick 0.28.0 or newer")),
            _ => Ok(json!({ "ok": true, "message": "Updating to 0.29.0; the server restarts in a moment" })),
        }
    }
    fn restart(&self, who: &Account) {
        if self.restart.schedule(RESTART_DELAY) {
            self.asked.lock().unwrap().push(format!("restart {}", who.username));
        }
    }
}

async fn call(app: &App, method: &str, uri: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
    let b = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string());
    let req = match body {
        Some(j) => b.header("content-type", "application/json").body(Body::from(j.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    };
    let resp = router::<App>().with_state(app.clone()).oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn the_update_routes() {
    let app = App { host: TestHost::new(), asked: Arc::default(), restart: Restart::default() };
    let asked = || std::mem::take(&mut *app.asked.lock().unwrap());
    assert_eq!(call(&app, "GET", "/api/system/update", 2, None).await, (StatusCode::FORBIDDEN, json!({ "detail": "Admin only" })));
    assert_eq!(call(&app, "POST", "/api/system/update", 0, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "POST", "/api/system/restart", 2, None).await.0, StatusCode::FORBIDDEN);
    assert!(asked().is_empty());
    let (s, v) = call(&app, "GET", "/api/system/update", 1, None).await;
    assert_eq!((s, v["restart"].as_str()), (StatusCode::OK, Some("ready")));
    for (q, force) in [("?force=1", true), ("?force=true", true), ("?force=yes", true), ("?force=0", false), ("", false)] {
        call(&app, "GET", &format!("/api/system/update{q}"), 1, None).await;
        assert_eq!(asked().last().unwrap(), &format!("status force={force}"), "{q}");
    }
    // Update now: 202 with the app's answer; a tag, or none; the app's refusal as it is
    let (s, v) = call(&app, "POST", "/api/system/update", 1, None).await;
    assert_eq!((s, v["ok"].as_bool()), (StatusCode::ACCEPTED, Some(true)));
    call(&app, "POST", "/api/system/update", 1, Some(json!({ "tag": "0.29.0" }))).await;
    assert_eq!(asked(), vec!["update admin None", "update admin Some(\"0.29.0\")"]);
    assert_eq!(
        call(&app, "POST", "/api/system/update", 1, Some(json!({ "tag": "0.27.0" }))).await,
        (StatusCode::BAD_REQUEST, json!({ "detail": "Encore 0.27.0 can't update itself; pick 0.28.0 or newer" }))
    );
    asked();
    // Restart: 202 at once, the restart after the delay; asking twice is fine
    assert_eq!(
        call(&app, "POST", "/api/system/restart", 1, None).await,
        (StatusCode::ACCEPTED, json!({ "ok": true, "message": "Restarting" }))
    );
    assert!(!app.restart.is_requested());
    tokio::time::timeout(Duration::from_secs(3), app.restart.requested()).await.unwrap();
    assert_eq!(call(&app, "POST", "/api/system/restart", 1, None).await.0, StatusCode::ACCEPTED);
    assert_eq!(asked(), vec!["restart admin"], "planned once");
}
