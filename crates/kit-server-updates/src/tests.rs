//! The feed rules, the server's release rules, the restart and the routes on the kit's test app, over a demo
//! app's feed.

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

/// The demo app's feed: hotfixes, and a file for a phone and four desktop kinds.
fn demo() -> Feed {
    Feed::new("acme", "demo")
        .hotfix_prefix("hotfix-")
        .asset("android", "Demo-android-{version}.apk")
        .asset("windows", "Demo-{version}-windows-x86_64.exe")
        .asset("macos", "Demo-{version}-macos-universal.dmg")
        .asset("linux", "Demo-{version}-linux-x86_64.AppImage")
        .asset("deb", "demo_{version}_amd64.deb")
}

/// A release of `tag` with these files, all from the release downloads.
fn release(tag: &str, names: &[&str]) -> Value {
    json!({
        "tag_name": tag,
        "html_url": "https://github.com/acme/demo/releases/tag/v0.28.0",
        "body": "notes",
        "published_at": "2026-11-02T10:00:00Z",
        "assets": names.iter().map(|n| json!({
            "name": n,
            "browser_download_url": format!("{}{tag}/{n}", demo().downloads()),
            "size": 10,
        })).collect::<Vec<_>>(),
    })
}

#[test]
fn the_repositorys_addresses() {
    let r = demo();
    assert_eq!(r.releases_api(), "https://api.github.com/repos/acme/demo/releases?per_page=30");
    assert_eq!(r.releases_page(), "https://github.com/acme/demo/releases");
    assert_eq!(r.downloads(), "https://github.com/acme/demo/releases/download/");
    let name = |p| r.asset_name("0.28.0", p);
    assert_eq!(name("android").as_deref(), Some("Demo-android-0.28.0.apk"));
    assert_eq!(name("windows").as_deref(), Some("Demo-0.28.0-windows-x86_64.exe"));
    assert_eq!(name("macos").as_deref(), Some("Demo-0.28.0-macos-universal.dmg"));
    assert_eq!(name("linux").as_deref(), Some("Demo-0.28.0-linux-x86_64.AppImage"));
    assert_eq!(name("deb").as_deref(), Some("demo_0.28.0_amd64.deb"));
    assert_eq!(name("ios"), None, "a platform the app did not name");
    // a later pattern for the same platform replaces the earlier one; a pattern without {version} names nothing
    let r = demo().asset("deb", "demo-{version}.deb").asset("plain", "demo.zip");
    assert_eq!(r.asset_name("1.0.0", "deb").as_deref(), Some("demo-1.0.0.deb"));
    assert_eq!((r.asset_name("1.0.0", "plain"), r.asset_version("demo.zip", "plain")), (None, None));
    assert_eq!(r.assets.len(), 6);
}

