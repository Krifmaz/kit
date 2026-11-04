//! Attached files: the form is read, each file checked by its bytes and named safely, the user's quota kept,
//! and the files written under `<data_dir>/reports/<report_id>/<file_id>.<ext>`.

use crate::store::NewFile;
use crate::{mb, reports_dir, BugReportsHost, Limits, MAX_NAME};
use axum::extract::Multipart;
use axum::http::StatusCode;
use kit_server_core::db::now;
use kit_server_core::{err, Account, Error, Result};
use std::collections::HashMap;

/// A file taken from the form, checked and ready to be written.
pub(crate) struct Pending {
    /// the file's name, made safe
    pub(crate) name: String,
    /// its type
    pub(crate) mime: &'static str,
    /// the extension it is saved with
    pub(crate) ext: &'static str,
    /// its bytes
    pub(crate) data: Vec<u8>,
}

/// A bug report form: its text fields and its files.
pub(crate) struct Form {
    /// the text fields
    pub(crate) fields: HashMap<String, String>,
    /// the checked files
    pub(crate) files: Vec<Pending>,
}

impl Form {
    /// A text field's value; "" when it isn't there.
    pub(crate) fn text(&self, key: &str) -> &str {
        self.fields.get(key).map(|s| s.as_str()).unwrap_or("")
    }
}

/// A name a file system takes: control characters and `\ / : * ? " < > |` become `_`, trimmed.
fn safe_file_name(s: &str) -> String {
    s.chars().map(|c| if c.is_control() || "\\/:*?\"<>|".contains(c) { '_' } else { c }).collect::<String>().trim().to_string()
}

/// The file's own name without any folder parts, with the characters a file system refuses replaced and the
/// extension the bytes were sniffed as (never the one the browser sent: `readme.html` holding a PNG is stored as
/// `readme.png`), cut to 120 characters; `file.<ext>` when nothing is left.
pub fn safe_name(name: &str, ext: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    // control characters are dropped here, not replaced
    let base: String = base.chars().filter(|c| !c.is_control()).collect();
    let clean = safe_file_name(&base);
    let clean = clean.trim_start_matches('.');
    let stem = clean.rsplit_once('.').map(|(s, _)| s).unwrap_or(clean).trim().trim_start_matches('.');
    let stem: String = stem.chars().take(MAX_NAME - ext.len() - 1).collect();
    if stem.trim().is_empty() {
        format!("file.{ext}")
    } else {
        format!("{}.{ext}", stem.trim())
    }
}

/// What the bytes are – never what the browser said. Images are sniffed and decoded with the `image` crate, so a
/// broken file is refused, and so is one that would decode to more than [`Limits::max_pixel_bytes`] or
/// [`Limits::max_side`] a side (the check happens on the header, before anything that big is allocated); text
/// files are taken by their name (.txt, .log) when they are UTF-8. Answers (type, extension).
pub fn check_file(name: &str, data: &[u8], lim: &Limits) -> Result<(&'static str, &'static str)> {
    use image::ImageFormat as F;
    if let Ok(format) = image::guess_format(data) {
        let (mime, ext) = match format {
            F::Png => ("image/png", "png"),
            F::Jpeg => ("image/jpeg", "jpg"),
            F::WebP => ("image/webp", "webp"),
            F::Gif => ("image/gif", "gif"),
            _ => return Err(err(400, "Only PNG, JPEG, WebP or GIF images and .txt or .log text files can be attached")),
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(data), format);
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(lim.max_pixel_bytes);
        limits.max_image_width = Some(lim.max_side);
        limits.max_image_height = Some(lim.max_side);
        reader.limits(limits);
        reader.decode().map_err(|_| err(400, "That image can't be read"))?;
        return Ok((mime, ext));
    }
    let lower = name.to_ascii_lowercase();
    let ext = if lower.ends_with(".log") {
        "log"
    } else if lower.ends_with(".txt") {
        "txt"
    } else {
        return Err(err(400, "Only PNG, JPEG, WebP or GIF images and .txt or .log text files can be attached"));
    };
    if std::str::from_utf8(data).is_err() {
        return Err(err(400, "A text file has to be UTF-8"));
    }
    Ok(("text/plain", ext))
}

