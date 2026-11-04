//! The report handlers: file, list, count unread, read, reply, set the status, delete, and fetch a file.

use crate::files::{check_quota, read_form, store_files};
use crate::store::{FileRow, ListFilter, Message, NewMessage, NewReport, Report, Side};
use crate::{cut, fill, line, or_list, reports_dir, BugReportsConfig, BugReportsHost, Cfg, STATUSES};
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Extension;
use axum::Json;
use kit_server_core::db::now;
use kit_server_core::{err, Account, Admin, Result, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};

/// The report, or 404 – also when it is someone else's and the viewer is no admin.
pub(crate) async fn load(st: &impl BugReportsHost, id: i64, u: &Account) -> Result<Report> {
    let r = st.reports().report(id).await?.ok_or_else(|| err(404, "Report not found"))?;
    if !u.admin && r.user_id != u.id {
        return Err(err(404, "Report not found"));
    }
    Ok(r)
}

/// A report with its files and its thread.
pub(crate) struct Full {
    /// the report
    pub(crate) report: Report,
    /// the files attached to the report itself
    pub(crate) files: Vec<FileRow>,
    /// the replies, each with its files
    pub(crate) messages: Vec<(Message, Vec<FileRow>)>,
}

/// A report with its files and its replies.
pub(crate) async fn load_full(st: &impl BugReportsHost, report: Report) -> Result<Full> {
    let all = st.reports().files(report.id).await?;
    let files = all.iter().filter(|f| f.message_id.is_none()).cloned().collect();
    let messages = st
        .reports()
        .messages(report.id)
        .await?
        .into_iter()
        .map(|m| {
            let mine = all.iter().filter(|f| f.message_id == Some(m.id)).cloned().collect();
            (m, mine)
        })
        .collect();
    Ok(Full { report, files, messages })
}

/// A file as the API gives it, with its download URL under the routes' `base`.
fn file_json(base: &str, report_id: i64, f: &FileRow) -> Value {
    json!({ "id": f.id, "name": f.name, "mime": f.mime, "size": f.size, "url": format!("{base}/{report_id}/files/{}", f.id) })
}

/// A reply as the API gives it.
fn message_json(base: &str, report_id: i64, m: &Message, files: &[FileRow]) -> Value {
    json!({ "id": m.id, "username": m.username, "is_admin": m.is_admin, "body": m.body, "created_at": m.created_at,
            "files": files.iter().map(|f| file_json(base, report_id, f)).collect::<Vec<_>>() })
}

/// A report as the API gives it, with its files and its thread.
fn report_json(base: &str, full: &Full) -> Value {
    let r = &full.report;
    let id = r.id;
    json!({
        "report": {
            "id": id, "kind": r.kind, "title": r.title, "body": r.body, "page": r.page, "app_version": r.app_version,
            "device": r.device, "status": r.status, "username": r.username, "user_id": r.user_id,
            "created_at": r.created_at, "updated_at": r.updated_at,
            "files": full.files.iter().map(|f| file_json(base, id, f)).collect::<Vec<_>>(),
        },
        "messages": full.messages.iter().map(|(m, files)| message_json(base, id, m, files)).collect::<Vec<_>>(),
    })
}

/// `GET /api/bug-reports` (and export) query.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct ListQ {
    /// only this status
    pub(crate) status: String,
    /// only this kind
    pub(crate) kind: String,
    /// a search text
    pub(crate) q: String,
    /// "1" for the caller's own only (admins)
    pub(crate) mine: String,
    /// export only: `1,2,3`
    pub(crate) ids: String,
    /// export only: `txt` for one plain text file instead of the zip
    pub(crate) format: String,
}

/// The reports a list takes: the filters that are set (a status or kind the app doesn't know is no filter), and the
/// viewer's own reports unless an admin asks for all.
pub(crate) fn filter(cfg: &BugReportsConfig, q: &ListQ, u: &Account, admin: bool) -> ListFilter {
    let search = q.q.trim();
    ListFilter {
        status: STATUSES.contains(&q.status.as_str()).then(|| q.status.clone()),
        kind: cfg.words.has_kind(&q.kind).then(|| q.kind.clone()),
        search: (!search.is_empty()).then(|| search.to_string()),
        user_id: (!admin || matches!(q.mine.as_str(), "1" | "true")).then_some(u.id),
        ids: q.ids.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
    }
}

/// The side the viewer sees a report from: the admins' for an admin, the reporter's for anyone else.
fn side(u: &Account) -> Side {
    if u.admin {
        Side::Admin
    } else {
        Side::Reporter
    }
}