#[test]
fn hotfixes_and_prereleases_are_passed_over_for_the_newest_full_release() {
    let mut pre = release("v0.36.0", &[]);
    pre["prerelease"] = json!(true);
    let mut draft = release("v0.36.1", &[]);
    draft["draft"] = json!(true);
    let list = json!([release("hotfix-0.35.2.17", &[]), draft, pre, release("v0.35.2", &[]), release("v0.35.1", &[])]);
    let f = demo();
    assert_eq!(f.full_release(&list).unwrap()["tag_name"], "v0.35.2");
    assert!(f.full_release(&json!([release("hotfix-0.35.2.17", &[])])).is_none());
    assert!(f.full_release(&json!([release("v0.36.0-beta", &[]), release("0.35.0", &[])])).is_none());
    assert!(f.full_release(&json!({ "message": "Not Found" })).is_none());
    assert!(f.is_full_tag("v1.2.3") && !f.is_full_tag("v1.2") && !f.is_full_tag("1.2.3") && !f.is_full_tag("v1.2.3.4"));
    assert!(f.is_hotfix_tag("hotfix-0.36.0.17") && !f.is_hotfix_tag("v0.36.0"));
    assert_eq!((f.tag_version("hotfix-0.36.0.17"), f.tag_version("v0.36.0"), f.tag_version("0.36.0")), ("0.36.0.17", "0.36.0", "0.36.0"));
    // the app's own prefixes: bare versions, and no hotfixes at all
    let bare = Feed::new("acme", "demo").release_prefix("");
    assert!(bare.is_full_tag("1.2.3") && !bare.is_full_tag("v1.2.3"));
    assert!(!bare.is_hotfix_tag("hotfix-1.2.3.4"), "no hotfix prefix: no hotfixes");
    assert_eq!(bare.full_release(&json!([{ "tag_name": "1.2.3" }])).unwrap()["tag_name"], "1.2.3");
    let fix = Feed::new("acme", "demo").release_prefix("release-").hotfix_prefix("fix/");
    assert!(fix.is_full_tag("release-2.0.0") && fix.is_hotfix_tag("fix/2.0.0.1"));
    assert_eq!(Feed::new("a", "b").hotfix_prefix("").hotfix_prefix, None);
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
    let r = demo();
    let names = ["Demo-android-0.30.0.apk", "Demo-android-0.29.0.apk", "SHA256SUMS", "Demo-0.31.0-windows-x86_64.exe"];
    // a release made without the apps carries the last APK over under its own tag
    let rel = r.newest(&release("v0.31.0", &names), "0.29.0", "android").unwrap().unwrap();
    assert_eq!((rel.version.as_str(), rel.tag.as_str(), rel.asset.name.as_str()), ("0.30.0", "v0.31.0", "Demo-android-0.30.0.apk"));
    assert_eq!(rel.sums.as_ref().unwrap().name, "SHA256SUMS");
    assert_eq!((rel.notes.as_str(), rel.notes_cut, rel.published_at.as_str()), ("notes", false, "2026-11-02T10:00:00Z"));
    assert_eq!(rel.page, "https://github.com/acme/demo/releases/tag/v0.28.0");
    // already installed, or no APK at all
    assert_eq!(r.newest(&release("v0.31.0", &names), "0.30.0", "android"), Ok(None));
    assert_eq!(r.newest(&release("v0.31.0", &["SHA256SUMS"]), "0.1.0", "android"), Ok(None));
    assert_eq!(r.asset_version("Demo-android-0.30.0.apk", "android").as_deref(), Some("0.30.0"));
    for bad in ["Demo-android-0.30.apk", "Other-android-0.30.0.apk", "Demo-android-0.30.0.apk.sig", "Demo-android-v0.30.0.apk"] {
        assert_eq!(r.asset_version(bad, "android"), None, "{bad}");
    }
    // GitHub's digest of the upload, lowercase
    let mut v = release("v0.31.0", &["Demo-android-0.31.0.apk"]);
    v["assets"][0]["digest"] = json!(format!("sha256:{}", "AB".repeat(32)));
    assert_eq!(r.newest(&v, "0.30.0", "android").unwrap().unwrap().asset.sha256, "ab".repeat(32));
}

#[test]
fn a_newer_release_brings_this_computers_file_and_the_sums() {
    let r = demo();
    let names = [
        "Demo-0.28.0-windows-x86_64.exe",
        "Demo-0.28.0-macos-universal.dmg",
        "Demo-0.28.0-linux-x86_64.AppImage",
        "demo_0.28.0_amd64.deb",
        "Demo_0.28.0_x64-setup.exe",
        "SHA256SUMS",
    ];
    for p in ["windows", "macos", "linux", "deb"] {
        let rel = r.newest(&release("v0.28.0", &names), "0.27.0", p).unwrap().unwrap();
        assert_eq!((rel.version.as_str(), Some(rel.asset.name.clone())), ("0.28.0", r.asset_name("0.28.0", p)));
        assert!(rel.sums.unwrap().url.ends_with("/SHA256SUMS"));
    }
}

