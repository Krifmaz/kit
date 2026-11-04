//! The bug report routes through the router on [`TestHost`]: file, list, unread, read, reply, status, export
//! (zip and text), delete; who may see what; the refused uploads; the limits; and what the app is told.

use super::*;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use kit_server_core::testing::TestHost;
use kit_server_core::{p, Db};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;
use tower::ServiceExt;

/// What the tests' app was told: notifications and mails, by data folder.
static TOLD: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
/// The test rate limiter's counts.
static COUNTS: LazyLock<Mutex<HashMap<String, usize>>> = LazyLock::new(Default::default);

impl BugReportsHost for TestHost {
    fn data_dir(&self) -> PathBuf {
        self.data_dir.clone()
    }
    fn app_name(&self) -> String {
        "Demo".into()
    }
    fn app_version(&self) -> String {
        "1.4.0".into()
    }
    fn limited(&self, key: &str, limit: usize, _window_secs: f64) -> bool {
        let mut m = COUNTS.lock().unwrap();
        let n = m.entry(format!("{}|{key}", self.data_dir.display())).or_default();
        if *n >= limit {
            return true;
        }
        *n += 1;
        false
    }
    fn notify(&self, title: String, message: String) -> impl Future<Output = ()> + Send + 'static {
        TOLD.lock().unwrap().push((title, message));
        async {}
    }
    fn mail_enabled(&self) -> bool {
        true
    }
    fn mail_address(&self, user_id: i64) -> Result<String> {
        Ok(if user_id == 2 { "ana@example.com".into() } else { String::new() })
    }
    fn send_mail(&self, to: String, subject: String, text: String) -> impl Future<Output = Result<(), String>> + Send + 'static {
        TOLD.lock().unwrap().push((format!("mail {to}: {subject}"), text));
        async { Ok(()) }
    }
}

const BOUNDARY: &str = "----kit-bugreport-test";
const ADMIN: i64 = 1;
const USER: i64 = 2;
const OTHER: i64 = 3;

fn app() -> (Router, TestHost) {
    let st = TestHost::new();
    st.db.execute_batch("INSERT INTO users (id, username, role) VALUES (3, 'other', 'member')").unwrap();
    (router().with_state(st.clone()), st)
}

async fn send(app: &Router, method: &str, uri: &str, user: i64, content_type: &str, body: Vec<u8>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut b = Request::builder().method(method).uri(uri).header("x-test-user", user.to_string());
    if !content_type.is_empty() {
        b = b.header("content-type", content_type);
    }
    let resp = app.clone().oneshot(b.body(Body::from(body)).unwrap()).await.unwrap();
    let (status, headers) = (resp.status(), resp.headers().clone());
    let bytes = axum::body::to_bytes(resp.into_body(), 64 << 20).await.unwrap();
    (status, headers, bytes.to_vec())
}

async fn call(app: &Router, method: &str, uri: &str, user: i64, body: Option<Value>) -> (StatusCode, Value) {
    let (ct, bytes) = match body {
        Some(j) => ("application/json", j.to_string().into_bytes()),
        None => ("", vec![]),
    };
    let (status, _, bytes) = send(app, method, uri, user, ct, bytes).await;
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

type Upload<'a> = (&'a str, &'a str, &'a [u8]);