/// POST /api/bug-reports: file a report with up to [`crate::Limits::max_files`] files (a few an hour for
/// non-admins).
pub(crate) async fn create<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    SignedIn(u): SignedIn,
    form: Multipart,
) -> Result<Json<Value>> {
    let (admin, lim, words) = (u.admin, &cfg.limits, &cfg.words);
    if !admin && st.limited(&format!("bugreport:{}", u.id), lim.reports_per_hour, 3600.0) {
        return Err(err(429, "Too many reports in a short time – try again later"));
    }
    let form = read_form(&st, lim, form).await?;
    let title = line(form.text("title"));
    let body = form.text("body").trim().to_string();
    if title.is_empty() {
        return Err(err(400, "Give the report a title"));
    }
    if title.chars().count() > lim.max_title {
        return Err(err(400, format!("The title can have at most {} characters", lim.max_title)));
    }
    if body.is_empty() {
        return Err(err(400, "Describe what happened"));
    }
    if body.chars().count() > lim.max_body {
        return Err(err(400, format!("The description can have at most {} characters", lim.max_body)));
    }
    let kind = form.text("kind").trim();
    let kind = if kind.is_empty() {
        words.default_kind()
    } else if words.has_kind(kind) {
        kind
    } else {
        return Err(err(400, format!("Pick a kind: {}", or_list(words.kinds.iter().map(|(_, l)| l.as_str())))));
    };
    check_quota(&st, lim, &u, &form.files).await?;
    let t = now();
    let report = NewReport {
        user_id: u.id,
        username: u.username.clone(),
        kind: kind.to_string(),
        title: title.clone(),
        body,
        page: cut(&line(form.text("page")), 300),
        app_version: cut(&line(form.text("app_version")), 60),
        device: cut(&line(form.text("device")), 300),
        created_at: t,
        // an admin's own report is not news to them
        admin_seen_at: if admin { t } else { 0.0 },
    };
    let id = st.reports().create_report(&report).await?;
    store_files(&st, id, None, form.files).await?;
    st.log("info", &format!("{} filed bug report #{id}: {title}", u.username));
    tokio::spawn(st.notify(words.notify_new.clone(), format!("{}: {title}", u.username)));
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// GET /api/bug-reports: the reports the caller may see.
pub(crate) async fn list<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    SignedIn(u): SignedIn,
    Query(q): Query<ListQ>,
) -> Result<Json<Value>> {
    let rows = st.reports().list_reports(&filter(&cfg, &q, &u, u.admin), side(&u)).await?;
    let reports: Vec<Value> = rows
        .iter()
        .map(|s| {
            let r = &s.report;
            json!({
                "id": r.id, "kind": r.kind, "title": r.title, "status": r.status, "username": r.username, "user_id": r.user_id,
                "created_at": r.created_at, "updated_at": r.updated_at, "message_count": s.message_count, "file_count": s.file_count,
                "unread": s.unread,
            })
        })
        .collect();
    Ok(Json(json!({ "reports": reports })))
}

/// GET /api/bug-reports/unread: how many reports have something the caller hasn't seen.
pub(crate) async fn unread<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Result<Json<Value>> {
    let own = ListFilter { user_id: (!u.admin).then_some(u.id), ..ListFilter::default() };
    Ok(Json(json!({ "count": st.reports().unread_count(&own, side(&u)).await? })))
}

/// GET /api/bug-reports/{id}: a report with its thread; marks it seen.
pub(crate) async fn detail<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    SignedIn(u): SignedIn,
    Path(id): Path<i64>,
) -> Result<Json<Value>> {
    let r = load(&st, id, &u).await?;
    let t = now();
    if u.admin {
        st.reports().mark_seen(id, Side::Admin, t).await?;
    }
    if r.user_id == u.id {
        st.reports().mark_seen(id, Side::Reporter, t).await?;
    }
    Ok(Json(report_json(cfg.paths.base.trim_end_matches('/'), &load_full(&st, r).await?)))
}