#[test]
fn same_older_and_odd_releases_are_not_offered() {
    let r = demo();
    let p = "deb";
    assert_eq!(r.newest(&release("v0.27.0", &["demo_0.27.0_amd64.deb"]), "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&release("0.26.9", &[]), "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&release("junk", &[]), "0.27.0", p), Ok(None));
    let mut pre = release("v0.30.0", &["demo_0.30.0_amd64.deb"]);
    pre["prerelease"] = json!(true);
    assert_eq!(r.newest(&pre, "0.27.0", p), Ok(None));
    assert_eq!(r.newest(&json!({ "message": "Not Found" }), "0.27.0", p), Err("Couldn't check for updates: Not Found".into()));
    assert_eq!(r.newest(&json!({}), "0.27.0", p), Err("Couldn't check for updates: no release found".into()));
    // a newer release without this computer's file (a server-only release) is not offered
    assert_eq!(r.newest(&release("v0.28.0", &["Demo-0.27.0-linux-x86_64.AppImage", "SHA256SUMS"]), "0.27.0", "linux"), Ok(None));
    // files from anywhere else are ignored
    let mut v = release("v0.28.0", &["SHA256SUMS", "demo_0.28.0_amd64.deb"]);
    v["assets"][0]["browser_download_url"] = json!("https://evil.example/SHA256SUMS");
    v["html_url"] = json!("https://evil.example/");
    let rel = r.newest(&v, "0.27.0", p).unwrap().unwrap();
    assert_eq!((rel.sums, rel.page.as_str()), (None, "https://github.com/acme/demo/releases"));
}

