//! The report handlers: file, list, count unread, read, reply, set the status, delete, and fetch a file.

use crate::files::{check_quota, read_form, store_files};
use crate::{
    cut, ensure_table, line, reports_dir, status_label, BugReportsHost, KINDS, MAX_BODY, MAX_TITLE, REPLIES_PER_HOUR, REPORTS_PER_HOUR,
    STATUSES,
};
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use kit_server_core::db::now;
use kit_server_core::{err, p, Account, Admin, Result, Row, RowExt, SignedIn};
use serde::Deserialize;
use serde_json::{json, Value};

/// The report, or 404 – also when it is someone else's and the viewer is no admin.
pub(crate) fn load(st: &impl BugReportsHost, id: i64, u: &Account) -> Result<Row> {
    let r = st.db().one("SELECT * FROM bug_reports WHERE id=?", p![id])?.ok_or_else(|| err(404, "Report not found"))?;
    if !u.admin && r.i("user_id") != u.id {
        return Err(err(404, "Report not found"));
    }
    Ok(r)
}

/// A report with its files and its thread.
pub(crate) struct Full {
    /// the report's row
    pub(crate) report: Row,
    /// the files attached to the report itself
    pub(crate) files: Vec<Row>,
    /// the replies, each with its files
    pub(crate) messages: Vec<(Row, Vec<Row>)>,
}

/// A report with its files and its replies.
pub(crate) fn load_full(st: &impl BugReportsHost, report: Row) -> Result<Full> {
    let id = report.i("id");
    let all = st.db().query("SELECT id, message_id, name, mime, size, path FROM bug_report_files WHERE report_id=? ORDER BY id", p![id])?;
    let files = all.iter().filter(|f| f.get("message_id").is_none_or(Value::is_null)).cloned().collect();
    let messages = st
        .db()
        .query("SELECT id, user_id, username, is_admin, body, created_at FROM bug_report_messages WHERE report_id=? ORDER BY id", p![id])?
        .into_iter()
        .map(|m| {
            let mine = all.iter().filter(|f| f.i("message_id") == m.i("id")).cloned().collect();
            (m, mine)
        })
        .collect();
    Ok(Full { report, files, messages })
}

/// A file as the API gives it, with its download URL.
fn file_json(report_id: i64, f: &Row) -> Value {
    json!({ "id": f.i("id"), "name": f.s("name"), "mime": f.s("mime"), "size": f.i("size"), "url": format!("/api/bug-reports/{report_id}/files/{}", f.i("id")) })
}

/// A reply as the API gives it.
fn message_json(report_id: i64, m: &Row, files: &[Row]) -> Value {
    json!({ "id": m.i("id"), "username": m.s("username"), "is_admin": m.b("is_admin"), "body": m.s("body"), "created_at": m.f("created_at"),
            "files": files.iter().map(|f| file_json(report_id, f)).collect::<Vec<_>>() })
}

/// A report as the API gives it, with its files and its thread.
fn report_json(full: &Full) -> Value {
    let r = &full.report;
    let id = r.i("id");
    json!({
        "report": {
            "id": id, "kind": r.s("kind"), "title": r.s("title"), "body": r.s("body"), "page": r.s("page"), "app_version": r.s("app_version"),
            "device": r.s("device"), "status": r.s("status"), "username": r.s("username"), "user_id": r.i("user_id"),
            "created_at": r.f("created_at"), "updated_at": r.f("updated_at"),
            "files": full.files.iter().map(|f| file_json(id, f)).collect::<Vec<_>>(),
        },
        "messages": full.messages.iter().map(|(m, files)| message_json(id, m, files)).collect::<Vec<_>>(),
    })
}

/// SQL: when the report's last reply came (0 without one).
const LAST_MESSAGE: &str = "COALESCE((SELECT MAX(m.created_at) FROM bug_report_messages m WHERE m.report_id=r.id), 0)";

