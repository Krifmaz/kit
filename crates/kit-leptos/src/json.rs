//! Reading JSON answers the way Encore's JavaScript-born code reads them.
//!
//! From Encore's `web-rs/src/json.rs` (`js_string`, `truthy`, the `J` trait): `String(v ?? '')`, JavaScript
//! truthiness, `obj[key] || []` and friends, so the components read every answer exactly as Encore's pages do.

use serde_json::Value;

/// `String(v ?? '')`: a string as is, a whole number without `.0`, `null` as "".
pub fn js_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f.abs() < 1e15 {
                    format!("{}", f as i64)
                } else {
                    f.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// JavaScript truthiness.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Encore's `J`: the JavaScript ways of reading an object's fields.
pub trait J {
    /// `obj[key]` (`null` when missing).
    fn g(&self, key: &str) -> &Value;
    /// `String(obj[key] ?? '')`.
    fn s(&self, key: &str) -> String;
    /// `obj[key]` as a string if it is a non-empty string.
    fn st(&self, key: &str) -> Option<&str>;
    /// `Number(obj[key]) || 0`.
    fn f(&self, key: &str) -> f64;
    /// `!!obj[key]`.
    fn truthy(&self, key: &str) -> bool;
    /// `obj[key] || []`.
    fn a(&self, key: &str) -> Vec<Value>;
    /// `obj[key] || {}`.
    fn o(&self, key: &str) -> Value;
    /// `key in obj`.
    fn has(&self, key: &str) -> bool;
    /// `obj[key] == null`.
    fn is_nullish(&self, key: &str) -> bool;
}

impl J for Value {
    fn g(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&Value::Null)
    }
    fn s(&self, key: &str) -> String {
        js_string(self.g(key))
    }
    fn st(&self, key: &str) -> Option<&str> {
        self.g(key).as_str().filter(|s| !s.is_empty())
    }
    fn f(&self, key: &str) -> f64 {
        let v = self.g(key);
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            .or_else(|| v.as_bool().map(|b| if b { 1.0 } else { 0.0 }))
            .unwrap_or(0.0)
    }
    fn truthy(&self, key: &str) -> bool {
        truthy(self.g(key))
    }
    fn a(&self, key: &str) -> Vec<Value> {
        self.g(key).as_array().cloned().unwrap_or_default()
    }
    fn o(&self, key: &str) -> Value {
        let v = self.g(key);
        if v.is_object() {
            v.clone()
        } else {
            Value::Object(Default::default())
        }
    }
    fn has(&self, key: &str) -> bool {
        self.as_object().map(|o| o.contains_key(key)).unwrap_or(false)
    }
    fn is_nullish(&self, key: &str) -> bool {
        self.g(key).is_null()
    }
}