#[test]
fn only_files_of_the_projects_releases() {
    let r = demo();
    let d = r.downloads();
    assert!(r.is_release_file(&format!("{d}v0.30.0/Demo-android-0.30.0.apk")));
    assert!(r.is_release_file(&format!("{d}v0.30.0/SHA256SUMS?x=1")));
    for bad in [
        "https://github.com/acme/other/releases/download/v1/x.apk".to_string(),
        "http://github.com/acme/demo/releases/download/v1/x.apk".into(),
        "https://github.com:8443/acme/demo/releases/download/v1/x.apk".into(),
        "https://user@github.com/acme/demo/releases/download/v1/x.apk".into(),
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
    let m = parse_sums(&format!("{h1}  Demo-1-linux-x86_64.AppImage\n{h2} *./demo_1_amd64.deb\nnot a line\n{h1}\n"));
    assert_eq!(m["Demo-1-linux-x86_64.AppImage"], h1);
    assert_eq!(m["demo_1_amd64.deb"], "b".repeat(64));
    assert_eq!(m.len(), 2);
    let good = "c".repeat(64);
    let sums = format!("{good}  Demo-1.exe\n");
    assert!(verify(Some(&sums), "Demo-1.exe", &good).is_ok());
    assert!(verify(Some(&sums), "Demo-1.exe", &good.to_uppercase()).is_ok());
    let bad = "d".repeat(64);
    assert_eq!(
        verify(Some(&sums), "Demo-1.exe", &bad),
        Err(format!("The download does not match the release's SHA256SUMS, so it was not installed (got {bad}, expected {good})."))
    );
    assert_eq!(
        verify(Some(&sums), "Demo-2.exe", &good),
        Err("Demo-2.exe is not listed in the release's SHA256SUMS, so it was not installed.".into())
    );
    assert!(verify(Some(""), "Demo-1.exe", &good).is_err(), "an empty SHA256SUMS lists nothing");
    assert_eq!(
        verify(None, "Demo-1.exe", "x"),
        Err("The release has no SHA256SUMS, so Demo-1.exe could not be checked and was not installed.".into())
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
    assert!(SUMS.contains(&"SHA256SUMS.txt") && DEFAULT_MAX_DOWNLOAD_BYTES == 314_572_800);
    assert_eq!((demo().max_download_bytes, demo().max_download_bytes(5).max_download_bytes), (314_572_800, 5));
}

#[test]
fn a_release_is_kept_with_its_notes_cut_at_4000_characters() {
    let feed = demo();
    let body = format!("## What changed\r\n\r\n{}", "ü".repeat(5000));
    let v = json!({ "tag_name": "v0.29.0", "body": body, "html_url": "https://github.com/acme/demo/releases/tag/v0.29.0", "published_at": "2026-11-02T10:00:00Z" });
    let r = parse_release(&v, 1_800_000_000.0, &feed).unwrap();
    assert_eq!((r["tag"].as_str(), r["latest"].as_str(), r["hotfix"].as_bool()), (Some("v0.29.0"), Some("0.29.0"), Some(false)));
    assert_eq!(r["notes"].as_str().unwrap().chars().count(), 4000);
    assert!(r["notes"].as_str().unwrap().starts_with("## What changed\n\n"), "CRLF becomes LF");
    assert_eq!((r["notes_cut"].as_bool(), r["checked_at"].as_i64()), (Some(true), Some(1_800_000_000)));
    assert!(parse_release(&json!({ "message": "Not Found" }), 0.0, &feed).is_none());
    let r = parse_release(&json!({ "tag_name": "0.29.1" }), 0.0, &feed).unwrap();
    assert_eq!((r["notes"].as_str(), r["notes_cut"].as_bool()), (Some(""), Some(false)));
    assert_eq!(r["url"], json!("https://github.com/acme/demo/releases/tag/0.29.1"));
    assert_eq!(parse_release(&json!({ "tag_name": "hotfix-0.35.2.17" }), 0.0, &feed).unwrap()["latest"], json!("0.35.2.17"));
}

#[test]
fn the_hotfix_channel_takes_the_highest_of_releases_and_hotfixes() {
    let feed = demo();
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
    let r = pick_newest(&list, Channel::Hotfix, 0.0, &feed).unwrap();
    assert_eq!((r["tag"].as_str(), r["latest"].as_str(), r["hotfix"].as_bool()), (Some("hotfix-0.35.2.17"), Some("0.35.2.17"), Some(true)));
    let list = json!([{ "tag_name": "v0.35.3" }, { "tag_name": "hotfix-0.35.2.17", "prerelease": true }]);
    assert_eq!(pick_newest(&list, Channel::Hotfix, 0.0, &feed).unwrap()["latest"], json!("0.35.3"));
    assert!(pick_newest(&json!([]), Channel::Hotfix, 0.0, &feed).is_none());
    assert!(is_newer("0.35.3", "0.35.2.17", false));
    assert!(is_newer("0.35.2.17", "0.35.2", false));
    assert!(!is_newer("0.35.2", "0.35.2.17", false), "the stable channel does not go back from a hotfix");
    assert!(is_newer("0.26.1", "2.0.0", true), "a dev build");
    assert!(!is_newer("", "0.28.0", true));
}

#[test]
fn the_shared_part_of_the_update_answer() {
    let feed = demo();
    let rel =
        parse_release(&json!({ "tag_name": "v0.29.0", "body": "notes", "published_at": "2026-11-02T10:00:00Z" }), 1_800_000_000.0, &feed)
            .unwrap();
    let a = answer(Some(&rel), "0.28.0", false, &feed);
    assert_eq!(
        a,
        json!({ "current": "0.28.0", "dev": false, "latest": "0.29.0", "tag": "v0.29.0", "hotfix": false, "is_newer": true, "notes": "notes",
                "notes_cut": false, "url": "https://github.com/acme/demo/releases/tag/v0.29.0", "published_at": "2026-11-02T10:00:00Z",
                "checked_at": 1_800_000_000, "restart": "ready" })
    );
    let a = answer(None, "0.28.0", false, &feed);
    assert_eq!(
        (a["latest"].clone(), a["is_newer"].clone(), a["url"].clone()),
        (Value::Null, json!(false), json!("https://github.com/acme/demo/releases"))
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
        Ok(answer(None, "0.28.0", false, &demo()))
    }
    async fn update_now(&self, who: &Account, tag: Option<String>) -> Result<Value> {
        self.asked.lock().unwrap().push(format!("update {} {tag:?}", who.username));
        match tag.as_deref() {
            Some("0.27.0") => Err(err(400, "Demo 0.27.0 can't update itself; pick 0.28.0 or newer")),
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
    assert_eq!(call(&app, "GET", "/api/update", 2, None).await, (StatusCode::FORBIDDEN, json!({ "detail": "Admin only" })));
    assert_eq!(call(&app, "POST", "/api/update", 0, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "POST", "/api/restart", 2, None).await.0, StatusCode::FORBIDDEN);
    assert!(asked().is_empty());
    let (s, v) = call(&app, "GET", "/api/update", 1, None).await;
    assert_eq!((s, v["restart"].as_str()), (StatusCode::OK, Some("ready")));
    for (q, force) in [("?force=1", true), ("?force=true", true), ("?force=yes", true), ("?force=0", false), ("", false)] {
        call(&app, "GET", &format!("/api/update{q}"), 1, None).await;
        assert_eq!(asked().last().unwrap(), &format!("status force={force}"), "{q}");
    }
    // Update now: 202 with the app's answer; a tag, or none; the app's refusal as it is
    let (s, v) = call(&app, "POST", "/api/update", 1, None).await;
    assert_eq!((s, v["ok"].as_bool()), (StatusCode::ACCEPTED, Some(true)));
    call(&app, "POST", "/api/update", 1, Some(json!({ "tag": "0.29.0" }))).await;
    assert_eq!(asked(), vec!["update admin None", "update admin Some(\"0.29.0\")"]);
    assert_eq!(
        call(&app, "POST", "/api/update", 1, Some(json!({ "tag": "0.27.0" }))).await,
        (StatusCode::BAD_REQUEST, json!({ "detail": "Demo 0.27.0 can't update itself; pick 0.28.0 or newer" }))
    );
    asked();
    // Restart: 202 at once, the restart after the delay; asking twice is fine
    assert_eq!(call(&app, "POST", "/api/restart", 1, None).await, (StatusCode::ACCEPTED, json!({ "ok": true, "message": "Restarting" })));
    assert!(!app.restart.is_requested());
    tokio::time::timeout(Duration::from_secs(3), app.restart.requested()).await.unwrap();
    assert_eq!(call(&app, "POST", "/api/restart", 1, None).await.0, StatusCode::ACCEPTED);
    assert_eq!(asked(), vec!["restart admin"], "planned once");
}

/// The stable channel takes the highest full release from the list, not GitHub's "Latest".
#[test]
fn the_highest_release_wins_over_githubs_latest() {
    let feed = demo();
    let list = json!([
        { "tag_name": "v0.42.6" },
        { "tag_name": "v0.42.5" },
        { "tag_name": "hotfix-0.42.5.191" },
        { "tag_name": "hotfix-0.42.3.183" },
    ]);
    for channel in [Channel::Stable, Channel::Hotfix] {
        let r = pick_newest(&list, channel, 0.0, &feed).unwrap();
        assert_eq!(r["latest"], json!("0.42.6"), "{channel:?}");
        assert!(is_newer(r["latest"].as_str().unwrap(), "0.42.5.191", false));
    }
    let list = json!([{ "tag_name": "hotfix-0.42.6.195" }, { "tag_name": "v0.42.6" }]);
    assert_eq!(pick_newest(&list, Channel::Stable, 0.0, &feed).unwrap()["latest"], json!("0.42.6.195"));
    assert_eq!(pick_newest(&list, Channel::Hotfix, 0.0, &feed).unwrap()["latest"], json!("0.42.6.195"));
    let list = json!([
        { "tag_name": "hotfix-0.42.6.196", "prerelease": true },
        { "tag_name": "v0.43.0-beta", "prerelease": true },
        { "tag_name": "v0.43.0", "draft": true },
        { "tag_name": "v0.42.6" },
    ]);
    assert_eq!(pick_newest(&list, Channel::Stable, 0.0, &feed).unwrap()["latest"], json!("0.42.6"));
    assert_eq!(pick_newest(&list, Channel::Hotfix, 0.0, &feed).unwrap()["latest"], json!("0.42.6.196"));
}

#[tokio::test]
async fn the_app_can_move_the_routes() {
    let app = App { host: TestHost::new(), asked: Arc::default(), restart: Restart::default() };
    let r = router_with::<App>(UpdatePaths { update: "/admin/version".into(), restart: "/admin/reboot".into() }).with_state(app.clone());
    let req = |method: &str, uri: &str| Request::builder().method(method).uri(uri).header("x-test-user", "1").body(Body::empty()).unwrap();
    assert_eq!(r.clone().oneshot(req("GET", "/admin/version")).await.unwrap().status(), StatusCode::OK);
    assert_eq!(r.clone().oneshot(req("POST", "/admin/reboot")).await.unwrap().status(), StatusCode::ACCEPTED);
    assert_eq!(r.oneshot(req("GET", "/api/update")).await.unwrap().status(), StatusCode::NOT_FOUND, "the default path is not served");
    assert_eq!(UpdatePaths::default(), UpdatePaths { update: "/api/update".into(), restart: "/api/restart".into() });
}
