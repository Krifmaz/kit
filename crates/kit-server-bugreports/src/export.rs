//! The export: the matching reports as a zip (`reports.md`, `reports.json`, `files/…`), or with `format=txt` as
//! one plain text file with the files listed by name and text files (a crash log) written out in whole.

use crate::reports::{filter, load_full, Full, ListQ};
use crate::store::FileRow;
use crate::time::{when, ymd};
use crate::{fill, mb, reports_dir, BugReportsHost, Cfg, Words};
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use kit_server_core::db::now;
use kit_server_core::{err, Admin, Result};
use serde_json::{json, Value};
use std::io::Write;

/// The path of a file inside the zip.
fn zip_path(report_id: i64, f: &FileRow) -> String {
    format!("files/{report_id}/{}-{}", f.id, f.name)
}

/// A Markdown image or link to the file in the zip. The destination sits in angle brackets: a name like
/// "Screenshot 2026-10-18 (1).png" has spaces and a parenthesis, which end a bare destination.
fn md_file(report_id: i64, f: &FileRow) -> String {
    let p = zip_path(report_id, f);
    if f.mime.starts_with("image/") {
        format!("![{}](<{p}>)", f.name)
    } else {
        format!("[{}](<{p}>)", f.name)
    }
}

/// The export's heading and words.
struct App {
    /// the heading: [`Words::export_title`] with the app's name
    title: String,
    /// the app's version
    version: String,
    /// the kinds' and statuses' words
    words: Words,
}

/// reports.md: every report, its meta line, body, files and thread.
fn render_md(reports: &[Full], exported: &str, app: &App) -> String {
    let mut out = format!("# {} ({}, exported {exported})\n", app.title, app.version);
    for full in reports {
        let r = &full.report;
        let id = r.id;
        out.push_str(&format!("\n## #{id} [{}] {} — {}\n", app.words.kind_label(&r.kind), r.title, app.words.status_label(&r.status)));
        let mut meta = vec![format!("by {}", r.username), when(r.created_at)];
        for (label, v) in [("page", &r.page), ("app", &r.app_version), ("device", &r.device)] {
            if !v.trim().is_empty() {
                meta.push(format!("{label} {}", v.trim()));
            }
        }
        out.push_str(&format!("_{}_\n\n{}\n", meta.join(", "), r.body.trim()));
        for f in &full.files {
            out.push_str(&format!("\n{}\n", md_file(id, f)));
        }
        for (m, files) in &full.messages {
            let who = if m.is_admin { format!("{} (admin)", m.username) } else { m.username.clone() };
            out.push_str(&format!("\n**{who} — {}**\n\n{}\n", when(m.created_at), m.body.trim()));
            for f in files {
                out.push_str(&format!("\n{}\n", md_file(id, f)));
            }
        }
    }
    out
}

/// A text file (.txt, .log: a crash log) written into the `format=txt` export: at most this many bytes of each.
const TXT_INLINE_MAX: usize = 1 << 20;

/// The `format=txt` export: every report as plain text, its files and its thread's files listed by name, and the
/// text files (a crash log) written in whole under the name.
fn render_txt(reports: &[Full], exported: &str, base: &std::path::Path, app: &App) -> String {
    let rule = "=".repeat(72);
    let names = |files: &[FileRow]| {
        let mut out = files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ");
        for f in files.iter().filter(|f| f.mime.starts_with("text/") && !f.path.is_empty()) {
            let Ok(data) = std::fs::read(base.join(&f.path)) else { continue };
            let text = String::from_utf8_lossy(&data[..data.len().min(TXT_INLINE_MAX)]).into_owned();
            let more = if data.len() > TXT_INLINE_MAX { "\n… (cut)" } else { "" };
            out.push_str(&format!("\n\n----- {} -----\n{}{more}\n----- end of {} -----", f.name, text.trim_end(), f.name));
        }
        out
    };
    let mut out = format!("{} ({}, exported {exported})\n", app.title, app.version);
    for full in reports {
        let r = &full.report;
        out.push_str(&format!(
            "\n{rule}\n#{} [{}] {} - {}\n",
            r.id,
            app.words.kind_label(&r.kind),
            r.title,
            app.words.status_label(&r.status)
        ));
        out.push_str(&format!("By: {}\nCreated: {}\n", r.username, when(r.created_at)));
        for (label, v) in [("Page", &r.page), ("App", &r.app_version), ("Device", &r.device)] {
            if !v.trim().is_empty() {
                out.push_str(&format!("{label}: {}\n", v.trim()));
            }
        }
        out.push_str(&format!("{rule}\n\n{}\n", r.body.trim()));
        if !full.files.is_empty() {
            out.push_str(&format!("\nFiles: {}\n", names(&full.files)));
        }
        for (m, files) in &full.messages {
            let who = if m.is_admin { format!("{} (admin)", m.username) } else { m.username.clone() };
            out.push_str(&format!("\n--- {who}, {} ---\n{}\n", when(m.created_at), m.body.trim()));
            if !files.is_empty() {
                out.push_str(&format!("Files: {}\n", names(files)));
            }
        }
    }
    out
}