/// The SQL of "this report has something the viewer hasn't seen": for an admin a report nobody looked at
/// yet, or a user's reply after the last look; for the reporter an admin's reply after the last look.
fn unread_sql(admin: bool) -> String {
    if admin {
        format!("(r.admin_seen_at < r.created_at OR (r.last_reply_by='user' AND {LAST_MESSAGE} > r.admin_seen_at))")
    } else {
        format!("(r.last_reply_by='admin' AND {LAST_MESSAGE} > r.user_seen_at)")
    }
}

/// `GET /api/bug-reports` (and export) query.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ListQ {
    /// only this status
    pub status: String,
    /// only this kind
    pub kind: String,
    /// a search text
    pub q: String,
    /// "1" for the caller's own only (admins)
    pub mine: String,
    /// export only: `1,2,3`
    pub ids: String,
    /// export only: `txt` for one plain text file instead of the zip
    pub format: String,
}

/// WHERE … for a list: the filters that are set, and the viewer's own reports unless an admin asks for all.
pub(crate) fn filter_sql(q: &ListQ, u: &Account, admin: bool) -> (String, Vec<Value>) {
    let mut where_ = vec!["1=1".to_string()];
    let mut params = p![];
    if STATUSES.contains(&q.status.as_str()) {
        where_.push("r.status=?".into());
        params.push(json!(q.status));
    }
    if KINDS.contains(&q.kind.as_str()) {
        where_.push("r.kind=?".into());
        params.push(json!(q.kind));
    }
    let search = q.q.trim();
    if !search.is_empty() {
        where_.push("(r.title LIKE ? ESCAPE '\\' OR r.body LIKE ? ESCAPE '\\' OR r.username LIKE ? ESCAPE '\\')".into());
        let like = format!("%{}%", search.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
        params.extend([json!(like), json!(like), json!(like)]);
    }
    if !admin || matches!(q.mine.as_str(), "1" | "true") {
        where_.push("r.user_id=?".into());
        params.push(json!(u.id));
    }
    let ids: Vec<i64> = q.ids.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    if !ids.is_empty() {
        where_.push(format!("r.id IN ({})", vec!["?"; ids.len()].join(",")));
        params.extend(ids.into_iter().map(Value::from));
    }
    (where_.join(" AND "), params)
}

/// POST /api/bug-reports: file a report with up to [`crate::MAX_FILES`] files (20 an hour for non-admins).
pub async fn create<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn, form: Multipart) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let admin = u.admin;
    if !admin && st.limited(&format!("bugreport:{}", u.id), REPORTS_PER_HOUR, 3600.0) {
        return Err(err(429, "Too many reports in a short time – try again later"));
    }
    let form = read_form(&st, form).await?;
    let title = line(form.text("title"));
    let body = form.text("body").trim().to_string();
    if title.is_empty() {
        return Err(err(400, "Give the report a title"));
    }
    if title.chars().count() > MAX_TITLE {
        return Err(err(400, format!("The title can have at most {MAX_TITLE} characters")));
    }
    if body.is_empty() {
        return Err(err(400, "Describe what happened"));
    }
    if body.chars().count() > MAX_BODY {
        return Err(err(400, format!("The description can have at most {MAX_BODY} characters")));
    }
    let kind = form.text("kind").trim();
    let kind = if kind.is_empty() {
        "bug"
    } else if KINDS.contains(&kind) {
        kind
    } else {
        return Err(err(400, "Pick a kind: Bug, Not a bug or How do I…"));
    };
    check_quota(&st, &u, &form.files)?;
    let t = now();
    // an admin's own report is not news to them
    let admin_seen = if admin { t } else { 0.0 };
    let id = st.db().execute(
        "INSERT INTO bug_reports (user_id, username, kind, title, body, page, app_version, device, status, created_at, updated_at, last_reply_by, user_seen_at, admin_seen_at)
         VALUES (?,?,?,?,?,?,?,?,'new',?,?,'',?,?)",
        p![
            u.id,
            u.username,
            kind,
            title,
            body,
            cut(&line(form.text("page")), 300),
            cut(&line(form.text("app_version")), 60),
            cut(&line(form.text("device")), 300),
            t,
            t,
            t,
            admin_seen
        ],
    )?;
    store_files(&st, id, None, form.files).await?;
    st.log("info", &format!("{} filed bug report #{id}: {title}", u.username));
    tokio::spawn(st.notify("New bug report".to_string(), format!("{}: {title}", u.username)));
    Ok(Json(json!({ "ok": true, "id": id })))
}

