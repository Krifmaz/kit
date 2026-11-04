//! The settings cards drawn from the server's schema, so a new setting needs no UI work.
//!
//! [`SettingsCards`] draws a page's cards and wires Save (`PUT /api/settings`), Test (`POST
//! /api/diagnostics/test/<test>` after a quiet save), clear and the eye and copy buttons
//! (`GET /api/settings/secret/<key>`); [`SchemaFields`] is the fields alone. The paths are the configured ones
//! ([`Paths`](crate::config::Paths)), the words the configured [`FormWords`].

pub mod schema;

use crate::api::{self, ApiError};
use crate::clipboard::copy_text;
use crate::details::CopyButton;
use crate::html::{html, opt, seq};
use crate::icons::{chevron, icon, EYE, EYE_OFF};
use crate::json::{js_string, truthy, J};
use crate::toasts::toast;
use leptos::prelude::*;
use schema::{type_key, Field, Group};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::JsCast;
use web_sys::Element;

pub use schema::{visible_cards, Schema, Section, When};

/// A small neutral settings schema (General with an App and a Storage card, Mail with a card that has Test),
/// for demos and tests: every field type, a `when` rule each way, an advanced field and a secret text area.
pub const SAMPLE_SCHEMA: &str = include_str!("sample-schema.json");

/// What `GET /api/settings` sends for a saved secret ("•••• (saved)").
pub const MASK: &str = "••••";
/// The fold of the advanced fields.
pub const SHOW_ADVANCED: &str = "Show advanced";
/// The card's Save.
pub const SAVE: &str = "Save";
/// The toast after Save.
pub const SAVED: &str = "Settings saved";
/// The toast after a secret's clear.
pub const CLEARED: &str = "Cleared";
/// The Test button when the card names no words.
pub const TEST_CONNECTION: &str = "Test connection";
/// The line while Test runs.
pub const TESTING: &str = "Testing…";
/// Test's result when it failed without a sentence.
pub const TEST_FAILED: &str = "Test failed";
/// A test result that passed without a sentence.
pub const CONNECTED: &str = "Connected";
/// A test result that failed without a sentence.
pub const FAILED: &str = "Failed";
/// A secret text area's button, and the eye's title.
pub const SHOW_SAVED: &str = "Show saved";
/// The eye's title while hidden.
pub const SHOW: &str = "Show";
/// The eye's title while shown.
pub const HIDE: &str = "Hide";
/// The copy buttons of a secret.
pub const COPY: &str = kit_rules::toasts::COPY;
/// The line under a value an environment variable sets, `{env}` its name.
pub const ENV_NOTE: &str = "🔒 Set by {env} in the environment – change it there (the environment always wins)";
/// The line under a saved secret, before its clear link.
pub const SAVED_HIDDEN: &str = "Saved ✓ (hidden)";
/// A saved secret's clear link.
pub const CLEAR: &str = "clear";
/// The line under a secret that is not set.
pub const NOT_SET: &str = "Not set";
/// After [`NOT_SET`] when an environment variable could set it, `{env}` its name.
pub const OR_SET: &str = " – or set {env} in the environment";
/// The line under a value another part of the app fills in, `{plugin}` its name.
pub const PLUGIN_NOTE: &str = "🔗 Filled in by {plugin} – it changes this itself";
/// The line under a default an environment variable could set, `{env}` its name.
pub const DEFAULT_ENV: &str = "Default · env var {env}";
/// A saved secret text area's placeholder.
pub const PASTE_TO_REPLACE: &str = "Saved – paste to replace";
/// A password field's placeholder while an environment variable sets it.
pub const SET_BY_ENV: &str = "set by environment";
/// A saved password's placeholder.
pub const PASTE_NEW: &str = "saved – paste a new value to replace";
/// A password's placeholder while none is saved.
pub const NOT_SET_PLACEHOLDER: &str = "not set";

