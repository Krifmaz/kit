//! The database the crates use: the app's own, behind a small trait. Rows come back as JSON objects and
//! parameters go in as JSON values, as in Encore's `encore_db`, so a crate keeps Encore's SQL unchanged.

use crate::Result;
use serde_json::{Map, Value};

/// One result row: column name → value.
pub type Row = Map<String, Value>;

/// The app's database. Statements are SQLite's (Encore's SQL); each call runs one statement, on the caller's
/// thread, as Encore's handlers do.
pub trait Db: Send + Sync {
    /// Rows of a statement.
    fn query(&self, sql: &str, params: Vec<Value>) -> Result<Vec<Row>>;
    /// Runs a statement; returns the last inserted row id.
    fn execute(&self, sql: &str, params: Vec<Value>) -> Result<i64>;
    /// Runs several statements (a table the crate creates on first use).
    fn execute_batch(&self, sql: &str) -> Result<()>;
    /// The first row of a statement, if any.
    fn one(&self, sql: &str, params: Vec<Value>) -> Result<Option<Row>> {
        Ok(self.query(sql, params)?.into_iter().next())
    }
}

/// `p![a, b, c]`: query parameters of any JSON-convertible type.
#[macro_export]
macro_rules! p {
    () => { Vec::<$crate::__json::Value>::new() };
    ($($x:expr),+ $(,)?) => { vec![$($crate::__json::json!($x)),+] };
}

/// Small accessors for JSON rows (Encore's `RowExt`, same answers).
pub trait RowExt {
    /// The column as text (empty for null or missing; other values as JSON).
    fn s(&self, k: &str) -> String;
    /// The column as an integer (0 for null, missing or not a number; reals truncated).
    fn i(&self, k: &str) -> i64;
    /// The column as a real (0 when not a number).
    fn f(&self, k: &str) -> f64;
    /// The column as a boolean (`true`, or a non-zero number).
    fn b(&self, k: &str) -> bool;
}

impl RowExt for Row {
    fn s(&self, k: &str) -> String {
        match self.get(k) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) | None => String::new(),
            Some(v) => v.to_string(),
        }
    }
    fn i(&self, k: &str) -> i64 {
        self.get(k).and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64))).unwrap_or(0)
    }
    fn f(&self, k: &str) -> f64 {
        self.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0)
    }
    fn b(&self, k: &str) -> bool {
        match self.get(k) {
            Some(Value::Bool(b)) => *b,
            Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
            _ => false,
        }
    }
}

/// Seconds since 1970 as a real, the time stamp Encore's tables keep.
pub fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn row_accessors_match_encore() {
        let r: Row = json!({ "s": "a", "n": 3.7, "i": 2, "t": true, "z": null }).as_object().unwrap().clone();
        assert_eq!(r.s("s"), "a");
        assert_eq!(r.s("i"), "2");
        assert_eq!(r.s("z"), "");
        assert_eq!(r.i("n"), 3);
        assert_eq!(r.i("missing"), 0);
        assert_eq!(r.f("i"), 2.0);
        assert!(r.b("t") && r.b("i") && !r.b("z"));
        assert_eq!(p![1, "x"], vec![json!(1), json!("x")]);
        assert!(p![].is_empty());
        assert!(now() > 1.7e9);
    }
}