/// GET /api/bug-reports: the reports the caller may see.
pub async fn list<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn, Query(q): Query<ListQ>) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let admin = u.admin;
    let (where_, params) = filter_sql(&q, &u, admin);
    let rows = st.db().query(
        &format!(
            "SELECT r.id, r.kind, r.title, r.status, r.username, r.user_id, r.created_at, r.updated_at,
                    (SELECT COUNT(*) FROM bug_report_messages m WHERE m.report_id=r.id) message_count,
                    (SELECT COUNT(*) FROM bug_report_files f WHERE f.report_id=r.id) file_count,
                    {} unread
             FROM bug_reports r WHERE {where_} ORDER BY r.updated_at DESC, r.id DESC",
            unread_sql(admin)
        ),
        params,
    )?;
    let reports: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.i("id"), "kind": r.s("kind"), "title": r.s("title"), "status": r.s("status"), "username": r.s("username"), "user_id": r.i("user_id"),
                "created_at": r.f("created_at"), "updated_at": r.f("updated_at"), "message_count": r.i("message_count"), "file_count": r.i("file_count"),
                "unread": r.b("unread"),
            })
        })
        .collect();
    let mut v = json!({ "reports": reports });
    st.enrich(&u, &mut v).await;
    Ok(Json(v))
}

/// GET /api/bug-reports/unread: how many reports have something the caller hasn't seen.
pub async fn unread<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let admin = u.admin;
    let own = if admin { "" } else { "AND r.user_id=?" };
    let params = if admin { p![] } else { p![u.id] };
    let r = st.db().one(&format!("SELECT COUNT(*) n FROM bug_reports r WHERE {} {own}", unread_sql(admin)), params)?;
    Ok(Json(json!({ "count": r.map(|r| r.i("n")).unwrap_or(0) })))
}

/// GET /api/bug-reports/{id}: a report with its thread; marks it seen.
pub async fn detail<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn, Path(id): Path<i64>) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let r = load(&st, id, &u)?;
    let t = now();
    if u.admin {
        st.db().execute("UPDATE bug_reports SET admin_seen_at=? WHERE id=?", p![t, id])?;
    }
    if r.i("user_id") == u.id {
        st.db().execute("UPDATE bug_reports SET user_seen_at=? WHERE id=?", p![t, id])?;
    }
    let mut v = report_json(&load_full(&st, r)?);
    st.enrich(&u, &mut v).await;
    Ok(Json(v))
}

/// POST /api/bug-reports/{id}/messages: reply to a report.
pub async fn reply<S: BugReportsHost>(
    State(st): State<S>,
    SignedIn(u): SignedIn,
    Path(id): Path<i64>,
    form: Multipart,
) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let r = load(&st, id, &u)?;
    let admin = u.admin;
    if !admin && st.limited(&format!("bugreply:{}", u.id), REPLIES_PER_HOUR, 3600.0) {
        return Err(err(429, "Too many replies in a short time – try again later"));
    }
    let form = read_form(&st, form).await?;
    let body = form.text("body").trim().to_string();
    if body.is_empty() && form.files.is_empty() {
        return Err(err(400, "Write a reply or attach a file"));
    }
    if body.chars().count() > MAX_BODY {
        return Err(err(400, format!("A reply can have at most {MAX_BODY} characters")));
    }
    check_quota(&st, &u, &form.files)?;
    let t = now();
    let mid = st.db().execute(
        "INSERT INTO bug_report_messages (report_id, user_id, username, is_admin, body, created_at) VALUES (?,?,?,?,?,?)",
        p![id, u.id, u.username, admin as i64, body, t],
    )?;
    store_files(&st, id, Some(mid), form.files).await?;
    let status = r.s("status");
    let title = r.s("title");
    if admin {
        let status = if status == "new" { "answered".to_string() } else { status };
        st.db().execute(
            "UPDATE bug_reports SET last_reply_by='admin', status=?, updated_at=?, admin_seen_at=? WHERE id=?",
            p![status, t, t, id],
        )?;
        // the reporter hears about it by mail, when mail is set up and they have an address (not for a reply
        // to one's own report)
        if st.mail_enabled() && r.i("user_id") != u.id {
            let to = st.mail_address(r.i("user_id"))?;
            if !to.trim().is_empty() {
                let base = st.base_url();
                let link = if base.trim().is_empty() {
                    format!("#/bug-reports/{id}")
                } else {
                    format!("{}/#/bug-reports/{id}", base.trim().trim_end_matches('/'))
                };
                let subject = format!("{}: a reply to your report “{title}”", st.app_name());
                let text = format!("{} answered your report “{title}”:\n\n{body}\n\n{link}\n", u.username);
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
        st.db().execute(
            "UPDATE bug_reports SET last_reply_by='user', status=?, updated_at=?, user_seen_at=? WHERE id=?",
            p![status, t, t, id],
        )?;
        let who = &u.username;
        let text = if body.is_empty() {
            format!("{who} on #{id} {title}: (a file)")
        } else {
            format!("{who} on #{id} {title}: {}", cut(&body, 300))
        };
        tokio::spawn(st.notify("Bug report reply".to_string(), text));
    }
    Ok(Json(json!({ "ok": true, "id": mid })))
}

/// `POST /api/bug-reports/{id}/status` body.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct StatusBody {
    /// one of [`STATUSES`]
    pub status: String,
}

