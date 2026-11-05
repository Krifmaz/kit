//! The server's part of Update now against the mock Docker: tags, the pull, the helper's start; the probe's
//! reasons in plain words; and the record read back from an ended helper.
use super::*;
use crate::probe::{probe_with, SelfUpdate};
use crate::swap::tests::{app, fixture, serve_mock, sock_dir, HELPER_ID, IMAGE, SCHEMA};
use serde_json::json;

#[test]
fn image_tags_are_checked() {
    let app = app().first_self_updating("0.28.0");
    assert_eq!(check_tag(&app, "0.29.0").as_deref(), Ok("0.29.0"));
    assert_eq!(check_tag(&app, "v0.29.0").as_deref(), Ok("0.29.0"), "the images are tagged without the v");
    assert_eq!(check_tag(&app, "latest").as_deref(), Ok("latest"));
    assert_eq!(check_tag(&app, "0.28.0").as_deref(), Ok("0.28.0"));
    assert_eq!(check_tag(&app, "0.35.2.17").as_deref(), Ok("0.35.2.17"));
    for bad in ["", "0.29.0;rm", "../x", "a b", "x:y", &"a".repeat(65), "ghcr.io/other/image"] {
        assert_eq!(check_tag(&app, bad), Err("Not a valid image tag".to_string()), "{bad}");
    }
    assert_eq!(
        check_tag(&app, "0.27.0"),
        Err("MyApp 0.27.0 can't update itself; pick 0.28.0 or newer".into()),
        "older images have no helper"
    );
    assert_eq!(check_tag(&crate::swap::tests::app(), "0.1.0").as_deref(), Ok("0.1.0"), "no oldest release set: every tag");
}

#[test]
fn the_container_id_is_found_in_mountinfo() {
    use crate::docker::container_id_from_mountinfo;
    let id = "784f60a526476812415c46b2613ebae100e5df0b6fc23a4b255d19ccf653c7a5";
    let text = format!(
        "1236 1218 0:322 / / rw,relatime master:500 - overlay overlay rw,lowerdir=/var/lib/docker/overlay2/l/ABC\n\
         1250 1236 254:1 /var/lib/docker/containers/{id}/resolv.conf /etc/resolv.conf rw,relatime - ext4 /dev/vda1 rw\n\
         1251 1236 254:1 /var/lib/docker/containers/{id}/hostname /etc/hostname rw,relatime - ext4 /dev/vda1 rw\n"
    );
    assert_eq!(container_id_from_mountinfo(&text).as_deref(), Some(id));
    assert_eq!(container_id_from_mountinfo("22 1 254:1 / / rw - ext4 /dev/vda1 rw\n"), None);
    assert_eq!(container_id_from_mountinfo("1 1 0:1 /containers/short/hostname /etc/hostname rw - ext4 x rw"), None);
}