async fn post_form(app: &Router, uri: &str, user: i64, fields: &[(&str, &str)], files: &[Upload<'_>]) -> (StatusCode, Value) {
    let mut body = Vec::new();
    for (k, v) in fields {
        body.extend_from_slice(format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n").as_bytes());
    }
    for (name, mime, data) in files {
        body.extend_from_slice(
            format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: {mime}\r\n\r\n")
                .as_bytes(),
        );
        body.extend_from_slice(data);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    let (status, _, bytes) = send(app, "POST", uri, user, &format!("multipart/form-data; boundary={BOUNDARY}"), body).await;
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// CRC-32 (PNG chunks).
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut c = 0xffff_ffffu32;
    for b in parts.iter().flat_map(|p| p.iter()) {
        c ^= *b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// A `w`×`h` black RGBA8 PNG built by hand (one fixed-Huffman deflate block of "copy the last byte" runs), so the
/// test neither holds nor compresses the raw pixels.
fn huge_png(w: u32, h: u32) -> Vec<u8> {
    let n: u64 = h as u64 * (1 + w as u64 * 4);
    let mut z = vec![0x78u8, 0x01];
    let (mut acc, mut bits) = (0u64, 0u32);
    let mut put = |z: &mut Vec<u8>, v: u64, k: u32| {
        acc |= v << bits;
        bits += k;
        while bits >= 8 {
            z.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    };
    let code = |c: u32, k: u32| (c.reverse_bits() >> (32 - k)) as u64;
    put(&mut z, 1, 1);
    put(&mut z, 1, 2);
    put(&mut z, code(0x30, 8), 8);
    let mut left = n - 1;
    while left >= 258 {
        put(&mut z, code(0b1100_0101, 8), 8);
        put(&mut z, 0, 5);
        left -= 258;
    }
    for _ in 0..left {
        put(&mut z, code(0x30, 8), 8);
    }
    put(&mut z, 0, 7);
    if bits > 0 {
        z.push(acc as u8);
    }
    z.extend_from_slice(&((((n % 65521) << 16) | 1) as u32).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

async fn unread_count(app: &Router, user: i64) -> i64 {
    call(app, "GET", "/api/bug-reports/unread", user, None).await.1["count"].as_i64().unwrap()
}

#[tokio::test]
async fn report_reply_status_export_and_delete() {
    let (app, st) = app();
    let shot = png(6, 4);
    let (status, body) = post_form(
        &app,
        "/api/bug-reports",
        USER,
        &[
            ("kind", "bug"),
            ("title", "Save button does nothing"),
            ("body", "Nothing happened."),
            ("page", "#/projects/12"),
            ("app_version", "1.3.2"),
        ],
        &[("../shot one.png", "image/png", &shot)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let id = body["id"].as_i64().unwrap();
    let folder = st.data_dir.join("reports").join(id.to_string());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
    assert!(TOLD.lock().unwrap().contains(&("New bug report".into(), "ana: Save button does nothing".into())));

    // the admin has it unread, the reporter doesn't; someone else sees nothing
    let r = call(&app, "GET", "/api/bug-reports", ADMIN, None).await;
    assert_eq!(r.1["reports"][0]["file_count"], json!(1));
    assert_eq!(r.1["reports"][0]["unread"], json!(true));
    assert_eq!(unread_count(&app, ADMIN).await, 1);
    assert_eq!(unread_count(&app, USER).await, 0);
    assert_eq!(call(&app, "GET", "/api/bug-reports", OTHER, None).await.1["reports"], json!([]));
    assert_eq!(call(&app, "GET", "/api/bug-reports?mine=1", ADMIN, None).await.1["reports"], json!([]));
    assert_eq!(call(&app, "GET", "/api/bug-reports?kind=bug&q=save", ADMIN, None).await.1["reports"].as_array().unwrap().len(), 1);
    assert_eq!(call(&app, "GET", "/api/bug-reports?q=50%25", ADMIN, None).await.1["reports"], json!([]));

    // read: the file with its url; seen
    let r = call(&app, "GET", &format!("/api/bug-reports/{id}"), ADMIN, None).await;
    let f = &r.1["report"]["files"][0];
    assert_eq!(f["name"], json!("shot one.png"));
    let fid = f["id"].as_i64().unwrap();
    assert_eq!(f["url"], json!(format!("/api/bug-reports/{id}/files/{fid}")));
    assert_eq!(unread_count(&app, ADMIN).await, 0);
    let (status, headers, bytes) = send(&app, "GET", &format!("/api/bug-reports/{id}/files/{fid}"), USER, "", vec![]).await;
    assert_eq!((status, bytes), (StatusCode::OK, shot.clone()));
    assert_eq!(headers["content-type"], "image/png");
    assert_eq!(headers["content-disposition"], "inline; filename=\"shot one.png\"; filename*=UTF-8''shot%20one.png");
    assert_eq!(headers["cache-control"], "private, max-age=3600");
    assert_eq!(send(&app, "GET", &format!("/api/bug-reports/{id}/files/{fid}"), OTHER, "", vec![]).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", &format!("/api/bug-reports/{id}"), OTHER, None).await.1["detail"], json!("Report not found"));

    // the admin answers: answered, unread for the reporter, a mail to the reporter
    let (status, body) = post_form(&app, &format!("/api/bug-reports/{id}/messages"), ADMIN, &[("body", " ")], &[]).await;
    assert_eq!((status, body["detail"].clone()), (StatusCode::BAD_REQUEST, json!("Write a reply or attach a file")));
    assert_eq!(post_form(&app, &format!("/api/bug-reports/{id}/messages"), ADMIN, &[("body", "Looking.")], &[]).await.0, StatusCode::OK);
    let r = call(&app, "GET", "/api/bug-reports", USER, None).await;
    assert_eq!((r.1["reports"][0]["status"].clone(), r.1["reports"][0]["unread"].clone()), (json!("answered"), json!(true)));
    assert_eq!(unread_count(&app, USER).await, 1);
    tokio::task::yield_now().await;
    assert!(TOLD.lock().unwrap().contains(&(
        "mail ana@example.com: Demo: a reply to your report “Save button does nothing”".into(),
        format!("admin answered your report “Save button does nothing”:\n\nLooking.\n\n#/bug-reports/{id}\n")
    )));
    // the reporter answers with a log only: new again, unread for the admin
    let (status, _) =
        post_form(&app, &format!("/api/bug-reports/{id}/messages"), USER, &[], &[("console.log", "text/plain", b"TypeError\n")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unread_count(&app, ADMIN).await, 1);
    assert!(TOLD.lock().unwrap().contains(&("Bug report reply".into(), format!("ana on #{id} Save button does nothing: (a file)"))));
    assert_eq!(call(&app, "GET", &format!("/api/bug-reports/{id}"), ADMIN, None).await.1["report"]["status"], json!("new"));

    // status: admins only, one of the five
    assert_eq!(
        call(&app, "POST", &format!("/api/bug-reports/{id}/status"), USER, Some(json!({ "status": "fixed" }))).await.0,
        StatusCode::FORBIDDEN
    );
    let r = call(&app, "POST", &format!("/api/bug-reports/{id}/status"), ADMIN, Some(json!({ "status": "done" }))).await;
    assert_eq!(r.1["detail"], json!("Pick a status: New, Looking into it, Fixed, Won't fix or Answered"));
    assert_eq!(
        call(&app, "POST", &format!("/api/bug-reports/{id}/status"), ADMIN, Some(json!({ "status": "looking" }))).await.1,
        json!({ "ok": true })
    );

    // export: the zip
    assert_eq!(send(&app, "GET", "/api/bug-reports/export", USER, "", vec![]).await.0, StatusCode::FORBIDDEN);
    let (status, headers, bytes) = send(&app, "GET", &format!("/api/bug-reports/export?ids={id}"), ADMIN, "", vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "application/zip");
    let cd = headers["content-disposition"].to_str().unwrap();
    assert!(cd.starts_with("attachment; filename=\"demo-bug-reports-") && cd.ends_with(".zip\""), "{cd}");
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut md = String::new();
    zip.by_name("reports.md").unwrap().read_to_string(&mut md).unwrap();
    assert!(md.starts_with("# Demo bug reports (1.4.0, exported "), "{md}");
    assert!(md.contains(&format!("## #{id} [Bug] Save button does nothing — Looking into it")), "{md}");
    assert!(md.contains(&format!("![shot one.png](<files/{id}/{fid}-shot one.png>)")), "{md}");
    assert!(md.contains("**admin (admin) — ") && md.contains("**ana — "), "{md}");
    let mut js = String::new();
    zip.by_name("reports.json").unwrap().read_to_string(&mut js).unwrap();
    let js: Value = serde_json::from_str(&js).unwrap();
    assert_eq!((js["version"].clone(), js["reports"][0]["status_label"].clone()), (json!("1.4.0"), json!("Looking into it")));
    let mut img = Vec::new();
    zip.by_name(&format!("files/{id}/{fid}-shot one.png")).unwrap().read_to_end(&mut img).unwrap();
    assert_eq!(img, shot);
    // export: one text file, the log written out
    let (status, headers, bytes) = send(&app, "GET", &format!("/api/bug-reports/export?ids={id}&format=txt"), ADMIN, "", vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["content-type"], "text/plain; charset=utf-8");
    let txt = String::from_utf8(bytes).unwrap();
    assert!(txt.starts_with("Demo bug reports (1.4.0, exported "), "{txt}");
    assert!(txt.contains(&format!("#{id} [Bug] Save button does nothing - Looking into it\nBy: ana\nCreated: ")), "{txt}");
    assert!(txt.contains("Page: #/projects/12\nApp: 1.3.2\n") && txt.contains("\nFiles: shot one.png\n"), "{txt}");
    assert!(txt.contains("Files: console.log\n\n----- console.log -----\nTypeError\n----- end of console.log -----"), "{txt}");

    // delete: admins only; the rows and the folder go
    assert_eq!(call(&app, "DELETE", &format!("/api/bug-reports/{id}"), USER, None).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&app, "DELETE", &format!("/api/bug-reports/{id}"), ADMIN, None).await.1, json!({ "ok": true }));
    assert!(!folder.exists());
    assert_eq!(call(&app, "GET", &format!("/api/bug-reports/{id}"), ADMIN, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "GET", "/api/bug-reports", 0, None).await.0, StatusCode::UNAUTHORIZED);

    // an admin's own report is not news to them
    assert_eq!(
        post_form(&app, "/api/bug-reports", ADMIN, &[("kind", "crash"), ("title", "Note"), ("body", "x")], &[]).await.0,
        StatusCode::OK
    );
    assert_eq!(unread_count(&app, ADMIN).await, 0);
}

#[tokio::test]
async fn bad_uploads_are_refused_with_a_sentence() {
    let (app, st) = app();
    let fields = [("title", "How do I?"), ("body", "Can't find it.")];
    let refused = |r: (StatusCode, Value)| {
        assert_eq!(r.0, StatusCode::BAD_REQUEST, "{:?}", r.1);
        r.1["detail"].as_str().unwrap().to_string()
    };
    let only = "Only PNG, JPEG, WebP or GIF images and .txt or .log text files can be attached";
    assert_eq!(refused(post_form(&app, "/api/bug-reports", USER, &fields, &[("setup.exe", "application/x", b"MZ\x90\x00")]).await), only);
    let mut broken = b"\x89PNG\r\n\x1a\n".to_vec();
    broken.extend_from_slice(&[7u8; 64]);
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &fields, &[("a.png", "image/png", &broken)]).await),
        "That image can't be read"
    );
    let huge = huge_png(11000, 11000);
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &fields, &[("a.png", "image/png", &huge)]).await),
        "That image can't be read"
    );
    let big = vec![b'a'; 9 << 20];
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &fields, &[("big.txt", "text/plain", &big)]).await),
        "Each file can be at most 8 MB"
    );
    let shot = png(2, 2);
    let seven: Vec<Upload> = (0..7).map(|_| ("a.png", "image/png", shot.as_slice())).collect();
    assert_eq!(refused(post_form(&app, "/api/bug-reports", USER, &fields, &seven).await), "At most 6 files per report");
    assert_eq!(refused(post_form(&app, "/api/bug-reports", USER, &[("title", " "), ("body", "x")], &[]).await), "Give the report a title");
    assert_eq!(refused(post_form(&app, "/api/bug-reports", USER, &[("title", "T"), ("body", "")], &[]).await), "Describe what happened");
    let long = "t".repeat(201);
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &[("title", &long), ("body", "x")], &[]).await),
        "The title can have at most 200 characters"
    );
    let body = "b".repeat(20_001);
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &[("title", "T"), ("body", &body)], &[]).await),
        "The description can have at most 20000 characters"
    );
    assert_eq!(
        refused(post_form(&app, "/api/bug-reports", USER, &[("kind", "feature"), ("title", "T"), ("body", "x")], &[]).await),
        "Pick a kind: Bug, Not a bug, How do I… or Crash log"
    );
    assert!(!st.data_dir.join("reports").exists());
    // a PNG called .html is kept as .png; the title stays on one line
    let (status, body) = post_form(
        &app,
        "/api/bug-reports",
        USER,
        &[("title", "Notes\r\n## #9 fake"), ("body", "x")],
        &[("readme.html", "text/html", &shot)],
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let r = call(&app, "GET", &format!("/api/bug-reports/{}", body["id"]), USER, None).await;
    assert_eq!((r.1["report"]["title"].clone(), r.1["report"]["kind"].clone()), (json!("Notes ## #9 fake"), json!("bug")));
    assert_eq!(r.1["report"]["files"][0]["name"], json!("readme.png"));
}

