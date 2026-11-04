//! The settings cards drawn from the kit's sample schema ([`SAMPLE_SCHEMA`]).

use super::schema::{Schema, When};
use super::*;
use crate::testing::{assert_same_html, render, tokens, Token};
use serde_json::json;

fn schema_json() -> Value {
    serde_json::from_str(SAMPLE_SCHEMA).expect("the sample schema is JSON")
}
fn schema() -> Schema {
    Schema::from_json(&schema_json())
}

fn all_fields(v: &Value) -> Vec<Value> {
    v.a("sections").iter().flat_map(|s| s.a("groups")).flat_map(|g| g.a("fields")).collect()
}

/// `GET /api/settings` with every field at its default, set in the app (`source: ui`).
fn defaults() -> Value {
    let mut values = Map::new();
    let mut meta = Map::new();
    for f in all_fields(&schema_json()) {
        values.insert(f.s("key"), f.g("default").clone());
        meta.insert(f.s("key"), json!({ "env": f.g("env"), "source": "default" }));
    }
    values.insert("_meta".into(), Value::Object(meta));
    Value::Object(values)
}

/// `values` with `key` set to `v` from `source`.
fn with(mut values: Value, key: &str, v: Value, source: &str) -> Value {
    values[key] = v;
    values["_meta"][key]["source"] = json!(source);
    values
}

/// One settings page as the kit draws it.
fn page(id: &str, values: &Value, reveal: bool, focus: Option<&str>) -> String {
    let schema = schema();
    let sec = schema.section(id).expect("a page of the schema");
    let cards = visible_cards(&sec.groups, values);
    let (values, focus) = (values.clone(), focus.map(str::to_string));
    render(move || view! { <SettingsCards cards=cards values=values reveal=reveal focus=focus/> })
}

/// Whether `html` has an element `tag` with all of `attrs`.
fn has(html: &str, tag: &str, attrs: &[(&str, &str)]) -> bool {
    tokens(html)
        .iter()
        .any(|t| matches!(t, Token::Open(n, a) if n == tag && attrs.iter().all(|(k, v)| a.iter().any(|(ak, av)| ak == k && av == v))))
}

#[test]
fn a_card_draws_its_fields_save_and_notes() {
    let schema = schema();
    let g = schema.group("app").expect("the App card");
    let values = with(defaults(), "app_name", json!("My <App> & \"more\""), "ui");
    let cards = visible_cards(&[g], &values);
    let html = render(move || view! { <SettingsCards cards=cards values=values/> });
    let want = format!(
        r#"<details data-sec="app" open class="card settings-card">
            <summary class="row between" style="gap:10px;flex-wrap:nowrap;align-items:flex-start;">
              <div style="min-width:0;"><h2>App</h2><div class="small muted" style="margin-top:4px;font-weight:400;">The name people see and where the app is reached.</div></div>
              <span class="row" style="gap:8px;"><span class="chev">{chev}</span></span>
            </summary>
            <div style="height:14px;"></div>
            <div data-out></div>
            <div class="form-grid">
              <label class="field"><span>App name</span><input type="text" data-k="app_name" value="My &lt;App&gt; &amp; &quot;more&quot;" autocomplete="off" class="input"></label>
              <label class="field"><span>Public address</span><input type="text" data-k="base_url" value="" autocomplete="off" class="input">
                <small>Used in links the app sends out.</small><small class="src faint">Default · env var <span class="mono">APP_BASE_URL</span></small></label>
              <label class="field"><span>Stay signed in (days)</span><input type="number" data-k="session_days" step="1" value="30" autocomplete="off" class="input"></label>
              <div class="field" style="justify-content:center;"><label class="switch"><input type="checkbox" data-k="signups"><i></i>Let people sign up</label></div>
            </div>
            <div class="row mt" style="justify-content:flex-end;"><button data-save class="btn primary">{check}Save</button></div>
          </details>"#,
        chev = crate::icons::chevron(),
        check = icon("check")
    );
    assert_same_html(&html, &want);
}