/// Each reason in plain words, in the order they are checked.
#[tokio::test]
async fn the_probe_says_why_not() {
    let app = app();
    let dir = sock_dir("probe");
    let missing = dir.join("none.sock");
    assert_eq!(probe_with(&app, &missing, "abc", "").await.reason.as_deref(), Some("The Docker socket is not mounted"));
    std::fs::write(dir.join("file.sock"), "").unwrap();
    let r = probe_with(&app, &dir.join("file.sock"), "abc", "").await;
    assert!(!r.ready && r.reason.as_deref().unwrap().starts_with("The Docker socket is not mounted ("), "{r:?}");
    // a socket nobody answers on
    let dead = dir.join("dead.sock");
    drop(std::os::unix::net::UnixListener::bind(&dead).unwrap());
    let r = probe_with(&app, &dead, "abc", "").await;
    assert!(r.reason.as_deref().unwrap().starts_with("The Docker socket refused the request: "), "{r:?}");

    let (_d, m) = serve_mock(&dir).await;
    let sock = dir.join("docker.sock");
    let ins = fixture();
    let id = ins["Id"].as_str().unwrap();
    // the hostname is the id's first 12 characters
    let r = probe_with(&app, &sock, &id[..12], "").await;
    let want =
        SelfUpdate { ready: true, reason: None, id: Some(id.into()), name: Some("myapp".into()), image: Some(format!("{IMAGE}:latest")) };
    assert_eq!(r, want);
    assert_eq!(SelfUpdate::from_json(&r.json()), r, "the answer keeps in a cache");
    // a hostname of the owner's own: the id from mountinfo
    let mi = format!("1251 1236 254:1 /var/lib/docker/containers/{id}/hostname /etc/hostname rw - ext4 /dev/vda1 rw\n");
    assert!(probe_with(&app, &sock, "music", &mi).await.ready);
    // a Docker that doesn't know this machine
    assert_eq!(probe_with(&app, &sock, "laptop", "").await.reason.as_deref(), Some("Not running in a container"));
    // a container started with --rm can't be swapped
    m.fail.lock().unwrap().insert("rm");
    let r = probe_with(&app, &sock, &id[..12], "").await;
    assert_eq!(r.reason.as_deref(), Some("This container is removed when it stops (started with --rm), so it can't be replaced"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Update now up to the helper: the pull (Docker's words when it fails), then the helper's container from
/// the new image with the socket, started.
#[tokio::test]
async fn update_now_pulls_and_starts_the_helper() {
    let app = app();
    let dir = sock_dir("begin");
    let (d, m) = serve_mock(&dir).await;
    let id = fixture()["Id"].as_str().unwrap().to_string();
    let image = format!("{IMAGE}:0.28.0");
    let mut r = Report::start(&app, &image, "0.28.0", "0.27.0", "admin", 1.0);
    // an ended helper of an earlier try is removed first, so its log is not read as this update's
    m.fail.lock().unwrap().insert("helper-left");
    let name = start_update(&app, &d, &id, "0.28.0", Some(SCHEMA), &mut r).await.unwrap();
    m.fail.lock().unwrap().remove("helper-left");
    assert!(name.starts_with("myapp-update-"), "{name}");
    let calls = m.calls();
    assert_eq!(calls[0], "POST /images/create?fromImage=ghcr.io%2Fexample%2Fmyapp&tag=0.28.0");
    assert_eq!(calls[1], format!("GET /containers/{id}/json"));
    assert!(calls[2].starts_with("GET /containers/json?all=true&filters="), "{calls:?}");
    assert!(calls[2].contains("org.myapp.role%3Dself-update-helper"), "found by the app's label: {calls:?}");
    assert_eq!(calls[3], format!("GET /containers/{HELPER_ID}/json"));
    assert_eq!(calls[4], format!("DELETE /containers/{HELPER_ID}?force=true"));
    assert!(calls[5].starts_with("POST /containers/create?name=myapp-update-"), "{calls:?}");
    assert_eq!(calls[6], format!("POST /containers/{HELPER_ID}/start"));
    let body = m.body("/containers/create?name=myapp-update-").unwrap();
    assert_eq!(body["Cmd"], json!(["self-update-helper", "--target", id, "--image", image, "--schema", SCHEMA.to_string()]));
    assert_eq!(body["HostConfig"].get("AutoRemove"), None, "the helper's log outlives it");
    assert_eq!(
        r.lines.iter().map(|l| l.split(':').next().unwrap()).collect::<Vec<_>>(),
        ["pull", "pull", "helper", "helper"],
        "{:?}",
        r.lines
    );

    // the failure path on the server's side: the failing step and Docker's words
    let e = start_update(&app, &d, &id, "9.9.9", Some(SCHEMA), &mut r).await.unwrap_err();
    assert_eq!(e, ("pull", format!("Could not pull {IMAGE}:9.9.9: manifest unknown")));
    let e = start_update(&app, &d, &id, "denied", Some(SCHEMA), &mut r).await.unwrap_err().1;
    assert!(e.starts_with(&format!("Could not pull {IMAGE}:denied: pull access denied")), "{e}");
    assert!(e.contains("the myapp package on ghcr.io must be public (GitHub → Packages → myapp →"), "{e}");
    assert!(pull_error("i:1", "error from registry: unauthorized").contains("the image must be public"));
    assert_eq!(pull_error("i:1", "manifest unknown"), "Could not pull i:1: manifest unknown");
    assert_eq!(m.calls().iter().filter(|c| c.starts_with("POST /containers/create")).count(), 1, "no helper after a failed pull");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The server that comes back reads the ended helper's log into the record and removes the helper.
#[tokio::test]
async fn the_record_is_read_back_from_the_helper() {
    let app = app();
    let dir = sock_dir("readback");
    let (d, m) = serve_mock(&dir).await;
    m.fail.lock().unwrap().insert("helper-left");
    let mut r = Report::start(&app, &format!("{IMAGE}:0.28.0"), "0.28.0", "0.27.0", "admin", crate::now());
    read_back_and_check(&app, &d, &mut r).await;
    assert_eq!(r.state, report::State::Failed);
    assert_eq!(r.failed_step.as_deref(), Some("verify"));
    assert!(r.error.as_deref().unwrap().contains("the new MyApp refused the database"), "{r:?}");
    assert!(m.calls().contains(&format!("DELETE /containers/{HELPER_ID}?force=true")), "the helper goes: {:?}", m.calls());
    // an ended record is left as it is
    let calls = m.calls().len();
    read_back_and_check(&app, &d, &mut r).await;
    assert_eq!(m.calls().len(), calls);

    // no helper left and the new version answers: done
    m.fail.lock().unwrap().remove("helper-left");
    let mut r = Report::start(&app, &format!("{IMAGE}:0.28.0"), "0.28.0", "0.27.0", "admin", crate::now());
    report::read_back(&app, &d, &mut r, "0.28.0").await;
    assert_eq!(r.state, report::State::Ok);
    let _ = std::fs::remove_dir_all(&dir);
}

/// [`report::read_back`] on a server still at the old version.
async fn read_back_and_check(app: &App, d: &Docker, r: &mut Report) {
    report::read_back(app, d, r, "0.27.0").await;
}
