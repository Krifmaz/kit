//! The settings schema as the server describes it (`contracts/settings-schema.md`): pages (sections), the cards
//! on each page (groups) and the fields on each card, with their words, type, options and visibility rule.

use crate::json::{js_string, J};
use serde_json::Value;
use std::sync::Arc;

/// A field's visibility rule: `key=value` (shown while the select `key` has that value) or `key!=value` (shown
/// while it has any other).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct When {
    /// The select it follows.
    pub key: String,
    /// The value it compares with.
    pub value: String,
    /// true for `=`, false for `!=`.
    pub equal: bool,
}

impl When {
    /// Reads `key=value` or `key!=value`; None for anything else (the field is then always shown).
    pub fn parse(rule: &str) -> Option<When> {
        let (key, value, equal) = match rule.split_once("!=") {
            Some((k, v)) => (k, v, false),
            None => {
                let (k, v) = rule.split_once('=')?;
                (k, v, true)
            }
        };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() || value.is_empty() {
            return None;
        }
        Some(When { key: key.to_string(), value: value.to_string(), equal })
    }

    /// Whether the field shows while the select holds `current`.
    pub fn shows(&self, current: &str) -> bool {
        (current == self.value) == self.equal
    }
}

/// One field of a card.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The setting's key (`app_name`).
    pub key: String,
    /// Its label.
    pub label: String,
    /// The line under the field.
    pub help: Option<String>,
    /// `text`, `password`, `number`, `bool`, `select`, `checks`, `textarea`, `path`, `url` (path and url are
    /// drawn as text, so is a type this client does not know).
    pub kind: String,
    /// `(value, label)` for `select` and `checks`.
    pub options: Vec<(String, String)>,
    /// The visibility rule.
    pub when: Option<When>,
    /// The server never sends the value back (passwords, pasted cookies).
    pub secret: bool,
    /// The field takes the card's whole width.
    pub full: bool,
    /// A number input's step, as the attribute reads it ("0.1").
    pub step: Option<String>,
    /// Drawn under the card's "Show advanced" fold.
    pub advanced: bool,
}

impl Field {
    /// A field from the schema's JSON.
    pub fn from_json(f: &Value) -> Field {
        let text = |k: &str| f.st(k).map(str::to_string);
        Field {
            key: f.s("key"),
            label: f.s("label"),
            help: text("help"),
            kind: text("type").unwrap_or_else(|| "text".to_string()),
            options: f.a("options").iter().map(|o| (o.s("value"), o.s("label"))).collect(),
            when: f.st("when").and_then(When::parse),
            secret: f.truthy("secret"),
            full: f.truthy("full"),
            step: if f.is_nullish("step") { None } else { Some(js_string(f.g("step"))) },
            advanced: f.truthy("advanced"),
        }
    }

    /// A password field or a secret text area: the input starts empty and an empty input keeps the saved value.
    pub fn is_secret(&self) -> bool {
        self.kind == "password" || self.secret
    }
}

/// A card: a run of fields with Save (and Test when `test` is set).
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// Its id (`general`).
    pub id: String,
    /// Its title.
    pub title: String,
    /// The line under the title.
    pub desc: String,
    /// The heading over a run of cards ("" for none).
    pub part: String,
    /// The card's Test calls `POST` on the configured [`test`](crate::config::Paths::test) path with this name.
    pub test: Option<String>,
    /// The Test button's words ("Test connection" when missing).
    pub test_label: Option<String>,
    /// Its fields.
    pub fields: Vec<Field>,
}

impl Group {
    /// A card from the schema's JSON.
    pub fn from_json(g: &Value) -> Group {
        Group {
            id: g.s("id"),
            title: g.s("title"),
            desc: g.s("desc"),
            part: g.s("part"),
            test: g.st("test").map(str::to_string),
            test_label: g.st("test_label").map(str::to_string),
            fields: g.a("fields").iter().map(Field::from_json).collect(),
        }
    }

    /// The field `key`.
    pub fn field(&self, key: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.key == key)
    }
}

/// A settings page.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Its id (`general`).
    pub id: String,
    /// Its title in the nav.
    pub title: String,
    /// Older `?s=` names that open this page.
    pub aliases: Vec<String>,
    /// Its cards.
    pub groups: Vec<Arc<Group>>,
}

/// The whole answer of `GET /api/settings/schema` (the configured
/// [`settings_schema`](crate::config::Paths::settings_schema)).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schema {
    /// The pages.
    pub sections: Vec<Section>,
}

impl Schema {
    /// The schema from its JSON.
    pub fn from_json(v: &Value) -> Schema {
        Schema {
            sections: v
                .a("sections")
                .iter()
                .map(|s| Section {
                    id: s.s("id"),
                    title: s.s("title"),
                    aliases: s.a("aliases").iter().map(js_string).collect(),
                    groups: s.a("groups").iter().map(|g| Arc::new(Group::from_json(g))).collect(),
                })
                .collect(),
        }
    }

    /// Page `id`.
    pub fn section(&self, id: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.id == id)
    }

    /// Card `id`, on whatever page.
    pub fn group(&self, id: &str) -> Option<Arc<Group>> {
        self.sections.iter().flat_map(|s| s.groups.iter()).find(|g| g.id == id).cloned()
    }

    /// The page that shows `id`: a page's own id, one of its aliases, or the id of one of its cards, in that
    /// order; anything else as it is.
    pub fn page_of(&self, id: &str) -> String {
        let page = self
            .sections
            .iter()
            .find(|s| s.id == id)
            .or_else(|| self.sections.iter().find(|s| s.aliases.iter().any(|a| a == id)))
            .or_else(|| self.sections.iter().find(|s| s.groups.iter().any(|g| g.id == id)));
        page.map(|s| s.id.clone()).unwrap_or_else(|| id.to_string())
    }

    /// Whether page `id` holds the card `card`.
    pub fn holds(&self, id: &str, card: &str) -> bool {
        self.section(id).is_some_and(|s| s.groups.iter().any(|g| g.id == card))
    }
}

/// The values of a `checks` field compare without case, spaces and punctuation: a saved "Read only" ticks the
/// "readonly" box.
pub fn type_key(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect()
}

/// The cards of a page that this server has: each with the fields the values hold, cards with none left out.
pub fn visible_cards(groups: &[Arc<Group>], values: &Value) -> Vec<(Arc<Group>, Vec<Field>)> {
    groups
        .iter()
        .map(|g| (g.clone(), g.fields.iter().filter(|f| values.has(&f.key)).cloned().collect::<Vec<_>>()))
        .filter(|(_, fields)| !fields.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_rules_read_like_the_server_writes_them() {
        assert_eq!(When::parse("mail_mode=smtp"), Some(When { key: "mail_mode".into(), value: "smtp".into(), equal: true }));
        assert_eq!(When::parse("storage!=local"), Some(When { key: "storage".into(), value: "local".into(), equal: false }));
        assert_eq!(When::parse("nothing"), None);
        assert_eq!(When::parse("=x"), None);
        assert_eq!(When::parse("k="), None);
        let w = When::parse("storage!=local").unwrap();
        assert!(w.shows("s3") && !w.shows("local"));
        assert_eq!(type_key("Read only"), "readonly");
    }
}
