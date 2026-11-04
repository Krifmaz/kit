//! A settings form drawn from a table of fields: a desktop app's Settings → Server form, with its
//! cards that fold open, two fields per row,
//! switches, selects, check boxes, text areas, secrets with Show saved / Copy, the note under each field
//! about where its value comes from, and the card's Save and Test buttons.
//!
//! The fields come either from a table written in the app ([`field`], [`Field`]'s `const` builders and
//! [`Card::new`], say an app's `SECTIONS`) or from the server's answer to `GET /api/admin/settings/schema`
//! ([`Schema::from_json`], `contracts/settings-schema.md`).
//!
//! [`Form`] keeps what was typed and what is open; it holds no network code. [`Form::update`] applies a
//! [`Msg`] and answers with what the app must do next ([`Out`]): save a card with this body, run a test,
//! fetch a secret, copy a text, show a toast. The app makes the request and sends the answer back as a
//! [`Msg`] (`Values`, `Saved`, `Tested`, `Revealed`, `Copied`, `Cleared`).
//!
//! A table written as `Card { .. }` literals moves over by writing `card! {` where it wrote `Card {` (see
//! [`card!`](crate::card)).

use crate::settings::{help, input, input_locked, link_muted, result_line_copy, select_table, switch_maybe, two};
use crate::theme::{self, Btn};
use crate::widgets::text_area::text_area;
use crate::widgets::{self, btn_icon, btn_sm, icon, icon_btn, t, El, Icon};
use iced::widget::text_editor;
use iced::widget::{button, checkbox, column, container, row};
use iced::{Alignment, Length};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// A text of the table: borrowed from a `const` table, or owned when it came from the server's schema.
pub type Str = Cow<'static, str>;

/// The toast after a card was saved.
pub const SAVED: &str = "Settings saved";
/// The toast after a secret was cleared.
pub const CLEARED: &str = "Cleared";
/// The line under a card while its test runs.
pub const TESTING: &str = "Testing…";
/// A passed test whose answer has no message.
pub const CONNECTED: &str = "Connected";
/// A failed test whose answer has no message.
pub const FAILED: &str = "Failed";
/// A test that could not run and said nothing.
pub const TEST_FAILED: &str = "Test failed";
/// The Test button's words when the card names none (the web's old default).
pub const TEST_CONNECTION: &str = "Test connection";

const fn borrowed(s: &'static str) -> Str {
    Cow::Borrowed(s)
}

type StaticPairs<'a> = &'a [(&'a str, &'a str)];

/// The options of a `select` or `checks` field: `(value, label)` pairs, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pairs {
    /// From a `const` table.
    Static(&'static [(&'static str, &'static str)]),
    /// From the server's schema.
    Owned(Vec<(String, String)>),
}

impl Pairs {
    /// The pairs, in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> + Clone + '_ {
        let (a, b): (StaticPairs, &[(String, String)]) = match self {
            Pairs::Static(s) => (s, &[]),
            Pairs::Owned(v) => (&[], v),
        };
        a.iter().map(|(v, l)| (*v, *l)).chain(b.iter().map(|(v, l)| (v.as_str(), l.as_str())))
    }

    /// How many options there are.
    pub fn len(&self) -> usize {
        match self {
            Pairs::Static(s) => s.len(),
            Pairs::Owned(v) => v.len(),
        }
    }

    /// No options.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The values of a [`When`] rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Words {
    /// From a `const` table.
    Static(&'static [&'static str]),
    /// From the server's schema.
    Owned(Vec<String>),
}

impl Words {
    /// The words, in order.
    pub fn iter(&self) -> impl Iterator<Item = &str> + '_ {
        let (a, b): (&[&str], &[String]) = match self {
            Words::Static(s) => (s, &[]),
            Words::Owned(v) => (&[], v),
        };
        a.iter().copied().chain(b.iter().map(String::as_str))
    }

    /// Whether `w` is one of them.
    pub fn contains(&self, w: &str) -> bool {
        self.iter().any(|x| x == w)
    }
}

/// A field's visibility rule (`when` in the schema): shown only while the select `key` has one of
/// `values` (`key=value`), or, with `not`, while it has none of them (`key!=value`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct When {
    /// The select the rule reads.
    pub key: Str,
    /// The values it compares with.
    pub values: Words,
    /// `!=`: shown while the select has none of the values.
    pub not: bool,
}

impl When {
    /// `key=value`: shown while the select `key` has one of `values`.
    pub const fn is(key: &'static str, values: &'static [&'static str]) -> When {
        When { key: borrowed(key), values: Words::Static(values), not: false }
    }

    /// `key!=value`: shown while the select `key` has none of `values`.
    pub const fn is_not(key: &'static str, values: &'static [&'static str]) -> When {
        When { key: borrowed(key), values: Words::Static(values), not: true }
    }

    /// The schema's rule text: `"key=value"` or `"key!=value"`.
    pub fn parse(rule: &str) -> Option<When> {
        let (key, value, not) = match rule.split_once("!=") {
            Some((k, v)) => (k, v, true),
            None => {
                let (k, v) = rule.split_once('=')?;
                (k, v, false)
            }
        };
        let (key, value) = (key.trim(), value.trim());
        if key.is_empty() {
            return None;
        }
        Some(When { key: Cow::Owned(key.to_string()), values: Words::Owned(vec![value.to_string()]), not })
    }

    /// Whether a field with this rule shows while its select has `current`.
    pub fn holds(&self, current: &str) -> bool {
        self.values.contains(current) != self.not
    }
}

/// One setting of the form: the web's `S(key, label, type, extra)`.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The setting's name in `GET/PUT /api/admin/settings`.
    pub key: Str,
    /// The field's label.
    pub label: Str,
    /// How it is drawn: `text`, `path`, `url`, `number`, `password`, `bool`, `select`, `checks`, `textarea`.
    pub kind: Str,
    /// The line under the field.
    pub help: Option<Str>,
    /// Takes the whole row.
    pub full: bool,
    /// A textarea whose saved value is never sent back (a private key, cookies).
    pub secret: bool,
    /// The number input's step (`"0.1"`); `None`: whole numbers.
    pub step: Option<Str>,
    /// The options of a `select` or `checks` field.
    pub options: Pairs,
    /// Shown only while a select has (or has not) one of some values.
    pub when: Option<When>,
}

/// A field of a `const` table (the web's `s(key, label, type)`).
pub const fn field(key: &'static str, label: &'static str, kind: &'static str) -> Field {
    Field::new(key, label, kind)
}

impl Field {
    /// A field with no help, options or rule (the web's `s(key, label, type)`).
    pub const fn new(key: &'static str, label: &'static str, kind: &'static str) -> Field {
        Field {
            key: borrowed(key),
            label: borrowed(label),
            kind: borrowed(kind),
            help: None,
            full: false,
            secret: false,
            step: None,
            options: Pairs::Static(&[]),
            when: None,
        }
    }

    /// With a help line under it.
    pub const fn help(mut self, h: &'static str) -> Self {
        std::mem::forget(self.help.replace(borrowed(h)));
        self
    }

    /// Taking the whole row.
    pub const fn full(mut self) -> Self {
        self.full = true;
        self
    }

    /// A text area whose saved value is never sent back.
    pub const fn secret(mut self) -> Self {
        self.secret = true;
        self
    }

    /// With a number step (`"0.1"`).
    pub const fn step(mut self, st: &'static str) -> Self {
        std::mem::forget(self.step.replace(borrowed(st)));
        self
    }

    /// With the options of a `select` or `checks`.
    pub const fn options(mut self, o: &'static [(&'static str, &'static str)]) -> Self {
        std::mem::forget(std::mem::replace(&mut self.options, Pairs::Static(o)));
        self
    }