/// The settings form's words. [`FormWords::DEFAULT`] holds the kit's; an app passes its own in
/// [`Words::form`](crate::config::Words::form).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormWords {
    /// The fold with a card's advanced fields ([`SHOW_ADVANCED`]).
    pub show_advanced: &'static str,
    /// A card's save button ([`SAVE`]).
    pub save: &'static str,
    /// The toast once a card was saved ([`SAVED`]).
    pub saved: &'static str,
    /// The toast once a saved value was cleared ([`CLEARED`]).
    pub cleared: &'static str,
    /// A card's test button when the card names none ([`TEST_CONNECTION`]).
    pub test_connection: &'static str,
    /// The line while the test runs ([`TESTING`]).
    pub testing: &'static str,
    /// The line when the test call itself failed ([`TEST_FAILED`]).
    pub test_failed: &'static str,
    /// A test that passed without a sentence ([`CONNECTED`]).
    pub connected: &'static str,
    /// A test that failed without a sentence ([`FAILED`]).
    pub failed: &'static str,
    /// The button that shows a saved secret ([`SHOW_SAVED`]).
    pub show_saved: &'static str,
    /// The eye while hidden ([`SHOW`]).
    pub show: &'static str,
    /// The eye while shown ([`HIDE`]).
    pub hide: &'static str,
    /// The button that copies a secret ([`COPY`]).
    pub copy: &'static str,
    /// The line under a value an environment variable sets, with `{env}` ([`ENV_NOTE`]).
    pub env_note: &'static str,
    /// The line under a saved secret ([`SAVED_HIDDEN`]).
    pub saved_hidden: &'static str,
    /// Its clear link ([`CLEAR`]).
    pub clear: &'static str,
    /// The line under a secret that is not set ([`NOT_SET`]).
    pub not_set: &'static str,
    /// After it when an environment variable could set it, with `{env}` ([`OR_SET`]).
    pub or_set: &'static str,
    /// The line under a value another part of the app fills in, with `{plugin}` ([`PLUGIN_NOTE`]).
    pub plugin_note: &'static str,
    /// The line under a default an environment variable could set, with `{env}` ([`DEFAULT_ENV`]).
    pub default_env: &'static str,
    /// A saved secret text area's placeholder ([`PASTE_TO_REPLACE`]).
    pub paste_to_replace: &'static str,
    /// A password's placeholder while an environment variable sets it ([`SET_BY_ENV`]).
    pub set_by_env: &'static str,
    /// A saved password's placeholder ([`PASTE_NEW`]).
    pub paste_new: &'static str,
    /// A password's placeholder while none is saved ([`NOT_SET_PLACEHOLDER`]).
    pub not_set_placeholder: &'static str,
}

impl FormWords {
    /// The kit's words.
    pub const DEFAULT: FormWords = FormWords {
        show_advanced: SHOW_ADVANCED,
        save: SAVE,
        saved: SAVED,
        cleared: CLEARED,
        test_connection: TEST_CONNECTION,
        testing: TESTING,
        test_failed: TEST_FAILED,
        connected: CONNECTED,
        failed: FAILED,
        show_saved: SHOW_SAVED,
        show: SHOW,
        hide: HIDE,
        copy: COPY,
        env_note: ENV_NOTE,
        saved_hidden: SAVED_HIDDEN,
        clear: CLEAR,
        not_set: NOT_SET,
        or_set: OR_SET,
        plugin_note: PLUGIN_NOTE,
        default_env: DEFAULT_ENV,
        paste_to_replace: PASTE_TO_REPLACE,
        set_by_env: SET_BY_ENV,
        paste_new: PASTE_NEW,
        not_set_placeholder: NOT_SET_PLACEHOLDER,
    };
}

impl Default for FormWords {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn words() -> FormWords {
    crate::config::get().words.form
}

/// `template` split at `{name}`: the words before and after it (all of it before when it has none).
fn around<'a>(template: &'a str, name: &str) -> (&'a str, &'a str) {
    template.split_once(name).unwrap_or((template, ""))
}

/// `template` with its `{name}` drawn by `inner`.
fn with_part(template: &'static str, name: &str, inner: AnyView) -> AnyView {
    let (a, b) = around(template, name);
    view! { {a}{inner}{b} }.into_any()
}

fn is_masked(v: &str) -> bool {
    v.starts_with(MASK)
}

/// Whether an environment variable sets `key` (`meta`: the `_meta` of `GET /api/settings`): the value is
/// shown locked and Save leaves it out.
pub fn env_locked(meta: &Value, key: &str) -> bool {
    meta.o(key).s("source") == "env"
}

/// The note under a value an environment variable sets ("🔒 Set by APP_NAME in the environment – change it there
/// (the environment always wins)").
#[component]
pub fn EnvNote(
    /// The variable's name.
    #[prop(into)]
    env: String,
) -> impl IntoView {
    let w = words();
    view! { <small class="src env">{with_part(w.env_note, "{env}", view! { <span class="mono">{env}</span> }.into_any())}</small> }
}

