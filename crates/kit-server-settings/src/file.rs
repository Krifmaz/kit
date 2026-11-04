//! settings.toml: an app's one table of server settings, read and checked ([`parse`]) and turned into Rust
//! source ([`generate`]) by the app's build script. A file that does not fit is refused with the reason: an
//! unknown attribute, a type that does not match the default, a select without options, a field on a card with
//! no label, a card or page that does not exist, a `when` rule that does not parse or names a missing setting or
//! option, an accessor name that a method of the app's settings type or a Rust keyword already takes.
//!
//! The file's form:
//!
//! ```toml
//! [sections.general]                # a page of the Settings screen, in nav order
//! title = "General"
//! aliases = ["main"]                # other link names that open it
//!
//! [groups.general]                  # a card, in page order
//! section = "general"
//! title = "General"
//! desc = "Your server's name."
//! # part, test, test_label
//!
//! [keys.app_name]                   # a setting
//! env = "APP_NAME"
//! default = "Demo"                  # its TOML type is the setting's type
//! group = "general"                 # shown on that card (then label is needed)
//! label = "App name"
//! # secret, help, type, options, when, unit, min, max, step, full, advanced
//!
//! [keys.api_key]                    # a setting no card shows
//! default = ""
//! secret = true
//! ```

use crate::table::{DefaultValue, Field, Group, KeyDef, Section, Table};
use crate::when;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use toml::{Table as TomlTable, Value};

/// The field types a key's `type` may name.
pub const TYPES: &[&str] = &["text", "password", "number", "bool", "select", "checks", "textarea", "path", "url"];
/// The attributes a `[keys.*]` entry may have.
pub const KEY_ATTRS: &[&str] =
    &["env", "default", "secret", "group", "label", "help", "type", "options", "when", "unit", "min", "max", "step", "full", "advanced"];
/// The Rust keywords, which no setting may be named (its accessor would not compile).
pub const KEYWORDS: &[&str] = &[
    "as", "async", "await", "box", "break", "const", "continue", "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn",
    "for", "if", "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "self",
    "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use", "virtual", "where", "while",
    "yield",
];

/// How [`generate`] writes the code.
#[derive(Debug, Clone, Copy)]
pub struct Codegen<'a> {
    /// the type that gets an accessor per setting (for example `"crate::Settings"`); `None` for no accessors
    pub accessors_on: Option<&'a str>,
    /// that type's own methods, which no setting may be named (for example `load`, `get`, `str`)
    pub methods: &'a [&'a str],
}

/// One `[keys.*]` entry being read.
struct Key<'a> {
    /// the setting's name
    name: &'a str,
    /// its attributes
    t: &'a TomlTable,
    /// its field type (given, or worked out from the default)
    kind: String,
}

/// String attribute `attr` of `t` (`None` when absent; an error naming `at` when not a string).
fn s<'a>(t: &'a TomlTable, attr: &str, at: &str) -> Result<Option<&'a str>, String> {
    match t.get(attr) {
        None => Ok(None),
        Some(Value::String(v)) => Ok(Some(v)),
        Some(_) => Err(format!("{at}: {attr} must be a string")),
    }
}

/// Boolean attribute `attr` of `t` (`false` when absent).
fn flag(t: &TomlTable, attr: &str, at: &str) -> Result<bool, String> {
    match t.get(attr) {
        None => Ok(false),
        Some(Value::Boolean(b)) => Ok(*b),
        Some(_) => Err(format!("{at}: {attr} must be true or false")),
    }
}

/// Number attribute `attr` of `t`, integer or float (`None` when absent).
fn num(t: &TomlTable, attr: &str, at: &str) -> Result<Option<f64>, String> {
    match t.get(attr) {
        None => Ok(None),
        Some(Value::Integer(i)) => Ok(Some(*i as f64)),
        Some(Value::Float(f)) => Ok(Some(*f)),
        Some(_) => Err(format!("{at}: {attr} must be a number")),
    }
}

/// The `[name.*]` table of the file.
fn table<'a>(root: &'a TomlTable, name: &str) -> Result<&'a TomlTable, String> {
    match root.get(name) {
        Some(Value::Table(t)) => Ok(t),
        _ => Err(format!("[{name}.*] tables are missing")),
    }
}

/// The entries of table `t`, each of which must itself be a table (`what` names them in errors).
fn entries<'a>(t: &'a TomlTable, what: &str) -> Result<Vec<(&'a str, &'a TomlTable)>, String> {
    t.iter()
        .map(|(k, v)| match v {
            Value::Table(e) => Ok((k.as_str(), e)),
            _ => Err(format!("{what}.{k} must be a table")),
        })
        .collect()
}