#[tokio::test]
async fn too_many_or_too_much_is_refused() {
    let (app, st) = app();
    let fields = [("title", "Again"), ("body", "the same")];
    for _ in 0..Limits::default().reports_per_hour {
        assert_eq!(post_form(&app, "/api/bug-reports", USER, &fields, &[]).await.0, StatusCode::OK);
    }
    let r = post_form(&app, "/api/bug-reports", USER, &fields, &[]).await;
    assert_eq!((r.0, r.1["detail"].clone()), (StatusCode::TOO_MANY_REQUESTS, json!("Too many reports in a short time – try again later")));
    for _ in 0..=Limits::default().reports_per_hour {
        assert_eq!(post_form(&app, "/api/bug-reports", ADMIN, &fields, &[]).await.0, StatusCode::OK);
    }
    // 500 MB a user, admins exempt
    let rid = post_form(&app, "/api/bug-reports", OTHER, &fields, &[]).await.1["id"].as_i64().unwrap();
    st.db
        .execute(
            "INSERT INTO bug_report_files (report_id, message_id, name, mime, size, path, created_at) VALUES (?,NULL,'old.log','text/plain',?,'',0)",
            p![rid, 600i64 << 20],
        )
        .unwrap();
    let r = post_form(&app, &format!("/api/bug-reports/{rid}/messages"), OTHER, &[], &[("more.log", "text/plain", b"line\n")]).await;
    assert_eq!(r.1["detail"], json!("Your reports already hold 600 MB of files (the most is 500 MB); the admin can delete old ones"));
    assert_eq!(post_form(&app, &format!("/api/bug-reports/{rid}/messages"), OTHER, &[("body", "words")], &[]).await.0, StatusCode::OK);
    assert_eq!(
        post_form(&app, &format!("/api/bug-reports/{rid}/messages"), ADMIN, &[], &[("more.log", "text/plain", b"line\n")]).await.0,
        StatusCode::OK
    );
    // the export holds at most 512 MB
    let (status, _, bytes) = send(&app, "GET", "/api/bug-reports/export", ADMIN, "", vec![]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["detail"], json!("That export would be over 512 MB – narrow it with the filters or export reports one by one"));
    assert_eq!(send(&app, "GET", "/api/bug-reports/export?ids=1", ADMIN, "", vec![]).await.0, StatusCode::OK);
}

