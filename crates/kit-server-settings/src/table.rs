//! The settings table as Rust data: every setting ([`KeyDef`]) and the Settings pages ([`Section`]) with their
//! cards ([`Group`]) and fields ([`Field`]). An app's build script generates them from its settings.toml
//! ([`crate::file::generate`]) as `static` items; [`crate::file::parse`] gives the same data at run time.

use serde_json::{json, Value};

/// A setting's default. Its kind is the setting's type: the loader coerces saved and environment values to it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DefaultValue {
    /// a checkbox
    Bool(bool),
    /// a whole number
    Int(i64),
    /// a real number
    Float(f64),
    /// text
    Text(&'static str),
}

impl DefaultValue {
    /// The default as JSON.
    pub fn to_json(self) -> Value {
        match self {
            DefaultValue::Bool(b) => json!(b),
            DefaultValue::Int(i) => json!(i),
            DefaultValue::Float(f) => json!(f),
            DefaultValue::Text(s) => json!(s),
        }
    }
}

/// One setting.
#[derive(Debug)]
pub struct KeyDef {
    /// the setting's name
    pub key: &'static str,
    /// the environment variable that sets it (and locks it in the UI)
    pub env: Option<&'static str>,
    /// the value until one is saved
    pub default: DefaultValue,
    /// never sent to a browser
    pub secret: bool,
    /// text, password, number, bool, select, checks, textarea, path or url
    pub kind: &'static str,
    /// the card on the Settings pages that shows it; None: edited elsewhere, or kept by the server itself
    pub group: Option<&'static str>,
    /// the field's label
    pub label: Option<&'static str>,
    /// the explanation under the field
    pub help: Option<&'static str>,
}

/// A field on a card.
#[derive(Debug)]
pub struct Field {
    /// the setting it edits
    pub key: &'static str,
    /// its label
    pub label: &'static str,
    /// the explanation under it
    pub help: Option<&'static str>,
    /// the input type (text, password, number, bool, select, checks, textarea, path, url)
    pub kind: &'static str,
    /// (value, label) for select and checks
    pub options: &'static [(&'static str, &'static str)],
    /// visibility rule, `key=value` or `key!=value` (see [`crate::when`])
    pub when: Option<&'static str>,
    /// shown after the input ("min", "MB", …)
    pub unit: Option<&'static str>,
    /// the lowest number accepted
    pub min: Option<f64>,
    /// the highest number accepted
    pub max: Option<f64>,
    /// the step of a number input
    pub step: Option<f64>,
    /// takes the card's whole width
    pub full: bool,
    /// hidden unless "Show advanced" is on
    pub advanced: bool,
}

/// A card.
#[derive(Debug)]
pub struct Group {
    /// the card's id (also its anchor)
    pub id: &'static str,
    /// its title
    pub title: &'static str,
    /// the heading over a run of cards on the page ("" for none)
    pub part: &'static str,
    /// its description
    pub desc: &'static str,
    /// the card's Test button: POST /api/system/test/<test>
    pub test: Option<&'static str>,
    /// the Test button's text
    pub test_label: Option<&'static str>,
    /// its fields, in order
    pub fields: &'static [Field],
}

/// A page under Settings → Server. A page with no groups is drawn by its own code in the web app.
#[derive(Debug)]
pub struct Section {
    /// the page's id (`?s=`)
    pub id: &'static str,
    /// its title in the nav
    pub title: &'static str,
    /// old `?s=` names that open this page
    pub aliases: &'static [&'static str],
    /// its cards, in order
    pub groups: &'static [Group],
}

/// A whole table: every setting in the loader's order and the pages in nav order. An app makes it from its
/// generated statics (`Table { keys: TABLE, sections: SCHEMA }`).
#[derive(Debug, Clone, Copy)]
pub struct Table {
    /// every setting (Encore: A to Z, then the keys settings.toml lists last)
    pub keys: &'static [KeyDef],
    /// the Settings pages, their cards and fields
    pub sections: &'static [Section],
}

impl Table {
    /// The entry for `key`.
    pub fn def(&self, key: &str) -> Option<&'static KeyDef> {
        self.keys.iter().find(|d| d.key == key)
    }

    /// The default of `key`, as the loader uses it.
    pub fn default_of(&self, key: &str) -> Option<Value> {
        self.def(key).map(|d| d.default.to_json())
    }

    /// Every field on every card, page by page.
    pub fn fields(&self) -> impl Iterator<Item = &'static Field> {
        self.sections.iter().flat_map(|s| s.groups.iter()).flat_map(|g| g.fields.iter())
    }
}
