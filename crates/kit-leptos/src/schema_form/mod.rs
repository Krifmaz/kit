//! The settings cards drawn from the server's schema, so a new setting needs no UI work.
//!
//! From Encore's `web-rs/src/settings/schema_form.rs` (`field_html`, `source_note`, `env_note`, `show_when`,
//! `bind_show_when`, `fields_html`, `form_html`, `form_values`), `groups.rs` (`card_html`, `visible_cards`,
//! `cards_html`, `bind_card`, `bind_clear`, `bind_secret_buttons`, `Hub::save`) and `settings/mod.rs`
//! (`test_result`). One renderer draws every type: a switch for `bool`, a row of check boxes for `checks`, a
//! drop-down for `select`, a text area, a password input that starts empty (with the eye and copy buttons when
//! the server lets admins see saved secrets), and a text or number input for the rest. Under each field: its help
//! line and where its value comes from (an environment variable locks it; a saved secret says so and can be
//! cleared). A field with a `when` rule shows only while its select has the value. Fields marked `advanced` fold
//! under "Show advanced". The markup is Encore's; the tests hold it to Encore's recorded fixture of every page.
//!
//! [`SettingsCards`] draws a page's cards and wires Save (`PUT /api/admin/settings`), Test
//! (`POST /api/system/test/<test>` after a quiet save), clear and the eye and copy buttons
//! (`GET /api/admin/settings/secret/<key>`); [`SchemaFields`] is the fields alone.

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

/// What `GET /api/admin/settings` sends for a saved secret ("•••• (saved)"). From `schema_form.rs` `MASK`.
pub const MASK: &str = "••••";
/// The fold of the advanced fields. From `schema_form.rs` `fields_html`.
pub const SHOW_ADVANCED: &str = "Show advanced";
/// The card's Save. From `groups.rs` `card_html`.
pub const SAVE: &str = "Save";
/// The toast after Save. From `groups.rs` `Hub::save`.
pub const SAVED: &str = "Settings saved";
/// The toast after a secret's clear. From `groups.rs` `bind_clear`.
pub const CLEARED: &str = "Cleared";
/// The Test button when the card names no words. From `groups.rs` `card_html`.
pub const TEST_CONNECTION: &str = "Test connection";
/// The line while Test runs. From `groups.rs` `bind_card`.
pub const TESTING: &str = "Testing…";
/// Test's result when it failed without a sentence. From `groups.rs` `bind_card`.
pub const TEST_FAILED: &str = "Test failed";
/// A test result that passed without a sentence. From `settings/mod.rs` `test_result`.
pub const CONNECTED: &str = "Connected";
/// A test result that failed without a sentence. From `settings/mod.rs` `test_result`.
pub const FAILED: &str = "Failed";
/// A secret text area's button, and the eye's title. From `schema_form.rs` `field_html` and `groups.rs`.
pub const SHOW_SAVED: &str = "Show saved";
/// The eye's title while hidden. From `schema_form.rs` `field_html`.
pub const SHOW: &str = "Show";
/// The eye's title while shown. From `groups.rs` `bind_secret_buttons`.
pub const HIDE: &str = "Hide";
/// The copy buttons of a secret. From `schema_form.rs` `field_html`.
pub const COPY: &str = kit_rules::toasts::COPY;

fn is_masked(v: &str) -> bool {
    v.starts_with(MASK)
}

/// Whether an environment variable sets `key` (`meta`: the `_meta` of `GET /api/admin/settings`): the value is
/// shown locked and Save leaves it out. From `schema_form.rs` `env_locked`.
pub fn env_locked(meta: &Value, key: &str) -> bool {
    meta.o(key).s("source") == "env"
}

/// The note under a value an environment variable sets ("🔒 Set by ENCORE_… in your compose/env – change it
/// there (environment always wins)"). From `schema_form.rs` `env_note`.
#[component]
pub fn EnvNote(
    /// The variable's name.
    #[prop(into)]
    env: String,
) -> impl IntoView {
    view! { <small class="src env">"🔒 Set by "<span class="mono">{env}</span>" in your compose/env – change it there (environment always wins)"</small> }
}

