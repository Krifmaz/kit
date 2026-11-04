//! Setting values: how a saved or environment value becomes the setting's type ([`coerce`], [`clean_env`]), the
//! mask a secret is sent as ([`MASK`]), and [`Values`], what the settings routes need from wherever an app
//! keeps its values ([`crate::Store`] is the kit's own, on the app's database).

use crate::table::Table;
use serde_json::{json, Value};
use std::sync::Arc;

/// Sent to the browser in place of a secret's value; a value that starts with it was not changed in the UI.
pub const MASK: &str = "••••";

/// One setting as the loader keeps it (built from the table's [`crate::KeyDef`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Def {
    /// the environment variable that sets it (and locks it in the UI)
    pub env: Option<String>,
    /// the value until one is saved, as JSON
    pub default: Value,
    /// never sent to a browser
    pub secret: bool,
}

/// Where the environment variables come from: the process's own in use, a fixed set in tests.
pub type EnvLookup = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

/// Whether `v` is the masked form of a secret (sent back unchanged from the UI).
pub fn is_masked(v: &Value) -> bool {
    v.as_str().map(|s| s.starts_with(MASK)).unwrap_or(false)
}

/// `raw` (saved, or from the environment) as the type of `default`: "1"/"yes" for a boolean, a numeric string
/// for a number, any value as text; the default when it cannot be read as that type.
pub fn coerce(default: &Value, raw: &Value) -> Value {
    match default {
        Value::Bool(_) => match raw {
            Value::Bool(b) => Value::Bool(*b),
            Value::Number(n) => Value::Bool(n.as_f64().unwrap_or(0.0) != 0.0),
            Value::String(s) => Value::Bool(matches!(s.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "on")),
            _ => default.clone(),
        },
        Value::Number(n) if n.is_i64() || n.is_u64() => match raw {
            Value::Number(x) => json!(x.as_f64().unwrap_or(0.0) as i64),
            Value::String(s) => s.trim().parse::<f64>().map(|f| json!(f as i64)).unwrap_or(default.clone()),
            Value::Bool(b) => json!(*b as i64),
            _ => default.clone(),
        },
        Value::Number(_) => match raw {
            Value::Number(x) => json!(x.as_f64().unwrap_or(0.0)),
            Value::String(s) => s.trim().parse::<f64>().map(|f| json!(f)).unwrap_or(default.clone()),
            _ => default.clone(),
        },
        _ => match raw {
            Value::Null => json!(""),
            Value::String(s) => json!(s),
            other => json!(other.to_string()),
        },
    }
}

/// An environment variable's value as written: Compose list syntax keeps quotes literally (- KEY="abc"), and
/// copy/paste adds whitespace.
pub fn clean_env(raw: &str) -> String {
    let v = raw.trim();
    let b = v.as_bytes();
    if b.len() >= 2 && b[0] == b[b.len() - 1] && (b[0] == b'"' || b[0] == b'\'') {
        return v[1..v.len() - 1].trim().to_string();
    }
    v.to_string()
}

/// A secret's value as `GET /api/settings` shows it: `"•••• (saved)"` when one is set, `""` when not.
pub fn masked(v: &Value) -> Value {
    let set = !matches!(v, Value::String(s) if s.is_empty()) && !v.is_null();
    json!(if set { format!("{MASK} (saved)") } else { String::new() })
}

/// What the settings routes read and write: the app's values, wherever it keeps them. [`crate::Store`]
/// implements it; an app with its own settings type may too.
pub trait Values: Send + Sync {
    /// The table the values follow.
    fn table(&self) -> Table;
    /// Whether `key` is a setting.
    fn known(&self, key: &str) -> bool {
        self.table().def(key).is_some()
    }
    /// The value in effect for `key` (environment first, then saved or default; null for an unknown key).
    fn get(&self, key: &str) -> Value;
    /// Whether an environment variable sets `key` (the UI shows it locked).
    fn env_locked(&self, key: &str) -> bool;
    /// Save `values` (`{key: value}`), each coerced to its setting's type; unknown keys are skipped.
    fn set_many(&self, values: &Value);
    /// Every setting for the admin UI, secrets masked, with `_meta` (`{source, env}` per key).
    fn dump(&self) -> Value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_take_their_settings_type() {
        assert_eq!(coerce(&json!(false), &json!("Yes ")), json!(true));
        assert_eq!(coerce(&json!(false), &json!(0)), json!(false));
        assert_eq!(coerce(&json!(false), &json!(null)), json!(false));
        assert_eq!(coerce(&json!(30), &json!("12.7")), json!(12));
        assert_eq!(coerce(&json!(30), &json!("x")), json!(30));
        assert_eq!(coerce(&json!(30), &json!(true)), json!(1));
        assert_eq!(coerce(&json!(1.5), &json!("0.25")), json!(0.25));
        assert_eq!(coerce(&json!(1.5), &json!(true)), json!(1.5));
        assert_eq!(coerce(&json!(""), &json!(null)), json!(""));
        assert_eq!(coerce(&json!(""), &json!(5)), json!("5"));
    }

    #[test]
    fn environment_values_lose_their_quotes_and_spaces() {
        assert_eq!(clean_env(" \"abc\" "), "abc");
        assert_eq!(clean_env("'a b '"), "a b");
        assert_eq!(clean_env("\"abc'"), "\"abc'");
        assert_eq!(clean_env("\""), "\"");
    }

    #[test]
    fn secrets_are_masked() {
        assert_eq!(masked(&json!("x")), json!("•••• (saved)"));
        assert_eq!(masked(&json!("")), json!(""));
        assert_eq!(masked(&json!(null)), json!(""));
        assert!(is_masked(&json!("•••• (saved)")) && !is_masked(&json!("abc")) && !is_masked(&json!(1)));
    }
}