    /// Shown only while the select `key` has one of `values`.
    pub const fn when(mut self, key: &'static str, values: &'static [&'static str]) -> Self {
        std::mem::forget(self.when.replace(When::is(key, values)));
        self
    }

    /// Shown only while the select `key` has none of `values`.
    pub const fn when_not(mut self, key: &'static str, values: &'static [&'static str]) -> Self {
        std::mem::forget(self.when.replace(When::is_not(key, values)));
        self
    }

    /// A password or a secret textarea: the saved value is masked and never echoed back.
    pub fn is_secret(&self) -> bool {
        self.kind == "password" || self.secret
    }

    /// The field takes the whole row of the form grid.
    pub fn spans(&self) -> bool {
        self.full || self.kind == "textarea" || self.kind == "checks"
    }

    /// A field of the schema answer (`sections[].groups[].fields[]`).
    pub fn from_schema(f: &Value) -> Result<Field, String> {
        let key = req(f, "key", "A field")?;
        let label = req(f, "label", &format!("The field {key}"))?;
        let kind = req(f, "type", &format!("The field {key}"))?;
        let when = match f["when"].as_str() {
            Some(rule) => Some(When::parse(rule).ok_or_else(|| format!("The field {key} has a rule that does not parse: {rule}"))?),
            None => None,
        };
        let options: Vec<(String, String)> =
            f["options"].as_array().map(|a| a.iter().map(|o| (vs(&o["value"]), vs(&o["label"]))).collect()).unwrap_or_default();
        Ok(Field {
            key: Cow::Owned(key),
            label: Cow::Owned(label),
            kind: Cow::Owned(kind),
            help: f["help"].as_str().map(|h| Cow::Owned(h.to_string())),
            full: f["full"].as_bool().unwrap_or(false),
            secret: f["secret"].as_bool().unwrap_or(false),
            step: (!f["step"].is_null()).then(|| Cow::Owned(vs(&f["step"]))),
            options: Pairs::Owned(options),
            when,
        })
    }
}

/// A string attribute that must be there.
fn req(v: &Value, name: &str, what: &str) -> Result<String, String> {
    match v[name].as_str() {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(format!("{what} has no {name}")),
    }
}

/// The server's key for an option of a `checks` field: lower case letters and digits only (how `checks` values
/// compare).
pub fn type_key(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect()
}

/// One card of the form (a `<details>` on the web): a titled group of fields with an optional Test
/// button (`POST /api/system/test/<test>`).
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// The card's id (also its old `?s=` name).
    pub id: Str,
    /// The heading.
    pub title: Str,
    /// The line under the heading.
    pub desc: Str,
    /// The heading over a run of cards on the page; `""` for none.
    pub part: Str,
    /// The test the Test button runs; `None`: no button.
    pub test: Option<Str>,
    /// The Test button's words; `None`: [`TEST_CONNECTION`].
    pub test_label: Option<Str>,
    /// The fields, in order.
    pub fields: Cow<'static, [Field]>,
}

const fn opt(s: Option<&'static str>) -> Option<Str> {
    match s {
        Some(x) => Some(borrowed(x)),
        None => None,
    }
}

impl Card {
    /// A card of a `const` table, in the order of a `Card { id, title, desc, test, test_label, fields }` literal.
    pub const fn new(
        id: &'static str,
        title: &'static str,
        desc: &'static str,
        test: Option<&'static str>,
        test_label: Option<&'static str>,
        fields: &'static [Field],
    ) -> Card {
        Card {
            id: borrowed(id),
            title: borrowed(title),
            desc: borrowed(desc),
            part: borrowed(""),
            test: opt(test),
            test_label: opt(test_label),
            fields: Cow::Borrowed(fields),
        }
    }

    /// The Test button's words.
    pub fn test_words(&self) -> &str {
        self.test_label.as_deref().unwrap_or(TEST_CONNECTION)
    }

    /// Whether the server has any of the card's settings (the web skips a card it has none of).
    pub fn shown(&self, values: &Value) -> bool {
        self.fields.iter().any(|f| values.get(&*f.key).is_some())
    }

    /// A card of the schema answer (`sections[].groups[]`).
    pub fn from_schema(g: &Value) -> Result<Card, String> {
        let id = req(g, "id", "A card")?;
        let fields = g["fields"].as_array().map(|a| a.iter().map(Field::from_schema).collect::<Result<Vec<_>, _>>()).transpose()?;
        let text = |name: &str| Cow::Owned(g[name].as_str().unwrap_or_default().to_string());
        let maybe = |name: &str| g[name].as_str().map(|s| Cow::Owned(s.to_string()));
        Ok(Card {
            title: text("title"),
            desc: text("desc"),
            part: text("part"),
            test: maybe("test"),
            test_label: maybe("test_label"),
            fields: Cow::Owned(fields.unwrap_or_default()),
            id: Cow::Owned(id),
        })
    }
}

/// A [`Card`] of a `const` table, written like a `Card { id, title, desc, test, test_label, fields: &[..] }`
/// literal (a const table cannot pass `&[..]` of fields straight to [`Card::new`]):
///
/// ```
/// use kit_iced::card;
/// use kit_iced::form::{field, Card};
/// const SECTIONS: &[Card] = &[card! { id: "email", title: "Email", desc: "Lets the app send password-reset and sign-in emails.",
///     test: Some("email"), test_label: Some("Send test email"), fields: &[
///         field("smtp_host", "SMTP server", "text").help("e.g. smtp.fastmail.com – used for password-reset emails"),
///         field("smtp_password", "Password", "password"),
///     ] }];
/// assert_eq!(SECTIONS[0].fields[1].kind, "password");
/// ```
#[macro_export]
macro_rules! card {
    { id: $id:expr, title: $title:expr, desc: $desc:expr, test: $test:expr, test_label: $tl:expr, fields: &[$($f:expr),* $(,)?] $(,)? } => {
        $crate::form::Card::new($id, $title, $desc, $test, $tl, const { &[$($f),*] })
    };
}

/// A page under Settings → Server, from the schema answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// The page's id in links (`#/settings?s=<id>`).
    pub id: String,
    /// The nav entry and the page heading.
    pub title: String,
    /// Older `?s=` names that open this page.
    pub aliases: Vec<String>,
    /// The cards, in order; empty for a page the app draws with its own code.
    pub cards: Vec<Card>,
}

impl Section {
    /// The cards under their part headings: a new run starts where `part` changes.
    pub fn parts(&self) -> Vec<(&str, Vec<&Card>)> {
        let mut out: Vec<(&str, Vec<&Card>)> = Vec::new();
        for c in &self.cards {
            match out.last_mut() {
                Some((p, cards)) if *p == c.part => cards.push(c),
                _ => out.push((&c.part, vec![c])),
            }
        }
        out
    }
}

/// The answer of `GET /api/admin/settings/schema`: the pages, their cards and the fields on each card.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Schema {
    /// The pages, in nav order.
    pub sections: Vec<Section>,
}

impl Schema {
    /// Reads the schema answer; the error says what does not fit.
    pub fn from_json(v: &Value) -> Result<Schema, String> {
        let Some(sections) = v["sections"].as_array() else { return Err("The settings schema has no sections".into()) };
        let mut out = Vec::new();
        for s in sections {
            let id = req(s, "id", "A section")?;
            let cards = s["groups"].as_array().map(|a| a.iter().map(Card::from_schema).collect::<Result<Vec<_>, _>>()).transpose()?;
            out.push(Section {
                title: s["title"].as_str().unwrap_or_default().to_string(),
                aliases: s["aliases"]
                    .as_array()
                    .map(|a| a.iter().filter_map(Value::as_str).map(String::from).collect())
                    .unwrap_or_default(),
                cards: cards.unwrap_or_default(),
                id,
            });
        }
        Ok(Schema { sections: out })
    }

    /// Every card of every page, in order.
    pub fn cards(&self) -> impl Iterator<Item = &Card> + Clone + '_ {
        self.sections.iter().flat_map(|s| s.cards.iter())
    }

    /// The card with this id.
    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards().find(|c| c.id == id)
    }

    /// The page a `?s=` name opens: a page id first, then an alias, then the page holding a card of that id.
    pub fn find(&self, name: &str) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.id == name)
            .or_else(|| self.sections.iter().find(|s| s.aliases.iter().any(|a| a == name)))
            .or_else(|| self.sections.iter().find(|s| s.cards.iter().any(|c| c.id == name)))
    }
}