/// reports.json: the same shapes as the detail endpoint, with each file's `path` inside the zip.
fn render_json(reports: &[Full], exported: &str, app: &App) -> Value {
    let file = |rid: i64, f: &FileRow| json!({ "id": f.id, "name": f.name, "mime": f.mime, "size": f.size, "path": zip_path(rid, f) });
    let list: Vec<Value> = reports
        .iter()
        .map(|full| {
            let r = &full.report;
            let id = r.id;
            json!({
                "id": id, "kind": r.kind, "kind_label": app.words.kind_label(&r.kind), "title": r.title, "body": r.body, "page": r.page,
                "app_version": r.app_version, "device": r.device, "status": r.status, "status_label": app.words.status_label(&r.status),
                "username": r.username, "user_id": r.user_id, "created_at": r.created_at, "updated_at": r.updated_at,
                "files": full.files.iter().map(|f| file(id, f)).collect::<Vec<_>>(),
                "messages": full.messages.iter().map(|(m, files)| json!({
                    "id": m.id, "username": m.username, "is_admin": m.is_admin, "body": m.body, "created_at": m.created_at,
                    "files": files.iter().map(|f| file(id, f)).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({ "version": app.version, "exported": exported, "reports": list })
}

/// The zip: reports.md, reports.json and every file that is still on disk (a missing one is left out rather than
/// failing the whole export).
fn build_zip(reports: &[Full], exported: &str, base: &std::path::Path, app: &App) -> Result<Vec<u8>, String> {
    let fail = |e: &dyn std::fmt::Display| e.to_string();
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let text = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    z.start_file("reports.md", text).map_err(|e| fail(&e))?;
    z.write_all(render_md(reports, exported, app).as_bytes()).map_err(|e| fail(&e))?;
    z.start_file("reports.json", text).map_err(|e| fail(&e))?;
    let js = serde_json::to_string_pretty(&render_json(reports, exported, app)).map_err(|e| fail(&e))?;
    z.write_all(js.as_bytes()).map_err(|e| fail(&e))?;
    for full in reports {
        let id = full.report.id;
        let files = full.files.iter().chain(full.messages.iter().flat_map(|(_, f)| f.iter()));
        for f in files {
            if f.path.is_empty() {
                continue;
            }
            // a missing file is left out rather than failing the whole export
            let Ok(data) = std::fs::read(base.join(&f.path)) else { continue };
            z.start_file(zip_path(id, f), if f.mime.starts_with("image/") { stored } else { text }).map_err(|e| fail(&e))?;
            z.write_all(&data).map_err(|e| fail(&e))?;
        }
    }
    Ok(z.finish().map_err(|e| fail(&e))?.into_inner())
}

/// A download answer: the bytes, their type, `attachment; filename="…"`, never cached.
fn download(body: impl Into<Body>, mime: &'static str, name: &str) -> Response {
    let mut resp = body.into().into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")).unwrap_or(HeaderValue::from_static("attachment")),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    resp
}

/// GET /api/bug-reports/export: the matching reports as a zip (or one text file with `format=txt`; admins).
pub(crate) async fn export<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    Admin(u): Admin,
    Query(q): Query<ListQ>,
) -> Result<Response> {
    let filter = filter(&cfg, &q, &u, true);
    // the zip is built in memory: the files it would hold have to fit
    let held = st.reports().export_bytes(&filter).await?;
    if held > cfg.limits.max_export_bytes {
        return Err(err(
            400,
            format!(
                "That export would be over {} MB – narrow it with the filters or export reports one by one",
                mb(cfg.limits.max_export_bytes)
            ),
        ));
    }
    let mut reports: Vec<Full> = Vec::new();
    for r in st.reports().export_reports(&filter).await? {
        reports.push(load_full(&st, r).await?);
    }
    let exported = ymd(now());
    let name = st.app_name();
    let stem = cfg.words.export_stem(&name, &exported);
    let app = App { title: fill(&cfg.words.export_title, &[("app", &name)]), version: st.app_version(), words: cfg.words.clone() };
    let base = reports_dir(&st);
    if q.format.eq_ignore_ascii_case("txt") {
        let text = render_txt(&reports, &exported, &base, &app);
        return Ok(download(text, "text/plain; charset=utf-8", &format!("{stem}.txt")));
    }
    let bytes = tokio::task::spawn_blocking(move || build_zip(&reports, &exported, &base, &app))
        .await
        .map_err(|e| err(500, e.to_string()))?
        .map_err(|e| err(500, format!("Could not build the zip: {e}")))?;
    Ok(download(bytes, "application/zip", &format!("{stem}.zip")))
}