#[test]
fn a_rule_each_way_shows_and_hides_its_fields() {
    let draw = |storage: &str| page("general", &with(defaults(), "storage", json!(storage), "ui"), false, None);
    let local = draw("local");
    assert!(has(&local, "label", &[("data-when", "storage"), ("data-when-in", "local")]), "{local}");
    assert!(!has(&local, "label", &[("data-when-in", "local"), ("hidden", "")]), "{local}");
    assert!(has(&local, "label", &[("data-when-in", "s3"), ("hidden", "")]), "{local}");
    assert!(has(&local, "label", &[("data-when-not", "local"), ("hidden", "")]), "{local}");
    let s3 = draw("s3");
    assert!(has(&s3, "label", &[("data-when-in", "local"), ("hidden", "")]), "{s3}");
    assert!(!has(&s3, "label", &[("data-when-in", "s3"), ("hidden", "")]), "{s3}");
    assert!(!has(&s3, "label", &[("data-when-not", "local"), ("hidden", "")]), "{s3}");
    assert!(has(&s3, "option", &[("value", "s3"), ("selected", "")]), "{s3}");
    assert!(When::parse("a=b").is_some());
}

#[test]
fn every_field_type_of_the_schema_is_drawn() {
    let mut v = schema_json();
    v["sections"][0]["groups"][0]["fields"][0]["type"] = json!("colour");
    let schema = Schema::from_json(&v);
    let values = with(defaults(), "notify", json!("Reports,readonly"), "ui");
    let cards = visible_cards(&schema.section("general").unwrap().groups, &values);
    let v2 = values.clone();
    let general = render(move || view! { <SettingsCards cards=cards values=v2/> });
    assert!(
        has(&general, "input", &[("class", "input"), ("type", "text"), ("data-k", "app_name")]),
        "a type this client does not know, as text"
    );
    assert!(has(&general, "input", &[("type", "text"), ("data-k", "base_url")]), "url as text");
    assert!(has(&general, "input", &[("type", "text"), ("data-k", "data_path")]), "path as text");
    assert!(has(&general, "input", &[("type", "number"), ("step", "0.5")]), "number with its step");
    assert!(has(&general, "input", &[("type", "checkbox"), ("data-k", "signups")]), "bool as a switch");
    assert!(has(&general, "select", &[("data-k", "storage")]), "select");
    assert!(has(&general, "input", &[("type", "password"), ("data-k", "s3_secret"), ("placeholder", "not set")]), "password");
    let mail = page("mail", &values, false, None);
    assert!(has(&mail, "div", &[("class", "field"), ("data-k", "notify")]), "checks");
    assert!(
        has(&mail, "input", &[("value", "reports"), ("checked", "")]) && has(&mail, "input", &[("value", "read-only"), ("checked", "")]),
        "{mail}"
    );
    assert!(!has(&mail, "input", &[("value", "signups"), ("checked", "")]), "{mail}");
    assert!(has(&mail, "textarea", &[("class", "input mono"), ("data-k", "footer")]), "textarea");
    assert!(has(&mail, "label", &[("class", "field full")]), "a textarea takes the whole width");
    // the card's Test button says the card's words
    assert!(mail.contains("Send a test mail</button>") && has(&mail, "button", &[("data-test", "")]), "{mail}");
}

#[test]
fn secrets_and_values_set_elsewhere_say_so() {
    let values = with(defaults(), "smtp_password", json!("•••• (saved)"), "ui");
    let values = with(values, "dkim_key", json!("•••• (saved)"), "ui");
    let values = with(values, "smtp_host", json!("mail.example.org"), "env");
    let mail = page("mail", &values, true, None);
    assert!(has(&mail, "input", &[("data-k", "smtp_host"), ("disabled", "")]), "set by the environment: locked");
    assert!(
        mail.contains(
            "🔒 Set by <span class=\"mono\">APP_SMTP_HOST</span> in the environment – change it there (the environment always wins)"
        ),
        "{mail}"
    );
    assert!(has(&mail, "a", &[("data-clear", "smtp_password")]) && mail.contains("Saved ✓ (hidden)"), "{mail}");
    assert!(
        has(&mail, "input", &[("data-k", "smtp_password"), ("placeholder", "saved – paste a new value to replace"), ("data-eye", "1")]),
        "{mail}"
    );
    assert!(has(&mail, "button", &[("class", "eye-btn"), ("data-reveal", "smtp_password"), ("title", "Show")]), "{mail}");
    assert!(has(&mail, "button", &[("data-reveal", "dkim_key")]) && has(&mail, "button", &[("data-copy", "dkim_key")]), "{mail}");
    assert!(has(&mail, "textarea", &[("data-k", "dkim_key"), ("placeholder", "Saved – paste to replace")]), "{mail}");
    // without reveal no button asks for a saved secret
    let hidden = page("mail", &values, false, None);
    assert!(!has(&hidden, "button", &[("data-reveal", "smtp_password")]), "{hidden}");
}

