//! One line of the diagnostics page: [`Check`], with its [`Kind`] and kit [`Status`].

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// What was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// a connection to another service
    Service,
    /// a folder
    Path,
    /// a program
    Tool,
}

impl Kind {
    /// The kind as the answer names it (`service`, `path`, `tool`).
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Service => "service",
            Kind::Path => "path",
            Kind::Tool => "tool",
        }
    }

    /// The kind a word names; `None` for any other word.
    pub fn parse(s: &str) -> Option<Kind> {
        match s {
            "service" => Some(Kind::Service),
            "path" => Some(Kind::Path),
            "tool" => Some(Kind::Tool),
            _ => None,
        }
    }
}

/// How a check came out, as the page colours it (finer than `ok`: a check can pass but want a look).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// passed
    Ok,
    /// works, but something wants a look
    Warn,
    /// failed
    Fail,
}

impl Status {
    /// The status as the answer names it (`ok`, `warn`, `fail`).
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }

    /// The status a word names; `None` for anything else (an app's check may carry an HTTP status code under
    /// `status`, which is not a kit status).
    pub fn parse(v: &Value) -> Option<Status> {
        match v.as_str()? {
            "ok" => Some(Status::Ok),
            "warn" => Some(Status::Warn),
            "fail" => Some(Status::Fail),
            _ => None,
        }
    }
}

/// One check: `{name, kind, request?, ok, detail, status?, fix?, …}` plus any fields of the app's own.
///
/// It keeps the JSON object as it is (a check an app built as JSON comes out byte for byte the same), with
/// typed builders and readers for the contract's fields.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Check(Map<String, Value>);

impl Check {
    /// A check of `name` (what was checked, in plain words) that passed or not, with its result in one plain
    /// sentence.
    pub fn new(name: impl Into<String>, kind: Kind, ok: bool, detail: impl Into<String>) -> Check {
        let mut m = Map::new();
        m.insert("name".into(), Value::String(name.into()));
        m.insert("kind".into(), Value::String(kind.as_str().into()));
        m.insert("ok".into(), Value::Bool(ok));
        m.insert("detail".into(), Value::String(detail.into()));
        Check(m)
    }

    /// The same check with what was asked (`GET https://…`, a folder's path, a command line), shown so it can
    /// be copied.
    pub fn request(self, request: impl Into<String>) -> Check {
        self.with("request", Value::String(request.into()))
    }

    /// The same check with a kit status (`warn` for a check that passed but wants a look).
    pub fn status(self, status: Status) -> Check {
        self.with("status", Value::String(status.as_str().into()))
    }

    /// The same check with how to fix it, in plain words.
    pub fn fix(self, fix: impl Into<String>) -> Check {
        self.with("fix", Value::String(fix.into()))
    }

    /// The same check with a field of the app's own (`ms`, the response `body`, …).
    pub fn with(mut self, key: impl Into<String>, value: Value) -> Check {
        self.0.insert(key.into(), value);
        self
    }

    /// What was checked.
    pub fn name(&self) -> &str {
        self.0.get("name").and_then(Value::as_str).unwrap_or("")
    }

    /// What kind of thing was checked; `None` when the check names none the kit knows.
    pub fn kind(&self) -> Option<Kind> {
        self.0.get("kind").and_then(Value::as_str).and_then(Kind::parse)
    }

    /// Whether it passed.
    pub fn ok(&self) -> bool {
        self.0.get("ok").and_then(Value::as_bool).unwrap_or(false)
    }

    /// The result in one plain sentence.
    pub fn detail(&self) -> &str {
        self.0.get("detail").and_then(Value::as_str).unwrap_or("")
    }

    /// How it came out: the kit status when the check has one, else `ok` when it passed and `fail` when not.
    pub fn outcome(&self) -> Status {
        match self.0.get("status").and_then(Status::parse) {
            Some(s) => s,
            None if self.ok() => Status::Ok,
            None => Status::Fail,
        }
    }

    /// How to fix it, when the check says.
    pub fn fix_text(&self) -> Option<&str> {
        self.0.get("fix").and_then(Value::as_str)
    }

    /// A field of the check.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// The check as JSON, as the answer lists it.
    pub fn into_json(self) -> Value {
        Value::Object(self.0)
    }
}

/// A check an app built as JSON, kept as it is (a value that is not an object becomes an empty check).
impl From<Value> for Check {
    fn from(v: Value) -> Check {
        match v {
            Value::Object(m) => Check(m),
            _ => Check::default(),
        }
    }
}

impl From<Check> for Value {
    fn from(c: Check) -> Value {
        c.into_json()
    }
}