#[test]
fn names_labels_and_checks() {
    assert_eq!(safe_name("../../etc/passwd.png", "png"), "passwd.png");
    assert_eq!(safe_name("C:\\Users\\me\\shot:1?.png", "png"), "shot_1_.png");
    assert_eq!(safe_name("...", "jpg"), "file.jpg");
    assert_eq!(safe_name(".hidden", "jpg"), "hidden.jpg");
    assert_eq!(safe_name(&"é".repeat(200), "png").chars().count(), MAX_NAME);
    assert_eq!(line("  two\t\tspaces \r\n "), "two spaces");
    let w = Words::default();
    assert_eq!(w.kind_label("crash"), "Crash log");
    assert_eq!(w.kind_label("feature"), "Bug");
    assert_eq!(w.status_label("anything"), "New");
    assert_eq!(w.export_stem("Demo", "2026-11-04"), "demo-bug-reports-2026-11-04");
    assert_eq!(w.export_stem("My App", "2026-11-04"), "my-app-bug-reports-2026-11-04");
    assert_eq!(fill("{a} {b} {c} {", &[("a", "{b}"), ("b", "2")]), "{b} 2 {c} {");
    assert_eq!(or_list(["A"].into_iter()), "A");
    assert_eq!(or_list(["A", "B", "C"].into_iter()), "A, B or C");
    assert_eq!(Limits::default().upload(), 50 << 20);
    let lim = Limits::default();
    assert_eq!(check_file("notes.log", b"line", &lim).unwrap(), ("text/plain", "log"));
    assert_eq!(check_file("notes.txt", &[0xff, 0xfe], &lim).unwrap_err().detail, "A text file has to be UTF-8");
    assert!(check_file("notes.md", b"text", &lim).is_err());
    assert_eq!(check_file("ok.png", &huge_png(64, 48), &lim).unwrap(), ("image/png", "png"));
    assert!(check_file("wide.png", &huge_png(8193, 2), &lim).is_err());
    assert!(check_file("fine.png", &huge_png(8192, 2), &lim).is_ok());
    assert!(check_file("fine.png", &huge_png(64, 48), &Limits { max_side: 32, ..Limits::default() }).is_err());
}