/// The note under a field: set by an environment variable, a saved secret (with clear), a secret that is not
/// set, a plugin's value, or the environment variable that would set it. From `schema_form.rs` `source_note`.
fn source_note(f: &Field, v: &Value, meta: &Value) -> AnyView {
    let m = meta.o(&f.key);
    let secret = f.is_secret();
    if env_locked(meta, &f.key) {
        return view! { <EnvNote env=m.s("env")/> }.into_any();
    }
    if secret && truthy(v) {
        return view! { <small class="src">"Saved ✓ (hidden) · "<a href="#" data-clear=f.key.clone()>"clear"</a></small> }.into_any();
    }
    if secret {
        let env = m.truthy("env").then(|| view! { " – or set "<span class="mono">{m.s("env")}</span>" in your compose file" });
        return view! { <small class="src">"Not set"{opt(env)}</small> }.into_any();
    }
    if m.s("source") == "plugin" {
        return view! { <small class="src plugin">"🔗 Filled in by the "<b>{m.s("plugin")}</b>" plugin – it changes this itself"</small> }
            .into_any();
    }
    if m.s("source") == "ui" {
        return "".into_any();
    }
    opt(m.truthy("env").then(|| view! { <small class="src faint">"Default · env var "<span class="mono">{m.s("env")}</span></small> }))
}

/// The values of the `when` selects of a run of fields, as they change.
pub type Selects = ArcRwSignal<HashMap<String, String>>;