/// POST /api/bug-reports/{id}/status: set a report's status (admins).
pub async fn set_status<S: BugReportsHost>(
    State(st): State<S>,
    Admin(u): Admin,
    Path(id): Path<i64>,
    Json(b): Json<StatusBody>,
) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let status = b.status.trim();
    if !STATUSES.contains(&status) {
        return Err(err(400, "Pick a status: New, Looking into it, Fixed, Won't fix or Answered"));
    }
    let r = load(&st, id, &u)?;
    st.db().execute("UPDATE bug_reports SET status=?, updated_at=? WHERE id=?", p![status, now(), id])?;
    st.log("info", &format!("{} set bug report #{id} ({}) to {}", u.username, r.s("title"), status_label(status)));
    Ok(Json(json!({ "ok": true })))
}

/// DELETE /api/bug-reports/{id}: delete a report and its files (admins).
pub async fn remove<S: BugReportsHost>(State(st): State<S>, Admin(u): Admin, Path(id): Path<i64>) -> Result<Json<Value>> {
    ensure_table(&st)?;
    let r = load(&st, id, &u)?;
    st.db().execute("DELETE FROM bug_report_files WHERE report_id=?", p![id])?;
    st.db().execute("DELETE FROM bug_report_messages WHERE report_id=?", p![id])?;
    st.db().execute("DELETE FROM bug_reports WHERE id=?", p![id])?;
    let _ = tokio::fs::remove_dir_all(reports_dir(&st).join(id.to_string())).await;
    st.log("info", &format!("{} deleted bug report #{id} ({})", u.username, r.s("title")));
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
pub async fn file<S: BugReportsHost>(State(st): State<S>, SignedIn(u): SignedIn, Path((id, fid)): Path<(i64, i64)>) -> Result<Response> {
    ensure_table(&st)?;
    load(&st, id, &u)?;
    let f = st
        .db()
        .one("SELECT name, mime, path FROM bug_report_files WHERE id=? AND report_id=?", p![fid, id])?
        .ok_or_else(|| err(404, "File not found"))?;
    let rel = f.s("path");
    if rel.is_empty() {
        return Err(err(404, "File not found"));
    }
    let data = tokio::fs::read(reports_dir(&st).join(&rel)).await.map_err(|_| err(404, "File not found"))?;
    let mime = f.s("mime");
    let inline = mime.starts_with("image/");
    let mut resp = Body::from(data).into_response();
    let h = resp.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap_or(HeaderValue::from_static("application/octet-stream")));
    h.insert(header::CONTENT_DISPOSITION, disposition(inline, &f.s("name")));
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("private, max-age=3600"));
    Ok(resp)
}
