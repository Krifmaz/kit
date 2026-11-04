//! A field's visibility rule from settings.toml: `key=value` (shown while the select `key` has that value)
//! or `key!=value` (shown while it has any other). [`crate::file::parse`] checks every rule with it.

/// A parsed rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct When<'a> {
    /// the setting the rule looks at
    pub key: &'a str,
    /// the value it compares with
    pub value: &'a str,
    /// true for `=`, false for `!=`
    pub equal: bool,
}

/// Reads `key=value` or `key!=value`. The key is a setting name (a-z, 0-9, _), the value is not empty.
pub fn parse(rule: &str) -> Result<When<'_>, String> {
    let (key, value, equal) = match rule.split_once("!=") {
        Some((k, v)) => (k, v, false),
        None => match rule.split_once('=') {
            Some((k, v)) => (k, v, true),
            None => return Err(format!("{rule:?}: expected key=value or key!=value")),
        },
    };
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
        return Err(format!("{rule:?}: the key must be a setting name"));
    }
    if value.is_empty() || value.contains('=') || value.trim() != value {
        return Err(format!("{rule:?}: the value must be one word with no = and no spaces around it"));
    }
    Ok(When { key, value, equal })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_parse_as_key_and_value() {
        assert_eq!(parse("storage=s3"), Ok(When { key: "storage", value: "s3", equal: true }));
        assert_eq!(parse("mail_mode!=off"), Ok(When { key: "mail_mode", value: "off", equal: false }));
        for bad in ["", "storage", "=s3", "storage=", "Storage=x", "a b=c", "a=b=c", "a= b", "a!="] {
            assert!(parse(bad).is_err(), "{bad:?} should not parse");
        }
        assert_eq!(parse("x").unwrap_err(), "\"x\": expected key=value or key!=value");
    }
}