/// A form error in words: too big, or the form's own error.
fn form_error(lim: &Limits, e: axum::extract::multipart::MultipartError) -> Error {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        err(400, format!("That upload is too big: at most {} files of {} MB each", lim.max_files, mb(lim.max_file as i64)))
    } else {
        err(400, "Could not read the upload")
    }
}

/// Reads the whole form: text fields into `fields`, every `file` part (or any part with a file name) checked
/// and sized. Too many or too big files, and files of a kind we don't take, are refused with a sentence.
pub(crate) async fn read_form(st: &impl BugReportsHost, lim: &Limits, mut form: Multipart) -> Result<Form> {
    let form_error = |e| form_error(lim, e);
    let mut fields = HashMap::new();
    let mut raw: Vec<(String, Vec<u8>)> = vec![];
    while let Some(mut f) = form.next_field().await.map_err(form_error)? {
        let field = f.name().unwrap_or("").to_string();
        let file_name = f.file_name().map(|s| s.to_string());
        if field == "file" || file_name.is_some() {
            if raw.len() >= lim.max_files {
                return Err(err(400, format!("At most {} files per report", lim.max_files)));
            }
            let mut data = Vec::new();
            while let Some(chunk) = f.chunk().await.map_err(form_error)? {
                if data.len() + chunk.len() > lim.max_file {
                    return Err(err(400, format!("Each file can be at most {} MB", mb(lim.max_file as i64))));
                }
                data.extend_from_slice(&chunk);
            }
            if data.is_empty() {
                continue; // a file input left empty
            }
            raw.push((file_name.unwrap_or_default(), data));
        } else {
            let mut data = Vec::new();
            while let Some(chunk) = f.chunk().await.map_err(form_error)? {
                if data.len() + chunk.len() > 4 * lim.max_body {
                    return Err(err(400, "A text field is too long"));
                }
                data.extend_from_slice(&chunk);
            }
            fields.insert(field, String::from_utf8_lossy(&data).into_owned());
        }
    }
    if raw.is_empty() {
        return Ok(Form { fields, files: vec![] });
    }
    // decoding a few 8 MB images is not for the request thread, and not for many threads at once either
    let decodes = st.decodes();
    let _permit = decodes.acquire().await.map_err(|_| err(500, "Uploads are not being checked right now"))?;
    let lim = lim.clone();
    let files = tokio::task::spawn_blocking(move || -> Result<Vec<Pending>> {
        raw.into_iter()
            .map(|(name, data)| {
                let (mime, ext) = check_file(&name, &data, &lim)?;
                Ok(Pending { name: safe_name(&name, ext), mime, ext, data })
            })
            .collect()
    })
    .await
    .map_err(|e| err(500, e.to_string()))??;
    Ok(Form { fields, files })
}

/// The files a user's reports may hold in all: refuses `files` when they would take the user over
/// [`Limits::max_user_bytes`]. Admins are not held back.
pub(crate) async fn check_quota(st: &impl BugReportsHost, lim: &Limits, u: &Account, files: &[Pending]) -> Result<()> {
    if files.is_empty() || u.admin {
        return Ok(());
    }
    let held = st.reports().user_bytes(u.id).await?;
    let adding: i64 = files.iter().map(|f| f.data.len() as i64).sum();
    if held + adding > lim.max_user_bytes {
        return Err(err(
            400,
            format!(
                "Your reports already hold {} MB of files (the most is {} MB); the admin can delete old ones",
                held >> 20,
                mb(lim.max_user_bytes)
            ),
        ));
    }
    Ok(())
}

/// Writes the files under `<data_dir>/reports/<report_id>/` and records them.
pub(crate) async fn store_files(st: &impl BugReportsHost, report_id: i64, message_id: Option<i64>, files: Vec<Pending>) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let dir = reports_dir(st).join(report_id.to_string());
    tokio::fs::create_dir_all(&dir).await.map_err(|e| err(500, format!("Could not save the file: {e}")))?;
    for f in files {
        let row = NewFile { report_id, message_id, name: f.name, mime: f.mime.to_string(), size: f.data.len() as i64, created_at: now() };
        let fid = st.reports().add_file(&row).await?;
        let file_name = format!("{fid}.{}", f.ext);
        tokio::fs::write(dir.join(&file_name), &f.data).await.map_err(|e| err(500, format!("Could not save the file: {e}")))?;
        st.reports().set_file_path(fid, &format!("{report_id}/{file_name}")).await?;
    }
    Ok(())
}
