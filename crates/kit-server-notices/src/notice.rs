//! The notice object ([`Notice`]), its [`Severity`] and the Check again toasts ([`Toasts`]).

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// How serious a notice is, for its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// worth knowing
    Info,
    /// something wants a look
    Warn,
    /// something is broken
    Err,
}

impl Severity {
    /// The severity as the answer names it (`info`, `warn`, `err`).
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Err => "err",
        }
    }
}

/// A notice as a check made it: `{ok, text?, checking?, …}` plus any fields of the app's own (when it was
/// checked, what it looked at, …). The object is kept as it is, so an app's own answer stays byte for byte
/// the same.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Notice(Map<String, Value>);

impl Notice {
    /// Everything is fine: `{"ok": true, "text": null}`.
    pub fn fine() -> Notice {
        Notice::from(serde_json::json!({ "ok": true, "text": null }))
    }

    /// Something is wrong, in this sentence (the banner shows it as is): `{"ok": false, "text": …}`.
    pub fn failing(text: impl Into<String>) -> Notice {
        Notice::from(serde_json::json!({ "ok": false, "text": text.into() }))
    }

    /// The same notice with `checking` (a check still runs: a folder that does not answer).
    pub fn checking(self, checking: bool) -> Notice {
        self.with("checking", Value::Bool(checking))
    }

    /// The same notice with a field of the app's own.
    pub fn with(mut self, key: impl Into<String>, value: Value) -> Notice {
        self.0.insert(key.into(), value);
        self
    }

    /// `false`: the banner shows; `true`: nothing shows (a notice without `ok` counts as fine).
    pub fn ok(&self) -> bool {
        self.0.get("ok").and_then(Value::as_bool) != Some(false)
    }

    /// The sentence the banner shows.
    pub fn text(&self) -> Option<&str> {
        self.0.get("text").and_then(Value::as_str).filter(|t| !t.is_empty())
    }

    /// Whether a check still runs.
    pub fn is_checking(&self) -> bool {
        self.0.get("checking").and_then(Value::as_bool) == Some(true)
    }

    /// Whether a banner shows: not ok, and a sentence to show (no banner without one).
    pub fn shows(&self) -> bool {
        !self.ok() && self.text().is_some()
    }

    /// Only whether it is fine (`{"ok": …}`): what someone who is not an admin is told.
    pub fn plain(&self) -> Notice {
        Notice::default().with("ok", Value::Bool(self.ok()))
    }

    /// A field of the notice.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// The notice as JSON.
    pub fn into_json(self) -> Value {
        Value::Object(self.0)
    }
}

/// A notice an app built as JSON, kept as it is (a value that is not an object becomes an empty notice,
/// which counts as fine).
impl From<Value> for Notice {
    fn from(v: Value) -> Notice {
        match v {
            Value::Object(m) => Notice(m),
            _ => Notice::default(),
        }
    }
}

impl From<Notice> for Value {
    fn from(n: Notice) -> Value {
        n.into_json()
    }
}

/// The three sentences of Check again's toast for one notice; the app gives them (for a disk notice, say:
/// "The disk is still full", "Still checking the disk", "The disk has room again").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Toasts {
    /// still failing (a `warn` toast)
    pub failing: String,
    /// still checking (a `warn` toast)
    pub checking: String,
    /// fixed (an `ok` toast)
    pub fixed: String,
}

/// A toast to show: its sentence and its kind (`warn` or `ok`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toast<'a> {
    /// the sentence
    pub text: &'a str,
    /// `warn` or `ok`
    pub kind: &'static str,
}

impl Toasts {
    /// The three sentences.
    pub fn new(failing: impl Into<String>, checking: impl Into<String>, fixed: impl Into<String>) -> Toasts {
        Toasts { failing: failing.into(), checking: checking.into(), fixed: fixed.into() }
    }

    /// The toast after Check again answered `n`: still failing, else still checking, else fixed.
    pub fn after(&self, n: &Notice) -> Toast<'_> {
        if !n.ok() {
            Toast { text: &self.failing, kind: "warn" }
        } else if n.is_checking() {
            Toast { text: &self.checking, kind: "warn" }
        } else {
            Toast { text: &self.fixed, kind: "ok" }
        }
    }
}