#[test]
fn advanced_fields_fold_and_a_value_filled_in_elsewhere_is_read_only() {
    let url = Field::from_json(&json!({ "key": "mirror_url", "label": "Mirror URL", "type": "url" }));
    let port = Field::from_json(&json!({ "key": "mirror_port", "label": "Mirror port", "advanced": true }));
    let values = json!({
        "mirror_url": "http://mirror.local", "mirror_port": "8081",
        "_meta": { "mirror_url": { "source": "plugin", "plugin": "the sync add-on" } }
    });
    let (f2, v2) = (vec![url.clone(), port], values.clone());
    let h = render(move || view! { <SchemaFields fields=f2 values=v2/> });
    let fold = h.find(SHOW_ADVANCED).expect("a fold");
    assert!(h.find(r#"data-k="mirror_url""#).unwrap() < fold && h.find(r#"data-k="mirror_port""#).unwrap() > fold, "{h}");
    assert!(
        has(&h, "input", &[("value", "http://mirror.local"), ("readonly", "")])
            && h.contains("🔗 Filled in by <b>the sync add-on</b> – it changes this itself"),
        "{h}"
    );
    let h = render(move || view! { <SchemaFields fields=vec![url] values=values/> });
    assert!(!h.contains(SHOW_ADVANCED));
}

#[test]
fn the_card_a_link_names_opens() {
    let values = defaults();
    let html = page("general", &values, false, Some("storage"));
    assert!(
        has(&html, "details", &[("data-sec", "storage"), ("open", "")]) && !has(&html, "details", &[("data-sec", "app"), ("open", "")]),
        "{html}"
    );
    // a heading over the cards of a part
    assert!(html.contains(r#"<div class="nav-label" style="padding:18px 4px 8px;">Files</div>"#), "{html}");
    // a page of one card has it open
    assert!(has(&page("mail", &values, false, None), "details", &[("data-sec", "smtp"), ("open", "")]));
}

#[test]
fn pages_are_found_by_id_alias_or_card() {
    let schema = schema();
    let nav: Vec<(&str, &str)> = schema.sections.iter().map(|s| (s.id.as_str(), s.title.as_str())).collect();
    assert_eq!(nav, [("general", "General"), ("mail", "Mail")]);
    for (id, want) in
        [("mail", "mail"), ("email", "mail"), ("smtp", "mail"), ("app", "general"), ("storage", "general"), ("nothing", "nothing")]
    {
        assert_eq!(schema.page_of(id), want, "?s={id}");
    }
    assert!(schema.holds("general", "storage") && !schema.holds("mail", "storage"));
    // the cards a server without some fields has
    let some = json!({ "app_name": "x", "_meta": {} });
    let cards = visible_cards(&schema.section("general").unwrap().groups, &some);
    assert_eq!(cards.iter().map(|(g, f)| (g.id.as_str(), f.len())).collect::<Vec<_>>(), [("app", 1)]);
}

#[test]
fn a_test_result_says_how_it_went() {
    let ok = render(|| view! { <TestResult result=json!({ "ok": true })/> });
    assert_same_html(&ok, &format!(r#"<div class="test-result ok">{}<span>Connected</span></div>"#, icon("check")));
    let err = render(|| view! { <TestResult result=json!({ "ok": false, "message": "refused <x>" })/> });
    assert_same_html(
        &err,
        &format!(
            r#"<div class="test-result err">{}<span>refused &lt;x&gt;</span>{}</div>"#,
            icon("x"),
            crate::details::copy_button_html("refused <x>")
        ),
    );
    assert!(render(|| view! { <TestResult result=json!({ "ok": false })/> }).contains("<span>Failed</span>"));
}

#[test]
fn an_app_words_the_form_its_own_way() {
    let mut cfg = crate::config::AppConfig::default();
    cfg.words.form.save = "Speichern";
    cfg.words.form.default_env = "Standard ({env})";
    crate::config::set(cfg);
    let html = page("general", &defaults(), false, None);
    assert!(html.contains("Speichern</button>") && html.contains(r#"Standard (<span class="mono">APP_NAME</span>)"#), "{html}");
    crate::config::set(crate::config::AppConfig::default());
}