/// One field: `v` its value from `GET /api/admin/settings`, `meta` that answer's `_meta`, `reveal` whether the
/// server lets admins see saved secrets. From `schema_form.rs` `field_html` and `show_when`: the same elements,
/// attributes in the same order (a `when` rule's `data-when`, `data-when-in` or `data-when-not` and `hidden` before
/// `class`) and the same text nodes, so the page is Encore's node for node.
pub fn field_view(f: &Field, v: &Value, meta: &Value, reveal: bool, selects: Selects) -> AnyView {
    let key = f.key.clone();
    let locked = meta.o(&f.key).s("source") == "env";
    let readonly = !locked && meta.o(&f.key).s("source") == "plugin";
    let help = (opt(f.help.clone().map(|h| view! { <small>{h}</small> })), source_note(f, v, meta));
    let cls = if f.full || f.kind == "textarea" { "field full" } else { "field " };
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
            // option values are text; a numeric setting (streamrip_quality = 2) compares as text
            let vs = js_string(v);
            let opts = seq(f.options.iter().map(|(val, lab)| view! { <option value=val.clone() selected=*val == vs>{lab.clone()}</option> }));
            let k2 = key.clone();
            let sel = selects.clone();
            let select = view! {
                <select class="input" data-k=key.clone() id=format!("f-{key}") disabled=locked
                    on:change=move |e| { let v = event_target_value(&e); sel.update(|m| { m.insert(k2.clone(), v); }); }>{"\n      "}{opts}</select>
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
            // also for values set by environment variables (read-only, but viewable); never the WireGuard private key
            let reveal = reveal && truthy(v) && f.key != "vpn_wireguard_config";
            if f.kind == "textarea" {
                let btns = reveal.then(|| {
                    view! {
                        <span {..()} class="row" style="gap:8px">
                            <button {..()} type="button" class="btn sm" data-reveal=key.clone() {..html(format!("{EYE}{SHOW_SAVED}"))}>{""}</button>
                            <button {..()} type="button" class="btn sm" data-copy=key.clone() {..html(format!("{}{COPY}", icon("copy")))}>{""}</button>
                        </span>
                    }
                });
                let ph = if truthy(v) { "Saved – paste to replace" } else { "" };
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
                    "set by environment"
                } else if truthy(v) {
                    "saved – paste a new value to replace"
                } else {
                    "not set"
                };
                let input = view! {
                    <input class="input" type="password" data-k=key.clone() value="" disabled=locked readonly=readonly data-eye=reveal.then_some("1") autocomplete="new-password" placeholder=ph/>
                };
                let wrapped = if reveal {
                    view! {
                        <span class="pw-wrap secret">{input}
                            <button {..()} type="button" class="eye-btn" data-reveal=key.clone() title=SHOW aria-label={SHOW} {..html(EYE)}>{""}</button>
                            <button {..()} type="button" class="eye-btn" data-copy=key.clone() title=COPY aria-label={COPY} {..html(icon("copy"))}>{""}</button>
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

/// Draws a run of fields as a block of the app's own (Encore's VPN for Soulseek around the `vpn_` fields).
pub type Block = Arc<dyn Fn(AnyView) -> AnyView + Send + Sync>;

/// A block of fields: those whose key starts with `prefix` are drawn by `draw`, in place of the first of them,
/// and the fields after them in a grid of their own (`form-grid mt`). From `schema_form.rs` `grid_html`.
#[derive(Clone)]
pub struct FieldBlock {
    /// The key prefix of the block's fields (`vpn_`).
    pub prefix: String,
    /// Draws the block around its fields' grid cells.
    pub draw: Block,
}

/// The fields of a card with their values: a grid, and the advanced ones folded under "Show advanced". From
/// `schema_form.rs` `form_html` / `fields_html` / `grid_html`.
#[component]
pub fn SchemaFields(
    /// The fields to draw (those the values hold, [`visible_cards`]).
    fields: Vec<Field>,
    /// `GET /api/admin/settings` (with `_meta`).
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
        view! { <details class="adv-fields mt"><summary class="small muted">{SHOW_ADVANCED}</summary><div class="form-grid mt">{inner}</div></details> }
    });
    view! { {grid}{opt(folded)} }
}

/// A test result line: passed (✓) or failed (× and Copy). From `settings/mod.rs` `test_result`.
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
            CONNECTED.to_string()
        } else {
            FAILED.to_string()
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
    /// The card's extra blocks after its fields (Encore's Cadenza card has two), by card id.
    pub extra: Option<Extra>,
}

/// A card: a fold with the title, description and Test button in its summary, then the result line, the fields
/// and Save. From `groups.rs` `card_html`, with its line breaks between the tags.
#[component]
pub fn SettingsCard(
    /// The card.
    group: Arc<Group>,
    /// Its fields that this server has.
    fields: Vec<Field>,
    /// `GET /api/admin/settings`.
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
    /// Drawn before the fields (Encore's YouTube sign-in block).
    #[prop(default = None)]
    before: Option<AnyView>,
    /// The card's extra blocks after its fields.
    #[prop(default = None)]
    extra: Option<AnyView>,
    /// A run of fields drawn as a block of the app's own.
    #[prop(optional)]
    block: Option<FieldBlock>,
) -> impl IntoView {
    let node = NodeRef::<leptos::html::Details>::new();
    let el = move || node.get_untracked().map(|d| d.unchecked_into::<Element>());
    let out_view = out.map(|out| {
        move || match out.get() {
            Out::Empty => "".into_any(),
            Out::Testing => view! { <div class="test-result"><span class="spinner">{""}</span><span>{TESTING}</span></div> }.into_any(),
            Out::Results(rs) => seq(rs.into_iter().map(|r| view! { <TestResult result=r/> })),
        }
    });
    let desc = (!group.desc.is_empty())
        .then(|| view! { <div {..()} class="small muted" style="margin-top:4px;font-weight:400">{group.desc.clone()}</div> });
    let test = group.test.as_ref().map(|_| {
        let label = group.test_label.clone().unwrap_or_else(|| TEST_CONNECTION.to_string());
        view! {
            <button class="btn sm" data-test on:click=move |e| {
                if let (Some(el), Some(f)) = (el(), on_test) { e.prevent_default(); f.run(el); }
            } {..html(format!("{}{}", icon("refresh"), crate::html::esc(&label)))}>{""}</button>
        }
    });
    view! {
        <details class="card settings-card" data-sec=group.id.clone() open=open node_ref=node>
            {"\n      "}
            <summary {..()} class="row between" style="gap:10px;flex-wrap:nowrap;align-items:flex-start">
                <div {..()} style="min-width:0"><h2>{group.title.clone()}</h2>{opt(desc)}</div>
                {"\n        "}
                <span {..()} class="row" style="gap:8px">{opt(test)}<span class="chev" {..html(chevron())}>{""}</span></span>
            </summary>
            {"\n      "}
            <div {..()} style="height:14px">{""}</div>
            {"\n      "}
            <div data-out>{opt(out_view)}</div>
            {"\n      "}
            {opt(before)}
            <SchemaFields fields=fields values=values reveal=reveal block=block.unwrap_or_else(|| FieldBlock { prefix: "\u{0}".into(), draw: Arc::new(|v| v) })/>
            {opt(extra)}
            {"\n      "}
            <div {..()} class="row mt" style="justify-content:flex-end">
                <button class="btn primary" data-save on:click=move |_| { if let (Some(el), Some(f)) = (el(), on_save) { f.run(el); } }
                    {..html(format!("{}{SAVE}", icon("check")))}>{""}</button>
            </div>
            {"\n    "}
        </details>
    }
}

/// What Save sends for a card: every field of `g` found under `root`, except those an environment variable or a
/// plugin sets, an empty secret (keeps the saved one) and a mask the server sent; `checks` as comma-separated
/// values. From `schema_form.rs` `form_values`.
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

/// A saved secret, asked for when the eye or copy button is pressed. From `groups.rs` `secret_of`.
async fn secret_of(key: &str) -> Result<String, ApiError> {
    let enc: String = js_sys::encode_uri_component(key).into();
    Ok(api::get(&format!("/admin/settings/secret/{enc}")).await?.s("value"))
}

/// The eye and copy buttons of saved secrets in `card` (asked for from the server only when pressed). From
/// `groups.rs` `bind_secret_buttons`.
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
                if b.class_list().contains("eye-btn") {
                    b.set_inner_html(if hidden { EYE_OFF } else { EYE });
                    let _ = b.set_attribute("title", if hidden { HIDE } else { SHOW });
                } else {
                    b.set_inner_html(&if hidden { format!("{EYE_OFF}{HIDE}") } else { format!("{EYE}{SHOW_SAVED}") });
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
/// opens (a page of one card has it open). After clear the values are asked for again and the cards drawn anew,
/// as Encore does. From `groups.rs` `cards_html`, `bind_card` and `Hub::save`.
#[component]
pub fn SettingsCards(
    /// The page's cards with their fields.
    cards: Vec<(Arc<Group>, Vec<Field>)>,
    /// `GET /api/admin/settings`.
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
                    let failed = |e: ApiError| serde_json::json!({ "ok": false, "message": if e.message.is_empty() { TEST_FAILED.to_string() } else { e.message } });
                    let r = match save(&g, true, &card, values, line.clone()).await {
                        Ok(()) => match api::post_empty(&format!("/system/test/{}", g.test.as_deref().unwrap_or(""))).await {
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
                    match api::put("/admin/settings", Value::Object(body)).await {
                        Ok(v) => {
                            values.set(v);
                            toast(CLEARED, "ok");
                            drawn.update(|d| *d += 1);
                        }
                        Err(e) => web_sys::console::error_1(&e.message.into()),
                    }
                });
            });
        }
    }
}

/// Save of a card (and the quiet save before Test): `PUT /api/admin/settings` with what the card's fields hold;
/// the answer becomes the values, and a folder the server cannot write to comes back as a warning under the card.
/// From `groups.rs` `Hub::save`.
async fn save(g: &Group, quiet: bool, card: &Element, values: ArcRwSignal<Value>, line: ArcRwSignal<Out>) -> Result<(), ApiError> {
    let meta = values.get_untracked().o("_meta");
    let body = form_values(g, card, &meta);
    match api::put("/admin/settings", Value::Object(body)).await {
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
                toast(SAVED, "ok");
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