/// POST /api/bug-reports/{id}/messages: reply to a report.
pub(crate) async fn reply<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    SignedIn(u): SignedIn,
    Path(id): Path<i64>,
    form: Multipart,
) -> Result<Json<Value>> {
    let r = load(&st, id, &u).await?;
    let (admin, lim, words) = (u.admin, &cfg.limits, &cfg.words);
    if !admin && st.limited(&format!("bugreply:{}", u.id), lim.replies_per_hour, 3600.0) {
        return Err(err(429, "Too many replies in a short time – try again later"));
    }
    let form = read_form(&st, lim, form).await?;
    let body = form.text("body").trim().to_string();
    if body.is_empty() && form.files.is_empty() {
        return Err(err(400, "Write a reply or attach a file"));
    }
    if body.chars().count() > lim.max_body {
        return Err(err(400, format!("A reply can have at most {} characters", lim.max_body)));
    }
    check_quota(&st, lim, &u, &form.files).await?;
    let t = now();
    let message =
        NewMessage { report_id: id, user_id: u.id, username: u.username.clone(), is_admin: admin, body: body.clone(), created_at: t };
    let mid = st.reports().add_message(&message).await?;
    store_files(&st, id, Some(mid), form.files).await?;
    let (status, title) = (r.status, r.title);
    if admin {
        let status = if status == "new" { "answered".to_string() } else { status };
        st.reports().record_reply(id, Side::Admin, &status, t).await?;
        // the reporter hears about it by mail, when mail is set up and they have an address (not for a reply
        // to one's own report)
        if st.mail_enabled() && r.user_id != u.id {
            let to = st.mail_address(r.user_id)?;
            if !to.trim().is_empty() {
                let base = st.base_url();
                let link = fill(&words.report_link, &[("id", &id.to_string())]);
                let link = if base.trim().is_empty() {
                    link
                } else {
                    format!("{}/{}", base.trim().trim_end_matches('/'), link.trim_start_matches('/'))
                };
                let subject = fill(&words.mail_subject, &[("app", &st.app_name()), ("title", &title)]);
                let text = fill(&words.mail_text, &[("who", &u.username), ("title", &title), ("body", &body), ("link", &link)]);
                let (sending, st2) = (st.send_mail(to, subject, text), st.clone());
                tokio::spawn(async move {
                    if let Err(e) = sending.await {
                        st2.log("warning", &format!("Bug report reply mail failed: {e}"));
                    }
                });
            }
        }
    } else {
        let status = if status == "answered" { "new".to_string() } else { status };
        st.reports().record_reply(id, Side::Reporter, &status, t).await?;
        let who = &u.username;
        let text = if body.is_empty() {
            format!("{who} on #{id} {title}: (a file)")
        } else {
            format!("{who} on #{id} {title}: {}", cut(&body, 300))
        };
        tokio::spawn(st.notify(words.notify_reply.clone(), text));
    }
    Ok(Json(json!({ "ok": true, "id": mid })))
}

/// `POST /api/bug-reports/{id}/status` body.
#[derive(Deserialize, Default)]
#[serde(default)]
pub(crate) struct StatusBody {
    /// one of [`STATUSES`]
    pub(crate) status: String,
}

/// POST /api/bug-reports/{id}/status: set a report's status (admins).
pub(crate) async fn set_status<S: BugReportsHost>(
    State(st): State<S>,
    Extension(cfg): Cfg,
    Admin(u): Admin,
    Path(id): Path<i64>,
    Json(b): Json<StatusBody>,
) -> Result<Json<Value>> {
    let status = b.status.trim();
    if !STATUSES.contains(&status) {
        return Err(err(400, format!("Pick a status: {}", or_list(STATUSES.iter().map(|s| cfg.words.status_label(s))))));
    }
    let r = load(&st, id, &u).await?;
    st.reports().set_status(id, status, now()).await?;
    st.log("info", &format!("{} set bug report #{id} ({}) to {}", u.username, r.title, cfg.words.status_label(status)));
    Ok(Json(json!({ "ok": true })))
}

/// DELETE /api/bug-reports/{id}: delete a report and its files (admins).
pub(crate) async fn remove<S: BugReportsHost>(State(st): State<S>, Admin(u): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    let r = load(&st, id, &u).await?;
    st.reports().delete_report(id).await?;
    let _ = tokio::fs::remove_dir_all(reports_dir(&st).join(id.to_string())).await;
    st.log("info", &format!("{} deleted bug report #{id} ({})", u.username, r.title));
    Ok(Json(json!({ "ok": true })))
}

/// `filename="…"; filename*=UTF-8''…` for a Content-Disposition.
fn disposition(inline: bool, name: &str) -> HeaderValue {
    let ascii: String = name.chars().map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' }).collect();
    let v =
        format!("{}; filename=\"{ascii}\"; filename*=UTF-8''{}", if inline { "inline" } else { "attachment" }, urlencoding::encode(name));
    HeaderValue::from_str(&v).unwrap_or(HeaderValue::from_static("attachment"))
}

/// GET /api/bug-reports/{id}/files/{fid}: an attached file.
pub(crate) async fn file<S: BugReportsHost>(
    State(st): State<S>,
    SignedIn(u): SignedIn,
    Path((id, fid)): Path<(i64, i64)>,
) -> Result<Response> {
    load(&st, id, &u).await?;
    let f = st.reports().file(id, fid).await?.ok_or_else(|| err(404, "File not found"))?;
    if f.path.is_empty() {
        return Err(err(404, "File not found"));
    }
    let data = tokio::fs::read(reports_dir(&st).join(&f.path)).await.map_err(|_| err(404, "File not found"))?;
    let mime = f.mime;
    let inline = mime.starts_with("image/");
    let mut resp = Body::from(data).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap_or(HeaderValue::from_static("application/octet-stream")));
    h.insert(header::CONTENT_DISPOSITION, disposition(inline, &f.name));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=3600"));
    Ok(resp)
}