/// The `options` list of a key: `[value, label]` pairs.
fn options(t: &TomlTable, at: &str) -> Result<Vec<(String, String)>, String> {
    let Some(v) = t.get("options") else { return Ok(vec![]) };
    let Value::Array(items) = v else { return Err(format!("{at}: options must be a list of [value, label]")) };
    items
        .iter()
        .map(|o| match o {
            Value::Array(p) if p.len() == 2 => match (&p[0], &p[1]) {
                (Value::String(a), Value::String(b)) => Ok((a.clone(), b.clone())),
                _ => Err(format!("{at}: an option is two strings")),
            },
            _ => Err(format!("{at}: an option is [value, label]")),
        })
        .collect()
}

/// `v` for the rest of the program (a table is read once, at build time or when the app starts).
fn leak(v: &str) -> &'static str {
    Box::leak(v.to_owned().into_boxed_str())
}

/// An optional string for the rest of the program.
fn leak_opt(v: Option<&str>) -> Option<&'static str> {
    v.map(leak)
}

/// Check settings.toml (`text`) and read it as a [`Table`]: the keys A to Z and the pages with their cards and fields. `methods` are names no setting may
/// take (see [`Codegen::methods`]). The table's strings live as long as the program: read a file once.
pub fn parse(text: &str, methods: &[&str]) -> Result<Table, String> {
    let root: TomlTable = text.parse().map_err(|e| format!("{e}"))?;
    for k in root.keys() {
        if !["sections", "groups", "keys"].contains(&k.as_str()) {
            return Err(format!("unknown top-level entry {k}"));
        }
    }
    let sections = entries(table(&root, "sections")?, "sections")?;
    let groups = entries(table(&root, "groups")?, "groups")?;
    let raw_keys = entries(table(&root, "keys")?, "keys")?;

    // sections and groups
    let section_ids: HashSet<&str> = sections.iter().map(|(id, _)| *id).collect();
    for (id, t) in &sections {
        for attr in t.keys() {
            if !["title", "aliases"].contains(&attr.as_str()) {
                return Err(format!("sections.{id}: unknown attribute {attr}"));
            }
        }
        s(t, "title", id)?.ok_or(format!("sections.{id}: title is missing"))?;
    }
    for (id, t) in &groups {
        for attr in t.keys() {
            if !["section", "part", "title", "desc", "test", "test_label"].contains(&attr.as_str()) {
                return Err(format!("groups.{id}: unknown attribute {attr}"));
            }
        }
        let sec = s(t, "section", id)?.ok_or(format!("groups.{id}: section is missing"))?;
        if !section_ids.contains(sec) {
            return Err(format!("groups.{id}: no section {sec}"));
        }
        s(t, "title", id)?.ok_or(format!("groups.{id}: title is missing"))?;
        if s(t, "test", id)?.is_some() != s(t, "test_label", id)?.is_some() {
            return Err(format!("groups.{id}: test and test_label go together"));
        }
    }
    let group_ids: HashSet<&str> = groups.iter().map(|(id, _)| *id).collect();
    // a link to a page names it by its id, an alias or one of its cards: each name leads to one page only
    let mut opens: HashMap<&str, &str> = HashMap::new();
    let mut names: Vec<(&str, &str)> = sections.iter().map(|(id, _)| (*id, *id)).collect();
    for (id, t) in &sections {
        if let Some(Value::Array(a)) = t.get("aliases") {
            names.extend(a.iter().filter_map(Value::as_str).map(|alias| (alias, *id)));
        }
    }
    names.extend(groups.iter().filter_map(|(id, t)| Some((*id, t.get("section")?.as_str()?))));
    for (name, page) in names {
        match opens.insert(name, page) {
            Some(other) if other != page => return Err(format!("the name {name} would open both {other} and {page}")),
            _ => {}
        }
    }

    // keys
    let mut keys: Vec<Key> = vec![];
    for (name, t) in &raw_keys {
        let at = format!("keys.{name}");
        if !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || name.starts_with(|c: char| c.is_ascii_digit())
        {
            return Err(format!("{at}: a key is lower case letters, digits and _"));
        }
        if methods.contains(name) || KEYWORDS.contains(name) {
            return Err(format!("{at}: the name is taken by a method of the settings type or a Rust keyword"));
        }
        for attr in t.keys() {
            if !KEY_ATTRS.contains(&attr.as_str()) {
                return Err(format!("{at}: unknown attribute {attr}"));
            }
        }
        let default = t.get("default").ok_or(format!("{at}: default is missing"))?;
        let secret = flag(t, "secret", &at)?;
        let derived = match default {
            Value::Boolean(_) => "bool",
            Value::Integer(_) | Value::Float(_) => "number",
            Value::String(_) if secret => "password",
            Value::String(_) => "text",
            _ => return Err(format!("{at}: default must be a bool, number or string")),
        };
        let kind = s(t, "type", &at)?.unwrap_or(derived).to_string();
        if !TYPES.contains(&kind.as_str()) {
            return Err(format!("{at}: unknown type {kind}"));
        }
        let fits = match kind.as_str() {
            "bool" => matches!(default, Value::Boolean(_)),
            "number" => matches!(default, Value::Integer(_) | Value::Float(_)),
            "select" => matches!(default, Value::String(_) | Value::Integer(_)),
            _ => matches!(default, Value::String(_)),
        };
        if !fits {
            return Err(format!("{at}: a {kind} field cannot have the default {default:?}"));
        }
        if kind == "password" && !secret {
            return Err(format!("{at}: a password field is secret"));
        }
        let opts = options(t, &at)?;
        if matches!(kind.as_str(), "select" | "checks") == opts.is_empty() {
            return Err(format!("{at}: options are for select and checks fields, and those need them"));
        }
        if let Some(env) = s(t, "env", &at)? {
            if env.is_empty() || !env.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') {
                return Err(format!("{at}: env must be an environment variable name"));
            }
        }
        match s(t, "group", &at)? {
            Some(g) if !group_ids.contains(g) => return Err(format!("{at}: no group {g}")),
            Some(_) => {
                s(t, "label", &at)?.ok_or(format!("{at}: a field on a card needs a label"))?;
                if secret && !matches!(kind.as_str(), "password" | "textarea") {
                    return Err(format!("{at}: a secret on a card is a password or textarea field"));
                }
            }
            None => {
                for attr in ["when", "options", "unit", "min", "max", "step", "full", "advanced"] {
                    if t.contains_key(attr) && !(attr == "options" && kind == "select") {
                        return Err(format!("{at}: {attr} is for fields on a card (set group)"));
                    }
                }
            }
        }
        for attr in ["unit", "min", "max", "step"] {
            if t.contains_key(attr) && kind != "number" {
                return Err(format!("{at}: {attr} is for number fields"));
            }
        }
        for attr in ["label", "help", "unit"] {
            s(t, attr, &at)?;
        }
        for attr in ["min", "max", "step"] {
            num(t, attr, &at)?;
        }
        flag(t, "full", &at)?;
        flag(t, "advanced", &at)?;
        keys.push(Key { name, t, kind });
    }
    let by_name: BTreeMap<&str, &Key> = keys.iter().map(|k| (k.name, k)).collect();
    for k in &keys {
        let at = format!("keys.{}", k.name);
        if let Some(rule) = s(k.t, "when", &at)? {
            let w = when::parse(rule).map_err(|e| format!("{at}: when {e}"))?;
            let Some(other) = by_name.get(w.key) else { return Err(format!("{at}: when names no setting {}", w.key)) };
            if other.kind != "select" {
                return Err(format!("{at}: when names {}, which is not a select", w.key));
            }
            if !options(other.t, &at)?.iter().any(|(v, _)| v == w.value) {
                return Err(format!("{at}: when {rule}: {} has no option {}", w.key, w.value));
            }
            if w.equal && w.key == k.name {
                return Err(format!("{at}: when names the field itself"));
            }
        }
    }

    // TABLE order: A to Z
    let order: Vec<&str> = by_name.keys().copied().collect();

    // the table, in the loader's order
    let mut defs: Vec<KeyDef> = vec![];
    for name in &order {
        let k = by_name[name];
        let at = format!("keys.{name}");
        let default = match &k.t["default"] {
            Value::Boolean(b) => DefaultValue::Bool(*b),
            Value::Integer(i) => DefaultValue::Int(*i),
            Value::Float(f) => DefaultValue::Float(*f),
            Value::String(v) => DefaultValue::Text(leak(v)),
            _ => unreachable!("checked above"),
        };
        defs.push(KeyDef {
            key: leak(name),
            env: leak_opt(s(k.t, "env", &at)?),
            default,
            secret: flag(k.t, "secret", &at)?,
            kind: leak(&k.kind),
            group: leak_opt(s(k.t, "group", &at)?),
            label: leak_opt(s(k.t, "label", &at)?),
            help: leak_opt(s(k.t, "help", &at)?),
        });
    }

    // the pages, their cards (in file order) and each card's fields (the keys that name it, in file order)
    let mut pages: Vec<Section> = vec![];
    for (sid, st) in &sections {
        let aliases: Vec<&'static str> = match st.get("aliases") {
            None => vec![],
            Some(Value::Array(a)) => {
                a.iter().map(|v| v.as_str().map(leak).ok_or(format!("sections.{sid}: aliases are strings"))).collect::<Result<_, _>>()?
            }
            Some(_) => return Err(format!("sections.{sid}: aliases must be a list")),
        };
        let mut cards: Vec<Group> = vec![];
        for (gid, gt) in groups.iter().filter(|(_, gt)| gt.get("section").and_then(Value::as_str) == Some(sid)) {
            let (title, part, desc) = (s(gt, "title", gid)?.unwrap(), s(gt, "part", gid)?.unwrap_or(""), s(gt, "desc", gid)?.unwrap_or(""));
            let mut fields: Vec<Field> = vec![];
            for k in keys.iter().filter(|k| k.t.get("group").and_then(Value::as_str) == Some(gid)) {
                let at = format!("keys.{}", k.name);
                let opts: Vec<(&'static str, &'static str)> = options(k.t, &at)?.iter().map(|(v, l)| (leak(v), leak(l))).collect();
                fields.push(Field {
                    key: leak(k.name),
                    label: leak(s(k.t, "label", &at)?.unwrap()),
                    help: leak_opt(s(k.t, "help", &at)?),
                    kind: leak(&k.kind),
                    options: opts.leak(),
                    when: leak_opt(s(k.t, "when", &at)?),
                    unit: leak_opt(s(k.t, "unit", &at)?),
                    min: num(k.t, "min", &at)?,
                    max: num(k.t, "max", &at)?,
                    step: num(k.t, "step", &at)?,
                    full: flag(k.t, "full", &at)?,
                    advanced: flag(k.t, "advanced", &at)?,
                });
            }
            cards.push(Group {
                id: leak(gid),
                title: leak(title),
                part: leak(part),
                desc: leak(desc),
                test: leak_opt(s(gt, "test", gid)?),
                test_label: leak_opt(s(gt, "test_label", gid)?),
                fields: fields.leak(),
            });
        }
        pages.push(Section { id: leak(sid), title: leak(s(st, "title", sid)?.unwrap()), aliases: aliases.leak(), groups: cards.leak() });
    }
    Ok(Table { keys: defs.leak(), sections: pages.leak() })
}