/// The note under a field: set by an environment variable, a saved secret (with clear), a secret that is not
/// set, a value another part of the app fills in, or the environment variable that would set it.
fn source_note(f: &Field, v: &Value, meta: &Value) -> AnyView {
    let w = words();
    let m = meta.o(&f.key);
    let secret = f.is_secret();
    let mono = |s: String| view! { <span class="mono">{s}</span> }.into_any();
    if env_locked(meta, &f.key) {
        return view! { <EnvNote env=m.s("env")/> }.into_any();
    }
    if secret && truthy(v) {
        return view! { <small class="src">{w.saved_hidden}" · "<a href="#" data-clear=f.key.clone()>{w.clear}</a></small> }.into_any();
    }
    if secret {
        let env = m.truthy("env").then(|| with_part(w.or_set, "{env}", mono(m.s("env"))));
        return view! { <small class="src">{w.not_set}{opt(env)}</small> }.into_any();
    }
    if m.s("source") == "plugin" {
        let by = view! { <b>{m.s("plugin")}</b> }.into_any();
        return view! { <small class="src plugin">{with_part(w.plugin_note, "{plugin}", by)}</small> }.into_any();
    }
    if m.s("source") == "ui" {
        return "".into_any();
    }
    opt(m.truthy("env").then(|| view! { <small class="src faint">{with_part(w.default_env, "{env}", mono(m.s("env")))}</small> }))
}

/// The values of the `when` selects of a run of fields, as they change.
pub type Selects = ArcRwSignal<HashMap<String, String>>;

