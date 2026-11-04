//! The answer of `GET /api/settings/schema` (contracts/settings-schema.md): the pages, cards and fields
//! of the Settings screen, with each field's words, type, options, visibility rule and whether the environment
//! locks it. Values are not in it (`GET /api/settings` has them).

use crate::table::{Field, Table};
use serde_json::{json, Value};

/// A whole number without a fraction (65535, not 65535.0).
fn number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

/// The schema answer of `table`; `env_locked` says whether an environment variable sets a key on this server.
pub fn answer(table: &Table, env_locked: impl Fn(&str) -> bool) -> Value {
    let field = |f: &Field| {
        let d = table.def(f.key).expect("SCHEMA fields are in TABLE");
        json!({
            "key": f.key,
            "label": f.label,
            "help": f.help,
            "type": f.kind,
            "options": f.options.iter().map(|(value, label)| json!({ "value": value, "label": label })).collect::<Vec<_>>(),
            "when": f.when,
            "secret": d.secret,
            "env": d.env,
            "env_locked": env_locked(f.key),
            "default": d.default.to_json(),
            "unit": f.unit,
            "min": f.min.map(number),
            "max": f.max.map(number),
            "step": f.step.map(number),
            "full": f.full,
            "advanced": f.advanced,
        })
    };
    let sections: Vec<Value> = table
        .sections
        .iter()
        .map(|s| {
            let groups: Vec<Value> = s
                .groups
                .iter()
                .map(|g| {
                    json!({
                        "id": g.id,
                        "title": g.title,
                        "part": g.part,
                        "desc": g.desc,
                        "test": g.test,
                        "test_label": g.test_label,
                        "fields": g.fields.iter().map(field).collect::<Vec<_>>(),
                    })
                })
                .collect();
            json!({ "id": s.id, "title": s.title, "aliases": s.aliases, "groups": groups })
        })
        .collect();
    json!({ "sections": sections })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_keep_their_fraction_only_when_they_have_one() {
        assert_eq!(number(65535.0), json!(65535));
        assert_eq!(number(0.1), json!(0.1));
        assert_eq!(number(1e16), json!(1e16));
    }
}
