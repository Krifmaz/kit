//! The error answer: every failure is `{"detail": "<plain sentence>"}` with an HTTP status, as Encore's
//! `encore_core::error` answers, so Encore's clients show kit errors exactly as they show Encore's own.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// A failed request: the HTTP status, the `detail` the client shows in full (and lets the person copy), and
/// any more fields of the body.
#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    /// HTTP status of the response
    pub status: StatusCode,
    /// the message, sent as `{"detail": …}`
    pub detail: String,
    /// more fields merged into the error body
    pub extra: Option<Value>,
}

/// What a kit handler returns.
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    /// An error with HTTP `status` (500 when not a valid code) and `detail`.
    pub fn new(status: u16, detail: impl Into<String>) -> Self {
        Error { status: StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), detail: detail.into(), extra: None }
    }

    /// The same error with `extra` fields in its body.
    pub fn with(mut self, extra: Value) -> Self {
        self.extra = Some(extra);
        self
    }

    /// A database failure: logged with its cause, answered as Encore does ("Database error", 500).
    pub fn database(cause: impl std::fmt::Display) -> Self {
        tracing::error!("database: {cause}");
        Error::new(500, "Database error")
    }
}

/// Shorthand for [`Error::new`].
pub fn err(status: u16, detail: impl Into<String>) -> Error {
    Error::new(status, detail)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.detail, self.status.as_u16())
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let mut body = json!({ "detail": self.detail });
        if let (Some(Value::Object(extra)), Value::Object(b)) = (self.extra, &mut body) {
            b.extend(extra);
        }
        (self.status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_detail_with_extra_fields() {
        let e = err(429, "Too many reports").with(json!({ "retry_after": 60 }));
        assert_eq!(e.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(e.to_string(), "Too many reports (429)");
        let r = e.into_response();
        assert_eq!(r.status(), 429);
    }

    #[test]
    fn a_bad_status_is_500() {
        assert_eq!(err(1000, "x").status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(Error::database("disk I/O error").detail, "Database error");
    }
}
