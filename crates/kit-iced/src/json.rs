//! Reading the server's JSON answers leniently: a field as text, and JavaScript truthiness.

use serde_json::Value;

/// A field as text: strings as they are, numbers and booleans written out, anything else "".
pub fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// Whether a field is set the way JavaScript reads it: true, a non-zero number, a non-empty string,
/// a non-empty list or any object.
pub fn truthy(v: &Value, key: &str) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(_)) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fields_read_like_the_desktop_app() {
        let v = json!({ "a": "x", "n": 3, "b": true, "z": 0, "e": "", "l": [], "o": {} });
        assert_eq!(s(&v, "a"), "x");
        assert_eq!(s(&v, "n"), "3");
        assert_eq!(s(&v, "missing"), "");
        assert!(truthy(&v, "n") && truthy(&v, "b") && truthy(&v, "o") && truthy(&v, "a"));
        assert!(!truthy(&v, "z") && !truthy(&v, "e") && !truthy(&v, "l") && !truthy(&v, "missing"));
    }
}