/// One field: `v` its value from `GET /api/settings`, `meta` that answer's `_meta`, `reveal` whether the
/// server lets admins see saved secrets.
pub fn field_view(f: &Field, v: &Value, meta: &Value, reveal: bool, selects: Selects) -> AnyView {
    let w = words();
    let key = f.key.clone();
    let locked = meta.o(&f.key).s("source") == "env";
    let readonly = !locked && meta.o(&f.key).s("source") == "plugin";
    let help = (opt(f.help.clone().map(|h| view! { <small>{h}</small> })), source_note(f, v, meta));
    let cls = if f.full || f.kind == "textarea" { "field full" } else { "field" };
    let label = f.label.clone();
    // the `when` rule: shown while its select has the value (bind_show_when)
    let rule = f.when.clone();
    let (when_key, when_in, when_not) = match &rule {
        Some(w) if w.equal => (Some(w.key.clone()), Some(w.value.clone()), None),
        Some(w) => (Some(w.key.clone()), None, Some(w.value.clone())),
        None => (None, None, None),
    };
    let watched = selects.clone();
    let hidden = move || rule.as_ref().is_some_and(|w| !w.shows(watched.get().get(&w.key).map(String::as_str).unwrap_or("")));
    match f.kind.as_str() {
        "bool" => view! {
            <div {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls style="justify-content:center">
                <label class="switch"><input type="checkbox" data-k=key checked=truthy(v) disabled=locked readonly=readonly/><i>{""}</i>{label}</label>{help}
            </div>
        }
        .into_any(),
        "checks" => {
            let vs = if v.is_null() { String::new() } else { js_string(v) };
            let on: Vec<String> = vs.split(',').map(type_key).collect();
            let boxes = seq(f.options.iter().map(|(val, lab)| {
                view! { <label {..()} class="row" style="gap:6px;align-items:center"><input type="checkbox" value=val.clone() checked=on.contains(&type_key(val)) disabled=locked readonly=readonly/>" "{lab.clone()}</label> }
            }));
            view! {
                <div {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls data-k=key>
                    <span>{label}</span><div {..()} class="row wrap" style="gap:6px 14px">{boxes}</div>{help}
                </div>
            }
            .into_any()
        }
        "select" => {
            // option values are text; a numeric setting (quality = 2) compares as text
            let vs = js_string(v);
            let opts = seq(f.options.iter().map(|(val, lab)| view! { <option value=val.clone() selected=*val == vs>{lab.clone()}</option> }));
            let k2 = key.clone();
            let sel = selects.clone();
            let select = view! {
                <select class="input" data-k=key.clone() id=format!("f-{key}") disabled=locked
                    on:change=move |e| { let v = event_target_value(&e); sel.update(|m| { m.insert(k2.clone(), v); }); }>{opts}</select>
            }
            .attr("readonly", readonly.then_some(""));
            view! {
                <label {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls>
                    <span>{label}</span>{select}{help}
                </label>
            }
            .into_any()
        }
        _ => {
            // also for values set by environment variables (read-only, but viewable)
            let reveal = reveal && truthy(v);
            if f.kind == "textarea" {
                let btns = reveal.then(|| {
                    view! {
                        <span {..()} class="row" style="gap:8px">
                            <button {..()} type="button" class="btn sm" data-reveal=key.clone() {..html(format!("{EYE}{}", w.show_saved))}>{""}</button>
                            <button {..()} type="button" class="btn sm" data-copy=key.clone() {..html(format!("{}{}", icon("copy"), w.copy))}>{""}</button>
                        </span>
                    }
                });
                let ph = if truthy(v) { w.paste_to_replace } else { "" };
                return view! {
                    <label {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls>
                        <span>{label}</span><textarea class="input mono" data-k=key.clone() disabled=locked readonly=readonly placeholder=ph>{""}</textarea>{opt(btns)}{help}
                    </label>
                }
                .into_any();
            }
            if f.kind == "password" {
                // secrets are only sent to the browser when the eye or copy button is pressed: an empty input keeps the saved value
                let ph = if locked {
                    w.set_by_env
                } else if truthy(v) {
                    w.paste_new
                } else {
                    w.not_set_placeholder
                };
                let input = view! {
                    <input class="input" type="password" data-k=key.clone() value="" disabled=locked readonly=readonly data-eye=reveal.then_some("1") autocomplete="new-password" placeholder=ph/>
                };
                let wrapped = if reveal {
                    view! {
                        <span class="pw-wrap secret">{input}
                            <button {..()} type="button" class="eye-btn" data-reveal=key.clone() title=w.show aria-label={w.show} {..html(EYE)}>{""}</button>
                            <button {..()} type="button" class="eye-btn" data-copy=key.clone() title=w.copy aria-label={w.copy} {..html(icon("copy"))}>{""}</button>
                        </span>
                    }
                    .into_any()
                } else {
                    input.into_any()
                };
                return view! { <label {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls><span>{label}</span>{wrapped}{help}</label> }
                    .into_any();
            }
            // text, path, url and number (and any type this client does not know yet)
            let kind = if f.kind == "number" { "number" } else { "text" };
            view! {
                <label {..()} data-when=when_key data-when-in=when_in data-when-not=when_not hidden=hidden class=cls>
                    <span>{label}</span><input class="input" type=kind data-k=key step=f.step.clone() value=js_string(v) autocomplete="off" disabled=locked readonly=readonly/>{help}
                </label>
            }
            .into_any()
        }
    }
}

/// Draws a run of fields as a block of the app's own (a box with a heading around the `proxy_` fields).
pub type Block = Arc<dyn Fn(AnyView) -> AnyView + Send + Sync>;

/// A block of fields: those whose key starts with `prefix` are drawn by `draw`, in place of the first of them,
/// and the fields after them in a grid of their own (`form-grid mt`).
#[derive(Clone)]
pub struct FieldBlock {
    /// The key prefix of the block's fields (`proxy_`).
    pub prefix: String,
    /// Draws the block around its fields' grid cells.
    pub draw: Block,
}

/// The fields of a card with their values: a grid, and the advanced ones folded under "Show advanced".
#[component]
pub fn SchemaFields(
    /// The fields to draw (those the values hold, [`visible_cards`]).
    fields: Vec<Field>,
    /// `GET /api/settings` (with `_meta`).
    values: Value,
    /// The server lets admins see saved secrets.
    #[prop(optional)]
    reveal: bool,
    /// A run of fields the app draws as a block of its own.
    #[prop(optional)]
    block: Option<FieldBlock>,
) -> impl IntoView {
    let meta = values.o("_meta");
    let mut start = HashMap::new();
    for f in &fields {
        if let Some(w) = &f.when {
            start.insert(w.key.clone(), js_string(values.g(&w.key)));
        }
    }
    let selects: Selects = ArcRwSignal::new(start);
    let (adv, plain): (Vec<&Field>, Vec<&Field>) = fields.iter().partition(|f| f.advanced);
    let one = |f: &Field| field_view(f, values.g(&f.key), &meta, reveal, selects.clone());
    let in_block = |f: &Field| block.as_ref().is_some_and(|b| f.key.starts_with(&b.prefix));
    let grid = match (plain.iter().position(|f| in_block(f)), &block) {
        (Some(at), Some(b)) => {
            let rest: Vec<&Field> = plain.iter().copied().filter(|f| !in_block(f)).collect();
            let at = at.min(rest.len());
            let inner = seq(plain.iter().filter(|f| in_block(f)).map(|f| one(f)));
            let after = (rest.len() > at).then(|| view! { <div class="form-grid mt">{seq(rest[at..].iter().map(|f| one(f)))}</div> });
            view! { <div class="form-grid">{seq(rest[..at].iter().map(|f| one(f)))}</div>{(b.draw)(inner)}{opt(after)} }.into_any()
        }
        _ => view! { <div class="form-grid">{seq(plain.iter().map(|f| one(f)))}</div> }.into_any(),
    };
    let folded = (!adv.is_empty()).then(|| {
        let inner = seq(adv.iter().map(|f| one(f)));
        view! { <details class="adv-fields mt"><summary class="small muted">{words().show_advanced}</summary><div class="form-grid mt">{inner}</div></details> }
    });
    view! { {grid}{opt(folded)} }
}

/// A test result line: passed (✓) or failed (× and Copy).
#[component]
pub fn TestResult(
    /// `{ok, message}`.
    result: Value,
) -> impl IntoView {
    let ok = result.truthy("ok");
    let msg = {
        let m = result.g("message");
        if truthy(m) {
            js_string(m)
        } else if ok {
            words().connected.to_string()
        } else {
            words().failed.to_string()
        }
    };
    let copy = (!ok).then(|| view! { <CopyButton text=msg.clone()/> });
    view! {
        <div class=if ok { "test-result ok" } else { "test-result err" }>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(crate::icons::paths(if ok { "check" } else { "x" }))}>{""}</svg>
            <span>{msg}</span>{opt(copy)}
        </div>
    }
}

/// What the card's `[data-out]` line shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Out {
    /// Nothing.
    Empty,
    /// Test runs.
    Testing,
    /// Results (a test's, or the warnings of a save).
    Results(Vec<Value>),
}

/// Draws a card's extra blocks, by card id.
pub type Extra = Arc<dyn Fn(&str) -> AnyView + Send + Sync>;

/// What the app adds to its cards.
#[derive(Clone)]
pub struct CardHooks {
    /// The card's extra blocks after its fields (a preview, a list of the app's own), by card id.
    pub extra: Option<Extra>,
}

/// A card: a fold with the title, description and Test button in its summary, then the result line, the fields
/// and Save.
#[component]
pub fn SettingsCard(
    /// The card.
    group: Arc<Group>,
    /// Its fields that this server has.
    fields: Vec<Field>,
    /// `GET /api/settings`.
    values: Value,
    /// The server lets admins see saved secrets.
    #[prop(optional)]
    reveal: bool,
    /// Starts unfolded.
    #[prop(optional)]
    open: bool,
    /// What the result line shows (left alone, the line is the app's to write into).
    #[prop(optional, into)]
    out: Option<ArcRwSignal<Out>>,
    /// Save was pressed (the card's `<details>`).
    #[prop(optional, into)]
    on_save: Option<Callback<Element>>,
    /// Test was pressed (the card's `<details>`).
    #[prop(optional, into)]
    on_test: Option<Callback<Element>>,
    /// Drawn before the fields (a sign-in block of the app's own).
    #[prop(default = None)]
    before: Option<AnyView>,
    /// The card's extra blocks after its fields.
    #[prop(default = None)]
    extra: Option<AnyView>,
    /// A run of fields drawn as a block of the app's own.
    #[prop(optional)]
    block: Option<FieldBlock>,
) -> impl IntoView {
    let w = words();
    let node = NodeRef::<leptos::html::Details>::new();
    let el = move || node.get_untracked().map(|d| d.unchecked_into::<Element>());
    let out_view = out.map(|out| {
        move || match out.get() {
            Out::Empty => "".into_any(),
            Out::Testing => view! { <div class="test-result"><span class="spinner">{""}</span><span>{w.testing}</span></div> }.into_any(),
            Out::Results(rs) => seq(rs.into_iter().map(|r| view! { <TestResult result=r/> })),
        }
    });
    let desc = (!group.desc.is_empty())
        .then(|| view! { <div {..()} class="small muted" style="margin-top:4px;font-weight:400">{group.desc.clone()}</div> });
    let test = group.test.as_ref().map(|_| {
        let label = group.test_label.clone().unwrap_or_else(|| w.test_connection.to_string());
        view! {
            <button class="btn sm" data-test on:click=move |e| {
                if let (Some(el), Some(f)) = (el(), on_test) { e.prevent_default(); f.run(el); }
            } {..html(format!("{}{}", icon("refresh"), crate::html::esc(&label)))}>{""}</button>
        }
    });
    view! {
        <details class="card settings-card" data-sec=group.id.clone() open=open node_ref=node>
            <summary {..()} class="row between" style="gap:10px;flex-wrap:nowrap;align-items:flex-start">
                <div {..()} style="min-width:0"><h2>{group.title.clone()}</h2>{opt(desc)}</div>
                <span {..()} class="row" style="gap:8px">{opt(test)}<span class="chev" {..html(chevron())}>{""}</span></span>
            </summary>
            <div {..()} style="height:14px">{""}</div>
            <div data-out>{opt(out_view)}</div>
            {opt(before)}
            <SchemaFields fields=fields values=values reveal=reveal block=block.unwrap_or_else(|| FieldBlock { prefix: "\u{0}".into(), draw: Arc::new(|v| v) })/>
            {opt(extra)}
            <div {..()} class="row mt" style="justify-content:flex-end">
                <button class="btn primary" data-save on:click=move |_| { if let (Some(el), Some(f)) = (el(), on_save) { f.run(el); } }
                    {..html(format!("{}{}", icon("check"), w.save))}>{""}</button>
            </div>
        </details>
    }
}

/// What Save sends for a card: every field of `g` found under `root`, except those an environment variable or a
/// plugin sets, an empty secret (keeps the saved one) and a mask the server sent; `checks` as comma-separated
/// values.
pub fn form_values(g: &Group, root: &Element, meta: &Value) -> Map<String, Value> {
    let mut body = Map::new();
    for el in crate::rt::qsa_in(root, "[data-k]") {
        let k = el.get_attribute("data-k").unwrap_or_default();
        let Some(f) = g.field(&k) else { continue };
        if matches!(meta.o(&f.key).s("source").as_str(), "env" | "plugin") {
            continue;
        }
        let secret = f.is_secret();
        if f.kind == "checks" {
            let on: Vec<String> =
                crate::rt::qsa_in(&el, "input:checked").iter().map(|i| i.get_attribute("value").unwrap_or_default()).collect();
            body.insert(f.key.clone(), Value::String(on.join(",")));
        } else if el.get_attribute("type").as_deref() == Some("checkbox") {
            body.insert(f.key.clone(), Value::Bool(el.dyn_ref::<web_sys::HtmlInputElement>().is_some_and(|i| i.checked())));
        } else {
            let v = value_of(&el);
            if (secret && v.trim().is_empty()) || is_masked(&v) {
                // keep the saved secret; never echo a mask back
            } else {
                body.insert(f.key.clone(), Value::String(if secret { v.trim().to_string() } else { v }));
            }
        }
    }
    body
}

fn value_of(el: &Element) -> String {
    if let Some(i) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        i.value()
    } else if let Some(t) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        t.value()
    } else if let Some(s) = el.dyn_ref::<web_sys::HtmlSelectElement>() {
        s.value()
    } else {
        String::new()
    }
}

fn set_value(el: &Element, v: &str) {
    if let Some(i) = el.dyn_ref::<web_sys::HtmlInputElement>() {
        i.set_value(v);
    } else if let Some(t) = el.dyn_ref::<web_sys::HtmlTextAreaElement>() {
        t.set_value(v);
    }
}

/// A saved secret, asked for when the eye or copy button is pressed.
async fn secret_of(key: &str) -> Result<String, ApiError> {
    let enc: String = js_sys::encode_uri_component(key).into();
    let path = crate::config::Paths::fill(&crate::config::get().paths.settings_secret, &enc);
    Ok(api::get(&path).await?.s("value"))
}

/// The eye and copy buttons of saved secrets in `card` (asked for from the server only when pressed).
pub fn bind_secret_buttons(card: &Element) {
    for b in crate::rt::qsa_in(card, "[data-reveal]") {
        let (card, b2) = (card.clone(), b.clone());
        crate::rt::listen_forever(&b, "click", move |e| {
            e.prevent_default();
            let key = b2.get_attribute("data-reveal").unwrap_or_default();
            let Some(input) = crate::rt::qs_in(&card, &format!(r#"[data-k="{key}"]"#)) else { return };
            let is_textarea = input.tag_name() == "TEXTAREA";
            let hidden =
                if is_textarea { !input.has_attribute("data-shown") } else { input.get_attribute("type").as_deref() == Some("password") };
            let b = b2.clone();
            crate::rt::spawn(async move {
                if hidden {
                    if value_of(&input).is_empty() {
                        match secret_of(&key).await {
                            Ok(v) => {
                                set_value(&input, &v);
                                let _ = input.set_attribute("data-revealed", "1");
                            }
                            Err(err) => {
                                toast(&err.message, "err");
                                return;
                            }
                        }
                    }
                    if !is_textarea {
                        let _ = input.set_attribute("type", "text");
                    }
                    let _ = input.set_attribute("data-shown", "1");
                } else {
                    if input.has_attribute("data-revealed") {
                        set_value(&input, "");
                        let _ = input.remove_attribute("data-revealed");
                    }
                    if !is_textarea {
                        let _ = input.set_attribute("type", "password");
                    }
                    let _ = input.remove_attribute("data-shown");
                }
                let w = words();
                if b.class_list().contains("eye-btn") {
                    b.set_inner_html(if hidden { EYE_OFF } else { EYE });
                    let _ = b.set_attribute("title", if hidden { w.hide } else { w.show });
                } else {
                    b.set_inner_html(&if hidden { format!("{EYE_OFF}{}", w.hide) } else { format!("{EYE}{}", w.show_saved) });
                }
            });
        });
    }
    for b in crate::rt::qsa_in(card, "[data-copy]") {
        let (card, b2) = (card.clone(), b.clone());
        crate::rt::listen_forever(&b, "click", move |e| {
            e.prevent_default();
            let key = b2.get_attribute("data-copy").unwrap_or_default();
            let Some(input) = crate::rt::qs_in(&card, &format!(r#"[data-k="{key}"]"#)) else { return };
            crate::rt::spawn(async move {
                let v = value_of(&input);
                let text = if !v.is_empty() {
                    v
                } else {
                    match secret_of(&key).await {
                        Ok(s) => s,
                        Err(err) => {
                            toast(&err.message, "err");
                            return;
                        }
                    }
                };
                copy_text(&text);
            });
        });
    }
}

/// The cards of one settings page, wired to the server: Save, Test, clear and the secret buttons.
///
/// `cards` are the page's [`visible_cards`]; a link may name the card to open (`focus`), else the first one
/// opens (a page of one card has it open). After clear the values are asked for again and the cards drawn
/// anew.
#[component]
pub fn SettingsCards(
    /// The page's cards with their fields.
    cards: Vec<(Arc<Group>, Vec<Field>)>,
    /// `GET /api/settings`.
    values: Value,
    /// The server lets admins see saved secrets.
    #[prop(optional)]
    reveal: bool,
    /// The card a link asked for.
    #[prop(default = None)]
    focus: Option<String>,
    /// The cards' extra blocks.
    #[prop(optional)]
    hooks: Option<CardHooks>,
) -> impl IntoView {
    let values = ArcRwSignal::new(values);
    let drawn = ArcRwSignal::new(0u32);
    let focus = focus.filter(|f| cards.iter().any(|(g, _)| g.id == *f));
    let n = cards.len();
    let cards = Arc::new(cards);
    let v2 = values.clone();
    let d2 = drawn.clone();
    move || {
        d2.track();
        let vals = v2.get_untracked();
        let mut part: Option<String> = None;
        let mut out = Vec::new();
        for (i, (g, fields)) in cards.iter().enumerate() {
            if part.as_deref() != Some(g.part.as_str()) {
                part = Some(g.part.clone());
                if !g.part.is_empty() {
                    out.push(view! { <div {..()} class="nav-label" style="padding:18px 4px 8px">{g.part.clone()}</div> }.into_any());
                }
            }
            let open = (i == 0 && focus.is_none()) || n == 1 || focus.as_deref() == Some(g.id.as_str());
            let line = ArcRwSignal::new(Out::Empty);
            let extra = hooks.as_ref().and_then(|h| h.extra.as_ref()).map(|f| f(&g.id));
            let (g_save, g_test, values_s, values_t, line_s, line_t) =
                (g.clone(), g.clone(), values.clone(), values.clone(), line.clone(), line.clone());
            let on_save = Callback::new(move |card: Element| {
                let (g, values, line) = (g_save.clone(), values_s.clone(), line_s.clone());
                crate::rt::spawn(async move {
                    let _ = save(&g, false, &card, values, line).await;
                });
            });
            let on_test = Callback::new(move |card: Element| {
                let (g, values, line) = (g_test.clone(), values_t.clone(), line_t.clone());
                let _ = card.set_attribute("open", "");
                let btn = crate::rt::qs_in(&card, "[data-test]");
                if let Some(b) = &btn {
                    let _ = b.set_attribute("disabled", "");
                }
                line.set(Out::Testing);
                crate::rt::spawn(async move {
                    let failed = |e: ApiError| serde_json::json!({ "ok": false, "message": if e.message.is_empty() { words().test_failed.to_string() } else { e.message } });
                    let test = crate::config::Paths::fill(&crate::config::get().paths.test, g.test.as_deref().unwrap_or(""));
                    let r = match save(&g, true, &card, values, line.clone()).await {
                        Ok(()) => match api::post_empty(&test).await {
                            Ok(r) => r,
                            Err(e) => failed(e),
                        },
                        Err(e) => failed(e),
                    };
                    line.set(Out::Results(vec![r]));
                    if let Some(b) = btn {
                        let _ = b.remove_attribute("disabled");
                    }
                });
            });
            out.push(
                view! { <SettingsCard group=g.clone() fields=fields.clone() values=vals.clone() reveal=reveal open=open out=line on_save=on_save on_test=on_test extra=extra/> }
                    .into_any(),
            );
        }
        // the secret buttons and the clear links, once the cards are on the page
        if crate::rt::IN_BROWSER {
            let (values, drawn) = (values.clone(), drawn.clone());
            request_animation_frame(move || bind_page(values, drawn));
        }
        seq(out)
    }
}

fn bind_page(values: ArcRwSignal<Value>, drawn: ArcRwSignal<u32>) {
    for card in crate::rt::qsa("[data-sec]") {
        if card.has_attribute("data-kit-bound") {
            continue;
        }
        let _ = card.set_attribute("data-kit-bound", "");
        bind_secret_buttons(&card);
        for a in crate::rt::qsa_in(&card, "[data-clear]") {
            let (a2, values, drawn) = (a.clone(), values.clone(), drawn.clone());
            crate::rt::listen_forever(&a, "click", move |e| {
                e.prevent_default();
                let key = a2.get_attribute("data-clear").unwrap_or_default();
                let (values, drawn) = (values.clone(), drawn.clone());
                crate::rt::spawn(async move {
                    let mut body = Map::new();
                    body.insert(key, Value::String(String::new()));
                    match api::put(&crate::config::get().paths.settings, Value::Object(body)).await {
                        Ok(v) => {
                            values.set(v);
                            toast(words().cleared, "ok");
                            drawn.update(|d| *d += 1);
                        }
                        Err(e) => web_sys::console::error_1(&e.message.into()),
                    }
                });
            });
        }
    }
}

/// Save of a card (and the quiet save before Test): `PUT /api/settings` with what the card's fields hold;
/// the answer becomes the values, and a folder the server cannot write to comes back as a warning under the card.
async fn save(g: &Group, quiet: bool, card: &Element, values: ArcRwSignal<Value>, line: ArcRwSignal<Out>) -> Result<(), ApiError> {
    let meta = values.get_untracked().o("_meta");
    let body = form_values(g, card, &meta);
    match api::put(&crate::config::get().paths.settings, Value::Object(body)).await {
        Ok(mut v) => {
            let warnings: Vec<String> = v
                .g("_warnings")
                .as_array()
                .map(|a| a.iter().map(|w| w.s("message")).filter(|m| !m.is_empty()).collect())
                .unwrap_or_default();
            if let Some(o) = v.as_object_mut() {
                o.remove("_warnings");
            }
            values.set(v);
            if !quiet {
                toast(words().saved, "ok");
            }
            if !warnings.is_empty() {
                line.set(Out::Results(warnings.into_iter().map(|m| serde_json::json!({ "ok": false, "message": m })).collect()));
            }
            Ok(())
        }
        Err(e) => {
            toast(&e.message, "err");
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests;