#[tokio::test]
async fn paths_limits_and_words_are_the_apps() {
    let st = TestHost::new();
    let words = Words {
        kinds: vec![("idea".into(), "Idea".into()), ("problem".into(), "Problem".into())],
        export_title: "{app} feedback".into(),
        export_file: "{app}-feedback".into(),
        notify_new: "New feedback".into(),
        mail_subject: "About “{title}”".into(),
        mail_text: "{who}: {body} ({link})".into(),
        report_link: "/feedback/{id}".into(),
        ..Words::default()
    };
    let limits = Limits { max_title: 10, max_files: 1, ..Limits::default() };
    let config = BugReportsConfig { paths: BugReportPaths { base: "/feedback/".into() }, limits, words };
    let app = router_with(config).with_state(st.clone());
    let fields = [("title", "Dark mode"), ("body", "Please.")];
    let (status, body) = post_form(&app, "/feedback", USER, &fields, &[]).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
    let id = body["id"].as_i64().unwrap();
    assert!(TOLD.lock().unwrap().contains(&("New feedback".into(), "ana: Dark mode".into())));
    let r = call(&app, "GET", &format!("/feedback/{id}"), USER, None).await;
    assert_eq!(r.1["report"]["kind"], json!("idea"));
    let refused = post_form(&app, "/feedback", USER, &[("kind", "bug"), ("title", "T"), ("body", "x")], &[]).await;
    assert_eq!(refused.1["detail"], json!("Pick a kind: Idea or Problem"));
    let refused = post_form(&app, "/feedback", USER, &[("title", "Eleven char"), ("body", "x")], &[]).await;
    assert_eq!(refused.1["detail"], json!("The title can have at most 10 characters"));
    let shot = png(2, 2);
    let two: Vec<Upload> = (0..2).map(|_| ("a.png", "image/png", shot.as_slice())).collect();
    assert_eq!(post_form(&app, "/feedback", USER, &fields, &two).await.1["detail"], json!("At most 1 files per report"));
    let (_, f) = post_form(&app, &format!("/feedback/{id}/messages"), USER, &[], &[("a.png", "image/png", &shot)]).await;
    assert_eq!(f["ok"], json!(true));
    let r = call(&app, "GET", &format!("/feedback/{id}"), USER, None).await;
    let url = r.1["messages"][0]["files"][0]["url"].as_str().unwrap().to_string();
    assert!(url.starts_with(&format!("/feedback/{id}/files/")), "{url}");
    assert_eq!(send(&app, "GET", &url, USER, "", vec![]).await.0, StatusCode::OK);
    // the admin's reply mail
    post_form(&app, &format!("/feedback/{id}/messages"), ADMIN, &[("body", "Soon.")], &[]).await;
    tokio::task::yield_now().await;
    assert!(TOLD.lock().unwrap().contains(&("mail ana@example.com: About “Dark mode”".into(), format!("admin: Soon. (/feedback/{id})"))));
    // the export's title and name
    let (status, headers, bytes) = send(&app, "GET", &format!("/feedback/export?ids={id}&format=txt"), ADMIN, "", vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers["content-disposition"].to_str().unwrap().starts_with("attachment; filename=\"demo-feedback-"));
    let txt = String::from_utf8(bytes).unwrap();
    assert!(txt.starts_with("Demo feedback (1.4.0, exported "), "{txt}");
    assert!(txt.contains(&format!("#{id} [Idea] Dark mode - Answered")), "{txt}");
    assert_eq!(call(&app, "GET", "/api/bug-reports", USER, None).await.0, StatusCode::NOT_FOUND);
}