// ----- values --------------------------------------------------------------------------------

static NULL: Value = Value::Null;

/// JavaScript's `String(n)` of a number.
fn js_num(n: f64) -> String {
    if !n.is_finite() {
        return "0".into();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let s = format!("{n:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A setting's value as the text an input shows (`""` for none, numbers as JavaScript writes them).
pub fn vs(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_f64().map(js_num).unwrap_or_else(|| n.to_string()),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// JavaScript truthiness of a value.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

/// `v[key]` as text when it is a string, number or bool; `""` otherwise.
fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// `_meta.<key>.source == "env"`: the setting is locked by an environment variable.
pub fn env_locked(values: &Value, key: &str) -> bool {
    values["_meta"][key]["source"].as_str() == Some("env")
}

/// The note under a field the environment locks.
pub fn env_note(values: &Value, key: &str) -> Option<String> {
    env_locked(values, key)
        .then(|| format!("🔒 Set by {} in your compose/env – change it there (environment always wins)", s(&values["_meta"][key], "env")))
}

/// The saved value the server masks (`•••• (saved)`).
pub fn is_masked(v: &str) -> bool {
    v.starts_with("••••")
}

/// `{ok, message}` of a test answer as words.
pub fn test_words(r: &Value) -> (bool, String) {
    let ok = truthy(&r["ok"]);
    let m = if truthy(&r["message"]) {
        s(r, "message")
    } else if ok {
        CONNECTED.into()
    } else {
        FAILED.into()
    };
    (ok, m)
}

// ----- the form ------------------------------------------------------------------------------

/// The form's messages. The app sends the answers of the requests [`Out`] asked for as
/// `Values`, `Saved`, `Tested`, `Revealed`, `Copied` and `Cleared`, with the error's words on failure.
#[derive(Clone, Debug)]
pub enum Msg {
    /// `GET /api/admin/settings` answered (with `_meta`).
    Values(Result<Value, String>),
    /// Fold a card open or shut.
    Toggle(String),
    /// Text typed into an input, or a select picked.
    Edit(String, String),
    /// A switch flipped.
    Bool(String, bool),
    /// A check box of a `checks` field: (key, option, ticked).
    Check(String, String, bool),
    /// An action in a text area.
    Area(String, text_editor::Action),
    /// Show a saved secret.
    Reveal(String),
    /// The secret asked for by [`Out::Reveal`] (`{value}`).
    Revealed(String, Result<Value, String>),
    /// Hide a shown secret.
    Hide(String),
    /// Copy a secret: what was typed, else the saved value.
    Copy(String),
    /// The secret asked for by [`Out::CopySecret`] (`{value}`).
    Copied(Result<Value, String>),
    /// Clear a saved secret.
    Clear(String),
    /// The save asked for by [`Out::Clear`] answered.
    Cleared(Result<Value, String>),
    /// Save a card.
    Save(String),
    /// The save asked for by [`Out::Save`] answered (the values again, with `_warnings`).
    Saved(String, Result<Value, String>),
    /// Save a card, then run its test.
    Test(String),
    /// The save and test asked for by [`Out::Test`]: (the saved values, the test's `{ok, message}`).
    Tested(String, Result<(Value, Result<Value, String>), String>),
    /// Copy a failed test's words.
    CopyText(String),
}

/// What the app does after a message.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// `PUT /api/admin/settings` with `body`, then send [`Msg::Saved`] with `card`.
    Save {
        /// The card's id.
        card: String,
        /// The settings to save.
        body: Map<String, Value>,
    },
    /// `PUT /api/admin/settings` with `body`, then `POST /api/system/test/<test>`, then send [`Msg::Tested`].
    Test {
        /// The card's id.
        card: String,
        /// The test to run.
        test: String,
        /// The settings to save first.
        body: Map<String, Value>,
    },
    /// `PUT /api/admin/settings` with `body` (`{key: ""}`), then send [`Msg::Cleared`].
    Clear {
        /// The setting cleared.
        key: String,
        /// The settings to save.
        body: Map<String, Value>,
    },
    /// `GET /api/admin/settings/secret/<key>`, then send [`Msg::Revealed`].
    Reveal {
        /// The secret's setting.
        key: String,
    },
    /// `GET /api/admin/settings/secret/<key>`, then send [`Msg::Copied`].
    CopySecret {
        /// The secret's setting.
        key: String,
    },
    /// Put this text on the clipboard.
    Copy(String),
    /// Copy this text and say whether it took.
    CopyText(String),
    /// A green toast.
    ToastOk(String),
    /// A red toast that stays.
    ToastErr(String),
}

/// What the app adds inside a card, for cards with blocks of their own (say a service's status or a list the app
/// keeps).
pub struct Slots<'a, M> {
    /// Under the test results, over the fields.
    pub top: Vec<El<'a, M>>,
    /// In place of the fields' grid (build it from [`Form::grid`] and [`Form::visible_fields`]).
    pub body: Option<El<'a, M>>,
    /// Under the fields, over Save.
    pub bottom: Vec<El<'a, M>>,
}

impl<M> Default for Slots<'_, M> {
    fn default() -> Self {
        Slots { top: Vec::new(), body: None, bottom: Vec::new() }
    }
}

/// The form's state: the values, what was typed, which cards are open and busy.
#[derive(Debug, Default)]
pub struct Form {
    /// `/api/admin/settings` (with `_meta`).
    values: Option<Value>,
    error: String,
    /// Text typed into inputs, by key (absent = the saved value shows).
    edits: HashMap<String, String>,
    /// Secrets shown in clear.
    shown: HashSet<String>,
    /// Secrets fetched from the server to show (emptied again when hidden).
    revealed: HashSet<String>,
    bools: HashMap<String, bool>,
    checks: HashMap<String, Vec<String>>,
    areas: HashMap<String, text_editor::Content>,
    /// Plain (not secret) text areas typed into: they keep their text when the values come again.
    typed: HashSet<String>,
    open: HashSet<String>,
    /// Test and save results under a card.
    flash: HashMap<String, Vec<(bool, String)>>,
    busy: HashSet<String>,
    /// The cards of the page shown, in order (the first the server has starts open).
    page: Vec<String>,
    /// The card a link asked for: unfolded instead of the first one.
    focus: Option<String>,
    /// Secrets the server never shows (no Show saved / Copy).
    never_shown: HashSet<String>,
}

impl Form {
    /// An empty form.
    pub fn new() -> Form {
        Form::default()
    }

    /// The same form with secret text areas whose saved value the server never shows (for example a
    /// private key): they get no Show saved / Copy.
    pub fn never_shown<S: Into<String>>(mut self, keys: impl IntoIterator<Item = S>) -> Form {
        self.never_shown = keys.into_iter().map(Into::into).collect();
        self
    }

    /// A page opens: forget everything typed and loaded. `page` is its cards in order; `focus` the card a
    /// link asked for. The app then loads the values and sends [`Msg::Values`].
    pub fn open<S: Into<String>>(&mut self, page: impl IntoIterator<Item = S>, focus: Option<String>) {
        self.values = None;
        self.error.clear();
        self.edits.clear();
        self.shown.clear();
        self.revealed.clear();
        self.bools.clear();
        self.checks.clear();
        self.areas.clear();
        self.typed.clear();
        self.flash.clear();
        self.busy.clear();
        self.open.clear();
        self.page = page.into_iter().map(Into::into).collect();
        self.focus = focus;
    }

    /// The values loaded (`None` while loading).
    pub fn values(&self) -> Option<&Value> {
        self.values.as_ref()
    }

    /// The error of the last load (`""` for none).
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Whether a card is folded open.
    pub fn is_open(&self, card: &str) -> bool {
        self.open.contains(card)
    }

    /// Whether a card's save or test is running.
    pub fn is_busy(&self, card: &str) -> bool {
        self.busy.contains(card)
    }

    /// The test and save results under a card.
    pub fn flash(&self, card: &str) -> &[(bool, String)] {
        self.flash.get(card).map(Vec::as_slice).unwrap_or_default()
    }

    /// The card that starts unfolded: the one a link asked for, else the first of the page the server has.
    fn first_open<'c>(&self, cards: impl IntoIterator<Item = &'c Card> + Clone) -> Option<String> {
        if self.focus.is_some() {
            return self.focus.clone();
        }
        let values = self.values.as_ref()?;
        self.page.iter().find(|id| cards.clone().into_iter().any(|c| c.id == **id && c.shown(values))).cloned()
    }

    /// After the values arrived: a text box per textarea field the server has.
    fn make_areas<'c>(&mut self, values: &Value, cards: impl IntoIterator<Item = &'c Card>) {
        for c in cards {
            for f in c.fields.iter().filter(|f| f.kind == "textarea" && values.get(&*f.key).is_some()) {
                if f.is_secret() {
                    self.areas.entry(f.key.to_string()).or_default();
                } else if !self.typed.contains(&*f.key) {
                    // a plain text area shows what is saved, so a Save sends it back whole (a secret
                    // one starts empty and is sent only when something was pasted)
                    self.areas.insert(f.key.to_string(), text_editor::Content::with_text(&vs(&values[&*f.key])));
                }
            }
        }
    }

    /// Takes new values (a load or a save's answer): drops `_warnings` and returns their messages.
    pub fn take_values<'c>(&mut self, mut v: Value, cards: impl IntoIterator<Item = &'c Card>) -> Vec<String> {
        let warnings: Vec<String> = match &v["_warnings"] {
            Value::Array(a) => a.iter().map(|w| s(w, "message")).filter(|m| !m.is_empty()).collect(),
            _ => Vec::new(),
        };
        if let Some(o) = v.as_object_mut() {
            o.remove("_warnings");
        }
        self.make_areas(&v, cards);
        self.values = Some(v);
        warnings
    }

    /// The value an input shows: what was typed, else the saved value.
    pub fn current(&self, values: &Value, key: &str) -> String {
        self.edits.get(key).cloned().unwrap_or_else(|| vs(&values[key]))
    }

    /// The `when` rule: shown only while the select it names has (or has not) one of the values.
    pub fn visible(&self, values: &Value, f: &Field) -> bool {
        match &f.when {
            None => true,
            Some(w) => w.holds(&self.current(values, &w.key)),
        }
    }

    /// The ticked options of a `checks` field, as [`type_key`]s.
    pub fn checked(&self, values: &Value, key: &str) -> Vec<String> {
        match self.checks.get(key) {
            Some(c) => c.clone(),
            None => vs(&values[key]).split(',').map(type_key).filter(|s| !s.is_empty()).collect(),
        }
    }

    /// The body of a card's Save, as the web builds it: every field the server has that the environment
    /// does not lock; secrets only when typed (an empty box keeps the saved value); checks joined in the
    /// options' order and spelling.
    pub fn save_body(&self, values: &Value, card: &Card) -> Map<String, Value> {
        let mut body = Map::new();
        for f in card.fields.iter().filter(|f| values.get(&*f.key).is_some()) {
            let key = &*f.key;
            if env_locked(values, key) {
                continue;
            }
            match &*f.kind {
                "checks" => {
                    let on = self.checked(values, key);
                    let joined: Vec<&str> = f.options.iter().filter(|(v, _)| on.contains(&type_key(v))).map(|(v, _)| v).collect();
                    body.insert(key.into(), Value::String(joined.join(",")));
                }
                "bool" => {
                    let on = self.bools.get(key).copied().unwrap_or_else(|| truthy(&values[key]));
                    body.insert(key.into(), Value::Bool(on));
                }
                "textarea" => {
                    let text = self.areas.get(key).map(|c| c.text()).unwrap_or_default();
                    if f.is_secret() && (text.trim().is_empty() || is_masked(&text)) {
                        continue;
                    }
                    body.insert(key.into(), Value::String(text.trim().to_string()));
                }
                "password" => {
                    let text = self.edits.get(key).cloned().unwrap_or_default();
                    if text.trim().is_empty() || is_masked(&text) || self.revealed.contains(key) {
                        continue;
                    }
                    body.insert(key.into(), Value::String(text.trim().to_string()));
                }
                _ => {
                    let text = self.current(values, key);
                    if is_masked(&text) {
                        continue;
                    }
                    body.insert(key.into(), Value::String(text));
                }
            }
        }
        body
    }

    /// Forget the typed values of a card (its saved values show again).
    pub fn settle(&mut self, card: &Card) {
        for f in card.fields.iter() {
            let key = &*f.key;
            self.edits.remove(key);
            self.bools.remove(key);
            self.checks.remove(key);
            self.shown.remove(key);
            self.revealed.remove(key);
            if f.kind == "textarea" {
                self.typed.remove(key);
                let saved = match &self.values {
                    Some(v) if !f.is_secret() => vs(&v[key]),
                    _ => String::new(),
                };
                if let Some(c) = self.areas.get_mut(key) {
                    *c = text_editor::Content::with_text(&saved);
                }
            }
        }
    }

    /// Applies a message. `cards` is every card the app may draw (a table: all of it; the schema:
    /// [`Schema::cards`]); the answer is what the app must do, in order.
    pub fn update<'c>(&mut self, msg: Msg, cards: impl IntoIterator<Item = &'c Card> + Clone) -> Vec<Out> {
        let find = |id: &str| cards.clone().into_iter().find(|c| c.id == id);
        match msg {
            Msg::Values(Ok(v)) => {
                self.take_values(v, cards.clone());
                if self.open.is_empty() {
                    if let Some(id) = self.first_open(cards.clone()) {
                        self.open.insert(id);
                    }
                }
            }
            Msg::Values(Err(e)) => self.error = e,
            Msg::Toggle(id) => {
                if !self.open.remove(&id) {
                    self.open.insert(id);
                }
            }
            Msg::Edit(k, v) => {
                self.revealed.remove(&k);
                self.edits.insert(k, v);
            }
            Msg::Bool(k, on) => {
                self.bools.insert(k, on);
            }
            Msg::Check(k, option, on) => {
                let mut list = self.checked(self.values.as_ref().unwrap_or(&NULL), &k);
                let key = type_key(&option);
                list.retain(|x| *x != key);
                if on {
                    list.push(key);
                }
                self.checks.insert(k, list);
            }
            Msg::Area(k, a) => {
                if a.is_edit() {
                    self.typed.insert(k.clone());
                }
                if let Some(c) = self.areas.get_mut(&k) {
                    c.perform(a);
                }
            }
            Msg::Reveal(k) => {
                if self.edits.get(&k).is_some_and(|v| !v.is_empty()) {
                    self.shown.insert(k);
                    return Vec::new();
                }
                return vec![Out::Reveal { key: k }];
            }
            Msg::Revealed(k, Ok(r)) => {
                let value = s(&r, "value");
                if let Some(c) = self.areas.get_mut(&k) {
                    *c = text_editor::Content::with_text(&value);
                } else {
                    self.edits.insert(k.clone(), value);
                }
                self.revealed.insert(k.clone());
                self.shown.insert(k);
            }
            Msg::Revealed(_, Err(e)) => return vec![Out::ToastErr(e)],
            Msg::Hide(k) => {
                self.shown.remove(&k);
                if self.revealed.remove(&k) {
                    self.edits.remove(&k);
                    if let Some(c) = self.areas.get_mut(&k) {
                        *c = text_editor::Content::new();
                    }
                }
            }
            Msg::Copy(k) => {
                let typed = self.edits.get(&k).cloned().or_else(|| self.areas.get(&k).map(|c| c.text())).unwrap_or_default();
                if !typed.trim().is_empty() {
                    return vec![Out::Copy(typed), Out::ToastOk(crate::details::COPIED.into())];
                }
                return vec![Out::CopySecret { key: k }];
            }
            Msg::Copied(Ok(r)) => return vec![Out::Copy(s(&r, "value")), Out::ToastOk(crate::details::COPIED.into())],
            Msg::Copied(Err(e)) => return vec![Out::ToastErr(e)],
            Msg::Clear(k) => {
                let mut body = Map::new();
                body.insert(k.clone(), Value::String(String::new()));
                return vec![Out::Clear { key: k, body }];
            }
            Msg::Cleared(Ok(v)) => {
                self.take_values(v, cards.clone());
                return vec![Out::ToastOk(CLEARED.into())];
            }
            Msg::Cleared(Err(e)) => return vec![Out::ToastErr(e)],
            Msg::Save(id) => {
                let (Some(values), Some(card)) = (&self.values, find(&id)) else { return Vec::new() };
                let body = self.save_body(values, card);
                self.busy.insert(id.clone());
                self.flash.remove(&id);
                return vec![Out::Save { card: id, body }];
            }
            Msg::Saved(id, Ok(v)) => {
                self.busy.remove(&id);
                let warnings = self.take_values(v, cards.clone());
                if let Some(card) = find(&id) {
                    self.settle(card);
                }
                if !warnings.is_empty() {
                    self.flash.insert(id, warnings.into_iter().map(|w| (false, w)).collect());
                }
                return vec![Out::ToastOk(SAVED.into())];
            }
            Msg::Saved(id, Err(e)) => {
                self.busy.remove(&id);
                return vec![Out::ToastErr(e)];
            }
            Msg::Test(id) => {
                let (Some(values), Some(card)) = (&self.values, find(&id)) else { return Vec::new() };
                let Some(test) = card.test.clone() else { return Vec::new() };
                let body = self.save_body(values, card);
                self.busy.insert(id.clone());
                self.open.insert(id.clone());
                self.flash.insert(id.clone(), vec![(true, TESTING.into())]);
                return vec![Out::Test { card: id, test: test.into_owned(), body }];
            }
            Msg::Tested(id, Ok((saved, tested))) => {
                self.busy.remove(&id);
                let mut lines: Vec<(bool, String)> = self.take_values(saved, cards.clone()).into_iter().map(|w| (false, w)).collect();
                if let Some(card) = find(&id) {
                    self.settle(card);
                }
                match tested {
                    Ok(r) => lines.push(test_words(&r)),
                    Err(e) => lines.push((false, if e.is_empty() { TEST_FAILED.into() } else { e })),
                }
                self.flash.insert(id, lines);
            }
            Msg::Tested(id, Err(e)) => {
                self.busy.remove(&id);
                self.flash.insert(id, vec![(false, if e.is_empty() { TEST_FAILED.into() } else { e })]);
            }
            Msg::CopyText(text) => return vec![Out::CopyText(text)],
        }
        Vec::new()
    }

    // ----- views ---------------------------------------------------------------------------

    /// The fields of a card the server has and whose `when` rule holds, in order.
    pub fn visible_fields<'c>(&self, card: &'c Card) -> Vec<&'c Field> {
        let values = self.values.as_ref().unwrap_or(&NULL);
        card.fields.iter().filter(|f| values.get(&*f.key).is_some() && self.visible(values, f)).collect()
    }

    /// The note under a field about where its value comes from.
    fn source_note<'a>(&self, f: &Field, values: &Value) -> Option<El<'a, Msg>> {
        let m = &values["_meta"][&*f.key];
        let set = truthy(&values[&*f.key]);
        if let Some(note) = env_note(values, &f.key) {
            return Some(help(note).into());
        }
        if f.is_secret() && set {
            return Some(
                row![help("Saved ✓ (hidden) ·"), link_muted("clear", Msg::Clear(f.key.to_string()))]
                    .spacing(4)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
        if f.is_secret() {
            let env = if truthy(&m["env"]) { format!(" – or set {} in your compose file", s(m, "env")) } else { String::new() };
            return Some(help(format!("Not set{env}")).into());
        }
        if m["source"].as_str() == Some("ui") {
            return None;
        }
        if truthy(&m["env"]) {
            return Some(widgets::faint(format!("Default · env var {}", s(m, "env"))).size(12).into());
        }
        None
    }

    /// One field: its input, its help line and the note on where its value comes from. `reveal`: the
    /// viewer may see saved secrets (Show / Copy).
    pub fn field_view<'a>(&'a self, f: &'a Field, reveal: bool) -> El<'a, Msg> {
        let values = self.values.as_ref().unwrap_or(&NULL);
        let key = f.key.to_string();
        let locked = env_locked(values, &f.key);
        let v = &values[&*f.key];
        let mut lines: Vec<El<'a, Msg>> = Vec::new();
        match &*f.kind {
            "bool" => {
                let on = self.bools.get(&*f.key).copied().unwrap_or_else(|| truthy(v));
                let k = key.clone();
                lines.push(switch_maybe(on, &*f.label, (!locked).then_some(move |x| Msg::Bool(k.clone(), x))));
            }
            "checks" => {
                let on = self.checked(values, &f.key);
                let mut boxes = row![].spacing(14);
                for (val, lab) in f.options.iter() {
                    let is = on.contains(&type_key(val));
                    let (k, o) = (key.clone(), val.to_string());
                    boxes = boxes.push(
                        checkbox(is)
                            .label(lab)
                            .size(16)
                            .text_size(13)
                            .spacing(6)
                            .on_toggle_maybe((!locked).then_some(move |x| Msg::Check(k.clone(), o.clone(), x))),
                    );
                }
                lines.push(widgets::small_muted(&*f.label).font(theme::semibold()).into());
                lines.push(boxes.wrap().into());
            }
            "select" => {
                let cur = self.current(values, &f.key);
                let k = key.clone();
                let picker: El<'a, Msg> = if locked {
                    help(f.options.iter().find(|(x, _)| *x == cur).map(|(_, l)| l).unwrap_or(cur.as_str()).to_string()).into()
                } else {
                    let table: Vec<(&str, &str)> = f.options.iter().collect();
                    select_table(&table, &cur, move |x| Msg::Edit(k.clone(), x))
                };
                lines.push(widgets::field(&*f.label, picker));
            }
            "textarea" => {
                lines.push(widgets::small_muted(&*f.label).font(theme::semibold()).into());
                if let Some(c) = self.areas.get(&*f.key) {
                    let k = key.clone();
                    let placeholder = if f.is_secret() && truthy(v) { "Saved – paste to replace" } else { "" };
                    lines.push(text_area(c, placeholder, 5, move |a| Msg::Area(k.clone(), a)));
                }
                if reveal && f.is_secret() && truthy(v) && !self.never_shown.contains(&*f.key) && !locked {
                    let shown = self.shown.contains(&*f.key);
                    lines.push(
                        row![
                            btn_sm(
                                if shown { "Hide" } else { "Show saved" },
                                Btn::Default,
                                Some(if shown { Msg::Hide(key.clone()) } else { Msg::Reveal(key.clone()) })
                            ),
                            btn_icon(Icon::Copy, "Copy", Btn::Default, Some(Msg::Copy(key.clone()))),
                        ]
                        .spacing(8)
                        .into(),
                    );
                }
            }
            "password" => {
                let text = self.edits.get(&*f.key).cloned().unwrap_or_default();
                let placeholder = if locked {
                    "set by environment"
                } else if truthy(v) {
                    "saved – paste a new value to replace"
                } else {
                    "not set"
                };
                let shown = self.shown.contains(&*f.key);
                let k = key.clone();
                let mut r = row![].spacing(6).align_y(Alignment::Center);
                if locked {
                    r = r.push(input_locked::<Msg>("").width(Length::Fill));
                } else {
                    r = r.push(input(placeholder, &text, move |x| Msg::Edit(k.clone(), x)).secure(!shown).width(Length::Fill));
                }
                if reveal && truthy(v) {
                    r = r.push(icon_btn(
                        if shown { Icon::X } else { Icon::Search },
                        if shown { "Hide" } else { "Show" },
                        Some(if shown { Msg::Hide(key.clone()) } else { Msg::Reveal(key.clone()) }),
                    ));
                    r = r.push(icon_btn(Icon::Copy, "Copy", Some(Msg::Copy(key.clone()))));
                }
                lines.push(widgets::field(&*f.label, r));
            }
            _ => {
                let text = self.current(values, &f.key);
                let k = key.clone();
                let b: El<'a, Msg> = if locked {
                    input_locked::<Msg>(&text).width(Length::Fill).into()
                } else {
                    input("", &text, move |x| Msg::Edit(k.clone(), x)).width(Length::Fill).into()
                };
                lines.push(widgets::field(&*f.label, b));
            }
        }
        if let Some(h) = &f.help {
            lines.push(help(&**h).into());
        }
        if let Some(n) = self.source_note(f, values) {
            lines.push(n);
        }
        column(lines).spacing(4).width(Length::Fill).into()
    }

    /// The fields two per row (`.form-grid`), the spanning ones on a row of their own.
    pub fn grid<'a>(&'a self, fields: &[&'a Field], reveal: bool) -> El<'a, Msg> {
        let mut rows = column![].spacing(14);
        let mut pending: Option<El<'a, Msg>> = None;
        for f in fields {
            let el = self.field_view(f, reveal);
            if f.spans() {
                if let Some(p) = pending.take() {
                    rows = rows.push(two(p, None));
                }
                rows = rows.push(el);
            } else if let Some(p) = pending.take() {
                rows = rows.push(two(p, Some(el)));
            } else {
                pending = Some(el);
            }
        }
        if let Some(p) = pending.take() {
            rows = rows.push(two(p, None));
        }
        rows.width(Length::Fill).into()
    }

    /// One card: the heading that folds it (with the chevron), the Test button, and when open the
    /// results, the fields and Save. `wrap` turns the form's messages into the app's; `slots` adds the
    /// app's own blocks (use `Slots::default()` for none).
    pub fn card_view<'a, M: Clone + 'a>(&'a self, card: &'a Card, reveal: bool, wrap: fn(Msg) -> M, slots: Slots<'a, M>) -> El<'a, M> {
        let p = theme::look();
        let open = self.open.contains(&*card.id);
        let busy = self.busy.contains(&*card.id);
        let mut head_words = column![widgets::h2(&*card.title)].spacing(4);
        if !card.desc.is_empty() {
            head_words = head_words.push(help(&*card.desc));
        }
        let header = button(
            row![
                head_words.width(Length::Fill),
                container(icon(if open { Icon::Chevron } else { Icon::More }, 18.0, p.muted)).padding([4, 0])
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        )
        .padding(0)
        .width(Length::Fill)
        .style(theme::btn(Btn::Row))
        .on_press(wrap(Msg::Toggle(card.id.to_string())));
        let mut head = row![header].spacing(10).align_y(Alignment::Start);
        if card.test.is_some() {
            let test: El<'a, Msg> =
                btn_icon(Icon::Refresh, card.test_words(), Btn::Default, (!busy).then(|| Msg::Test(card.id.to_string()))).into();
            head = head.push(test.map(wrap));
        }
        let mut body = column![head].spacing(14).width(Length::Fill);
        if open {
            for (ok, m) in self.flash(&card.id) {
                body = body.push(result_line_copy(*ok, m, Msg::CopyText).map(wrap));
            }
            for el in slots.top {
                body = body.push(el);
            }
            match slots.body {
                Some(el) => body = body.push(el),
                None => body = body.push(self.grid(&self.visible_fields(card), reveal).map(wrap)),
            }
            for el in slots.bottom {
                body = body.push(el);
            }
            let save: El<'a, Msg> =
                btn_icon(Icon::Check, if busy { "Saving…" } else { "Save" }, Btn::Primary, (!busy).then(|| Msg::Save(card.id.to_string())))
                    .into();
            body = body.push(container(save.map(wrap)).width(Length::Fill).align_x(Alignment::End));
        }
        widgets::card(body).width(Length::Fill).into()
    }
}

/// The heading over a run of cards (a card's `part`): small, bold, faint.
pub fn part_heading<'a, M: 'a>(label: &'a str) -> El<'a, M> {
    container(t(label).size(11).color(theme::look().faint).font(theme::bold())).padding(widgets::pad(10.0, 4.0, 0.0, 4.0)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const EVENTS: &[(&str, &str)] =
        &[("Sign-up", "Sign-up"), ("Login", "Login"), ("Backup", "Backup"), ("Error", "Error"), ("Other", "Other")];
    const EVENT_EXTRAS: &[(&str, &str)] = &[("Daily", "Daily"), ("Test", "Test"), ("Retry", "Retry"), ("Auto-renew", "Auto-renew")];

    const BACKUP: &[Field] = &[field("backup_ssh_key", "SSH key", "textarea").full().secret()];

    /// A few cards of an imaginary app, Demo, as its table writes them.
    const SECTIONS: &[Card] = &[
        card! { id: "general", title: "General", desc: "Your server's name and address, who can sign up, and how sign-ups are approved.", test: None, test_label: None, fields: &[
            field("app_name", "App name", "text"),
            field("base_url", "Public URL", "text").help("e.g. https://demo.example.com – used in links & notifications"),
            field("signup_mode", "Sign-up mode", "select").options(&[("invite", "Invite only"), ("open", "Open to anyone"), ("closed", "Closed")]),
            field("auto_approve", "Auto-approve all sign-ups", "bool"),
            field("session_days", "Stay signed in for (days)", "number"),
            field("rate_limit", "Requests / second", "number").step("0.1"),
            field("api_token", "API token (optional)", "password").full(),
        ] },
        card! { id: "storage", title: "Storage", desc: "", test: None, test_label: None, fields: &[
            field("data_path", "Data folder", "text"),
            field("extra_folders", "Extra folders", "textarea").full(),
            field("cache_path", "Cache folder", "text"),
        ] },
        card! { id: "notify", title: "Notifications", desc: "", test: None, test_label: None, fields: &[
            field("notify_events", "Events to mail about", "checks").full().options(EVENTS),
            field("notify_skip", "Skip events that are", "checks").full().options(EVENT_EXTRAS),
        ] },
        card! { id: "email", title: "Email", desc: "", test: Some("email"), test_label: None, fields: &[
            field("smtp_security", "Security", "select").options(&[("starttls", "STARTTLS"), ("tls", "TLS"), ("none", "None")]),
            field("starttls_port", "STARTTLS port", "text").when("smtp_security", &["starttls"]),
            field("smtp_password", "SMTP password", "password").when("smtp_security", &["starttls"]),
            field("tls_port", "TLS port", "text").when("smtp_security", &["tls"]),
            field("other_port", "Port", "text").when_not("smtp_security", &["starttls"]),
        ] },
        Card::new("backup", "Off-site backup", "", Some("backup"), Some("Test backup"), BACKUP),
    ];

    fn card(id: &str) -> &'static Card {
        SECTIONS.iter().find(|c| c.id == id).unwrap()
    }

    fn get(c: &'static Card, key: &str) -> &'static Field {
        c.fields.iter().find(|f| f.key == key).unwrap()
    }

    #[test]
    fn the_table_builds_from_const_fields() {
        let general = card("general");
        assert_eq!(general.fields[0].key, "app_name");
        assert_eq!(general.fields.last().unwrap().kind, "password");
        assert!(general.fields.last().unwrap().full);
        assert_eq!(get(general, "rate_limit").step.as_deref(), Some("0.1"));
        assert_eq!(get(card("email"), "starttls_port").when, Some(When::is("smtp_security", &["starttls"])));
        let key = get(card("backup"), "backup_ssh_key");
        assert!(key.is_secret() && key.spans() && key.kind == "textarea");
        assert!(get(general, "api_token").is_secret());
        assert!(!get(general, "app_name").spans());
        assert!(get(card("notify"), "notify_events").spans());
        assert_eq!(card("email").test_words(), TEST_CONNECTION);
        assert_eq!(card("backup").test_words(), "Test backup");
        assert_eq!(type_key("Read/Write"), "readwrite");
        assert_eq!(type_key("Auto-renew"), "autorenew");
    }

    #[test]
    fn a_plain_text_area_keeps_what_is_saved() {
        let values = json!({ "data_path": "/data", "extra_folders": "/mnt/a\n/mnt/b", "cache_path": "/c" });
        let card = card("storage");
        let mut st = Form::new();
        st.take_values(values.clone(), SECTIONS);
        assert_eq!(st.save_body(&values, card)["extra_folders"], "/mnt/a\n/mnt/b", "an untouched Save keeps the folders");
        // a value changed on the server (by another admin) shows when the values come again
        let newer = json!({ "data_path": "/data", "extra_folders": "/mnt/a\n/mnt/b\n/mnt/c", "cache_path": "/c" });
        st.take_values(newer.clone(), SECTIONS);
        assert_eq!(st.save_body(&newer, card)["extra_folders"], "/mnt/a\n/mnt/b\n/mnt/c");
        // what was typed stays until it is saved
        st.areas.insert("extra_folders".into(), text_editor::Content::with_text("/x"));
        st.typed.insert("extra_folders".into());
        st.take_values(newer.clone(), SECTIONS);
        assert_eq!(st.save_body(&newer, card)["extra_folders"], "/x");
        st.values = Some(newer.clone());
        st.settle(card);
        assert_eq!(st.areas["extra_folders"].text().trim(), "/mnt/a\n/mnt/b\n/mnt/c");
        // a secret one still starts empty and is left out until something is pasted
        let key = json!({ "backup_ssh_key": "•••• (saved)" });
        let mut st = Form::new();
        st.take_values(key.clone(), SECTIONS);
        assert!(st.save_body(&key, super::tests::card("backup")).get("backup_ssh_key").is_none());
        st.areas.insert("backup_ssh_key".into(), text_editor::Content::with_text(" -----BEGIN KEY----- "));
        assert_eq!(st.save_body(&key, super::tests::card("backup"))["backup_ssh_key"], "-----BEGIN KEY-----");
    }

    #[test]
    fn the_save_body_follows_the_web_rules() {
        let values = json!({
            "app_name": "Demo", "base_url": "http://x", "auto_approve": 1, "api_token": "•••• (saved)", "signup_mode": "invite",
            "rate_limit": 1.5,
            "_meta": { "base_url": { "source": "env", "env": "DEMO_BASE_URL" } }
        });
        let mut st = Form::new();
        st.edits.insert("app_name".into(), "Mine".into());
        st.bools.insert("auto_approve".into(), false);
        let card = card("general");
        let b = st.save_body(&values, card);
        assert_eq!(b["app_name"], "Mine");
        assert_eq!(b["auto_approve"], false);
        assert_eq!(b["signup_mode"], "invite");
        assert_eq!(b["rate_limit"], "1.5", "numbers go as the text the input shows");
        assert!(b.get("base_url").is_none(), "locked by the environment");
        assert!(b.get("api_token").is_none(), "nothing typed keeps the saved secret");
        assert!(b.get("session_days").is_none(), "the server does not have it");
        st.edits.insert("api_token".into(), " tok ".into());
        assert_eq!(st.save_body(&values, card)["api_token"], "tok");
        // a secret shown from the server is not sent back
        st.revealed.insert("api_token".into());
        assert!(st.save_body(&values, card).get("api_token").is_none());
        // checks join the ticked options in the table's order and spelling
        let values = json!({ "notify_events": "signup,login", "notify_skip": "" });
        let mut st = Form::new();
        st.checks.insert("notify_skip".into(), vec!["autorenew".into(), "test".into()]);
        let b = st.save_body(&values, super::tests::card("notify"));
        assert_eq!(b["notify_events"], "Sign-up,Login");
        assert_eq!(b["notify_skip"], "Test,Auto-renew");
    }

    #[test]
    fn when_rules_follow_the_select() {
        let values = json!({ "smtp_security": "starttls", "starttls_port": "", "tls_port": "", "other_port": "" });
        let mut st = Form::new();
        let t = card("email");
        let (starttls, tls, not) = (get(t, "starttls_port"), get(t, "tls_port"), get(t, "other_port"));
        assert!(st.visible(&values, starttls) && !st.visible(&values, tls) && !st.visible(&values, not));
        st.edits.insert("smtp_security".into(), "tls".into());
        assert!(!st.visible(&values, starttls) && st.visible(&values, tls) && st.visible(&values, not));
        // only the fields the server has and whose rule holds are drawn
        st.values = Some(values);
        let keys: Vec<&str> = st.visible_fields(t).iter().map(|f| &*f.key).collect();
        assert_eq!(keys, ["smtp_security", "tls_port", "other_port"]);
        assert_eq!(
            When::parse("smtp_security=tls"),
            Some(When { key: "smtp_security".into(), values: Words::Owned(vec!["tls".into()]), not: false })
        );
        assert!(When::parse("smtp_security!=none").is_some_and(|w| w.not && w.holds("tls") && !w.holds("none")));
        assert_eq!(When::parse("no rule"), None);
        assert_eq!(test_words(&json!({ "ok": true })), (true, "Connected".to_string()));
        assert_eq!(test_words(&json!({ "ok": false })), (false, "Failed".to_string()));
        assert_eq!(test_words(&json!({ "ok": false, "message": "no" })), (false, "no".to_string()));
    }

    #[test]
    fn the_first_card_the_server_has_starts_open() {
        let mut st = Form::new();
        st.open(["general", "notify"], None);
        st.update(Msg::Values(Ok(json!({ "notify_events": "Sign-up" }))), SECTIONS);
        assert!(st.is_open("notify") && !st.is_open("general"));
        // or the one a link asked for
        st.open(["general", "notify"], Some("general".into()));
        st.update(Msg::Values(Ok(json!({ "notify_events": "Sign-up" }))), SECTIONS);
        assert!(st.is_open("general") && !st.is_open("notify"));
        st.update(Msg::Toggle("general".into()), SECTIONS);
        assert!(!st.is_open("general"));
        st.update(Msg::Values(Err("Server unreachable".into())), SECTIONS);
        assert_eq!(st.error(), "Server unreachable");
    }

    #[test]
    fn the_update_says_what_the_app_does() {
        let values = json!({ "app_name": "Demo", "api_token": "•••• (saved)", "smtp_security": "starttls", "starttls_port": "", "smtp_password": "" });
        let mut st = Form::new();
        st.open(["general"], None);
        st.update(Msg::Values(Ok(values)), SECTIONS);
        st.update(Msg::Edit("app_name".into(), "Mine".into()), SECTIONS);
        let out = st.update(Msg::Save("general".into()), SECTIONS);
        let mut body = Map::new();
        body.insert("app_name".into(), json!("Mine"));
        assert_eq!(out, [Out::Save { card: "general".into(), body }]);
        assert!(st.is_busy("general"));
        let out = st.update(
            Msg::Saved("general".into(), Ok(json!({ "app_name": "Mine", "_warnings": [{ "message": "Restart needed" }] }))),
            SECTIONS,
        );
        assert_eq!(out, [Out::ToastOk("Settings saved".into())]);
        assert!(!st.is_busy("general"));
        assert_eq!(st.flash("general"), [(false, "Restart needed".to_string())]);
        assert!(st.values().unwrap().get("_warnings").is_none());
        assert_eq!(st.current(st.values().unwrap(), "app_name"), "Mine", "the typed value settled");
        assert_eq!(st.update(Msg::Saved("general".into(), Err("no".into())), SECTIONS), [Out::ToastErr("no".into())]);
        // a test saves first, then runs
        st.update(Msg::Values(Ok(json!({ "smtp_security": "starttls", "starttls_port": "587" }))), SECTIONS);
        let out = st.update(Msg::Test("email".into()), SECTIONS);
        assert!(
            matches!(&out[..], [Out::Test { card, test, body }] if card == "email" && test == "email" && body["starttls_port"] == "587")
        );
        assert!(st.is_open("email") && st.is_busy("email"));
        assert_eq!(st.flash("email"), [(true, "Testing…".to_string())]);
        st.update(Msg::Tested("email".into(), Ok((json!({ "smtp_security": "starttls" }), Err(String::new())))), SECTIONS);
        assert_eq!(st.flash("email"), [(false, "Test failed".to_string())]);
        st.update(
            Msg::Tested(
                "email".into(),
                Ok((json!({ "smtp_security": "starttls" }), Ok(json!({ "ok": true, "message": "SMTP 220 ready" })))),
            ),
            SECTIONS,
        );
        assert_eq!(st.flash("email"), [(true, "SMTP 220 ready".to_string())]);
        st.update(Msg::Tested("email".into(), Err("Not saved".into())), SECTIONS);
        assert_eq!(st.flash("email"), [(false, "Not saved".to_string())]);
        assert!(st.update(Msg::Test("general".into()), SECTIONS).is_empty(), "a card with no test");
        assert!(st.update(Msg::Save("nowhere".into()), SECTIONS).is_empty());
        // secrets: shown from what was typed, else fetched
        assert_eq!(st.update(Msg::Reveal("api_token".into()), SECTIONS), [Out::Reveal { key: "api_token".into() }]);
        st.update(Msg::Revealed("api_token".into(), Ok(json!({ "value": "tok_1" }))), SECTIONS);
        assert!(st.shown.contains("api_token") && st.save_body(st.values().unwrap(), card("general")).get("api_token").is_none());
        st.update(Msg::Hide("api_token".into()), SECTIONS);
        assert!(!st.edits.contains_key("api_token"), "a fetched secret is forgotten when hidden");
        st.update(Msg::Edit("smtp_password".into(), "pw".into()), SECTIONS);
        assert!(st.update(Msg::Reveal("smtp_password".into()), SECTIONS).is_empty());
        assert_eq!(
            st.update(Msg::Copy("smtp_password".into()), SECTIONS),
            [Out::Copy("pw".into()), Out::ToastOk("Copied to clipboard".into())]
        );
        assert_eq!(st.update(Msg::Copy("api_token".into()), SECTIONS), [Out::CopySecret { key: "api_token".into() }]);
        assert_eq!(
            st.update(Msg::Copied(Ok(json!({ "value": "tok_1" }))), SECTIONS),
            [Out::Copy("tok_1".into()), Out::ToastOk("Copied to clipboard".into())]
        );
        assert_eq!(st.update(Msg::Revealed("x".into(), Err("Owner only".into())), SECTIONS), [Out::ToastErr("Owner only".into())]);
        let mut body = Map::new();
        body.insert("api_token".into(), json!(""));
        assert_eq!(st.update(Msg::Clear("api_token".into()), SECTIONS), [Out::Clear { key: "api_token".into(), body }]);
        assert_eq!(st.update(Msg::Cleared(Ok(json!({ "api_token": "" }))), SECTIONS), [Out::ToastOk("Cleared".into())]);
        assert_eq!(st.update(Msg::CopyText("why".into()), SECTIONS), [Out::CopyText("why".into())]);
        // check boxes keep the ticked keys
        st.update(Msg::Values(Ok(json!({ "notify_events": "Sign-up" }))), SECTIONS);
        st.update(Msg::Check("notify_events".into(), "Login".into(), true), SECTIONS);
        st.update(Msg::Check("notify_events".into(), "Sign-up".into(), false), SECTIONS);
        assert_eq!(st.checked(st.values().unwrap(), "notify_events"), ["login"]);
    }

    #[test]
    fn the_schema_answer_reads() {
        let answer = json!({ "sections": [
            { "id": "general", "title": "General", "aliases": [], "groups": [
                { "id": "general", "title": "General", "part": "", "desc": "Your server.", "test": null, "test_label": null, "fields": [
                    { "key": "app_name", "label": "App name", "help": null, "type": "text", "options": [], "when": null, "secret": false,
                      "env": "DEMO_APP_NAME", "env_locked": false, "default": "Demo", "unit": null, "min": null, "max": null, "step": null, "full": false, "advanced": false },
                    { "key": "rate_limit", "label": "Requests / second", "help": "Keep at 1", "type": "number", "options": [], "when": null,
                      "secret": false, "step": 0.1, "full": false }
                ] }
            ] },
            { "id": "notifications", "title": "Notifications", "aliases": ["mail"], "groups": [
                { "id": "email", "title": "Email", "part": "", "desc": "", "test": "email", "test_label": "Test connection", "fields": [
                    { "key": "email_format", "label": "Format", "type": "select", "options": [{ "value": "1", "label": "Plain text" }, { "value": "2", "label": "HTML – with images" }] },
                    { "key": "smtp_password", "label": "SMTP password", "help": null, "type": "password", "options": [], "when": "smtp_security=starttls", "secret": true, "full": false }
                ] }
            ] },
            { "id": "blocked", "title": "Blocked", "aliases": [], "groups": [] }
        ] });
        let schema = Schema::from_json(&answer).unwrap();
        assert_eq!(schema.sections.len(), 3);
        assert_eq!(schema.find("mail").unwrap().id, "notifications");
        assert_eq!(schema.find("email").unwrap().id, "notifications");
        assert_eq!(schema.find("general").unwrap().id, "general");
        assert!(schema.find("nowhere").is_none());
        assert!(schema.sections[2].cards.is_empty());
        let rate = &schema.card("general").unwrap().fields[1];
        assert_eq!((rate.step.as_deref(), rate.help.as_deref()), (Some("0.1"), Some("Keep at 1")));
        let t = schema.card("email").unwrap();
        assert_eq!(t.test.as_deref(), Some("email"));
        assert_eq!(t.test_words(), "Test connection");
        assert_eq!(t.fields[0].options.iter().collect::<Vec<_>>(), [("1", "Plain text"), ("2", "HTML – with images")]);
        let pw = &t.fields[1];
        assert!(pw.is_secret() && pw.when == When::parse("smtp_security=starttls"));
        assert_eq!(schema.sections[1].parts().len(), 1);
        assert_eq!(schema.cards().count(), 2);
        // the form runs on the schema's cards as on a table
        let mut st = Form::new();
        st.open(schema.sections[1].cards.iter().map(|c| c.id.to_string()), None);
        st.update(Msg::Values(Ok(json!({ "email_format": 2, "smtp_password": "" }))), schema.cards());
        assert!(st.is_open("email"));
        let out = st.update(Msg::Save("email".into()), schema.cards());
        assert!(matches!(&out[..], [Out::Save { body, .. }] if body["email_format"] == "2" && !body.contains_key("smtp_password")));
        // what does not fit says so
        assert_eq!(Schema::from_json(&json!({})), Err("The settings schema has no sections".into()));
        let bad = json!({ "sections": [{ "id": "x", "groups": [{ "id": "c", "fields": [{ "key": "k", "label": "K", "type": "text", "when": "nope" }] }] }] });
        assert_eq!(Schema::from_json(&bad), Err("The field k has a rule that does not parse: nope".into()));
    }

    #[test]
    fn notes_and_values_read_as_the_server_sends_them() {
        let values = json!({ "_meta": { "base_url": { "source": "env", "env": "DEMO_BASE_URL" } } });
        assert!(env_locked(&values, "base_url") && !env_locked(&values, "app_name"));
        assert_eq!(
            env_note(&values, "base_url").as_deref(),
            Some("🔒 Set by DEMO_BASE_URL in your compose/env – change it there (environment always wins)")
        );
        assert!(is_masked("•••• (saved)") && !is_masked("tok"));
        assert_eq!(vs(&json!(2)), "2");
        assert_eq!(vs(&json!(0.05)), "0.05");
        assert_eq!(vs(&Value::Null), "");
        assert!(truthy(&json!(1)) && !truthy(&json!("")) && !truthy(&json!(0)));
    }

    #[test]
    fn the_card_draws() {
        let mut st = Form::new().never_shown(["backup_ssh_key"]);
        st.open(["general", "backup"], None);
        st.update(Msg::Values(Ok(json!({ "app_name": "Demo", "signup_mode": "open", "auto_approve": true, "api_token": "•••• (saved)", "backup_ssh_key": "•••• (saved)" }))), SECTIONS);
        st.update(Msg::Toggle("backup".into()), SECTIONS);
        for c in SECTIONS {
            let _: El<'_, Msg> = st.card_view(c, true, std::convert::identity, Slots::default());
        }
        let _: El<'_, Msg> = part_heading("Services");
    }
}