/// A string as a Rust literal.
fn lit(v: &str) -> String {
    format!("{v:?}")
}

/// An `Option<&str>` as Rust source.
fn opt_lit(v: Option<&str>) -> String {
    v.map(|v| format!("Some({v:?})")).unwrap_or_else(|| "None".into())
}

/// An `Option<f64>` as Rust source.
fn opt_num(v: Option<f64>) -> String {
    v.map(|v| format!("Some({v:?})")).unwrap_or_else(|| "None".into())
}

/// The one-line description of a key: its label, or its help when it has none.
fn what(k: &KeyDef) -> &str {
    k.label.or(k.help).unwrap_or("")
}

/// Check settings.toml (`text`) and turn it into Rust source for an app's `settings_gen.rs`: `Keys` (every
/// setting's name as a constant), `TABLE` (every [`KeyDef`]), `SCHEMA` (the [`Section`]s) and, with
/// [`Codegen::accessors_on`], a typed accessor per setting on that type, which calls its `bool`, `int`, `float`
/// or `str` method. The source names [`KeyDef`], [`DefaultValue`], [`Section`], [`Group`] and [`Field`]
/// unqualified: include it where those are in scope.
///
/// An app's build.rs:
///
/// ```no_run
/// let text = std::fs::read_to_string("settings.toml").expect("settings.toml");
/// let gen = kit_server_settings::file::Codegen { accessors_on: Some("crate::Settings"), methods: &["load", "get", "str"] };
/// let code = kit_server_settings::file::generate(&text, &gen).unwrap_or_else(|e| panic!("settings.toml: {e}"));
/// std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("settings_gen.rs"), code).unwrap();
/// ```
pub fn generate(text: &str, gen: &Codegen) -> Result<String, String> {
    let table = parse(text, gen.methods)?;
    let mut o = String::new();
    o.push_str("// Generated by build.rs from settings.toml; edit that file instead.\n\n");

    // Keys
    o.push_str("/// Every setting's name: `Keys::APP_NAME` is `\"app_name\"`.\npub struct Keys;\n\nimpl Keys {\n");
    for k in table.keys {
        let name = k.key;
        let what = what(k);
        let doc = if what.is_empty() { format!("The `{name}` setting.") } else { format!("The `{name}` setting: {what}") };
        writeln!(o, "    #[doc = {}]", lit(&doc)).unwrap();
        writeln!(o, "    pub const {}: &'static str = {};", name.to_uppercase(), lit(name)).unwrap();
    }
    o.push_str("}\n\n");

    // TABLE
    o.push_str("/// Every setting, A to Z.\n");
    o.push_str("pub static TABLE: &[KeyDef] = &[\n");
    for k in table.keys {
        let default = match k.default {
            DefaultValue::Bool(b) => format!("DefaultValue::Bool({b})"),
            DefaultValue::Int(i) => format!("DefaultValue::Int({i})"),
            DefaultValue::Float(f) => format!("DefaultValue::Float({f:?})"),
            DefaultValue::Text(v) => format!("DefaultValue::Text({})", lit(v)),
        };
        writeln!(
            o,
            "    KeyDef {{ key: {}, env: {}, default: {default}, secret: {}, kind: {}, group: {}, label: {}, help: {} }},",
            lit(k.key),
            opt_lit(k.env),
            k.secret,
            lit(k.kind),
            opt_lit(k.group),
            opt_lit(k.label),
            opt_lit(k.help),
        )
        .unwrap();
    }
    o.push_str("];\n\n");

    // SCHEMA
    o.push_str("/// The Settings pages in nav order, their cards in page order, and each card's fields.\n");
    o.push_str("pub static SCHEMA: &[Section] = &[\n");
    for sec in table.sections {
        let aliases: Vec<String> = sec.aliases.iter().map(|a| lit(a)).collect();
        writeln!(o, "    Section {{ id: {}, title: {}, aliases: &[{}], groups: &[", lit(sec.id), lit(sec.title), aliases.join(", "))
            .unwrap();
        for g in sec.groups {
            writeln!(
                o,
                "        Group {{ id: {}, title: {}, part: {}, desc: {}, test: {}, test_label: {}, fields: &[",
                lit(g.id),
                lit(g.title),
                lit(g.part),
                lit(g.desc),
                opt_lit(g.test),
                opt_lit(g.test_label),
            )
            .unwrap();
            for f in g.fields {
                let opts: Vec<String> = f.options.iter().map(|(v, l)| format!("({}, {})", lit(v), lit(l))).collect();
                writeln!(
                    o,
                    "            Field {{ key: {}, label: {}, help: {}, kind: {}, options: &[{}], when: {}, unit: {}, min: {}, max: {}, step: {}, full: {}, advanced: {} }},",
                    lit(f.key),
                    lit(f.label),
                    opt_lit(f.help),
                    lit(f.kind),
                    opts.join(", "),
                    opt_lit(f.when),
                    opt_lit(f.unit),
                    opt_num(f.min),
                    opt_num(f.max),
                    opt_num(f.step),
                    f.full,
                    f.advanced,
                )
                .unwrap();
            }
            o.push_str("        ] },\n");
        }
        o.push_str("    ] },\n");
    }
    o.push_str("];\n\n");

    // accessors
    if let Some(on) = gen.accessors_on {
        o.push_str("/// A typed accessor per setting (the value in effect: environment, then saved, then default).\n");
        writeln!(o, "impl {on} {{").unwrap();
        for k in table.keys {
            let name = k.key;
            let (ret, call) = match k.default {
                DefaultValue::Bool(_) => ("bool", "bool"),
                DefaultValue::Int(_) => ("i64", "int"),
                DefaultValue::Float(_) => ("f64", "float"),
                DefaultValue::Text(_) => ("String", "str"),
            };
            let what = what(k);
            let doc = if what.is_empty() { format!("`{name}`") } else { format!("`{name}`: {what}") };
            writeln!(o, "    #[doc = {}]", lit(&doc)).unwrap();
            writeln!(o, "    pub fn {name}(&self) -> {ret} {{\n        self.{call}(Keys::{})\n    }}", name.to_uppercase()).unwrap();
        }
        o.push_str("}\n");
    }
    Ok(o)
}
