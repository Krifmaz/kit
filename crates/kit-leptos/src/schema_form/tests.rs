//! The settings cards drawn from the schema are the cards Encore draws.
//!
//! Ported from Encore's `web-rs/src/settings/form_tests.rs` and `schema_form.rs` tests. `fixtures/` holds
//! Encore's own fixtures, copied unchanged from Krifmaz/Encore at commit
//! `5c0bdcba82307c522e4516d1bc51a81c8efa4026` (`web-rs/src/settings/fixtures/`):
//! `settings-schema.json` is the answer of `GET /api/settings/schema` saved from Encore's server, and
//! `settings-cards.html` is what Encore's web app draws for every Server page, for two sets of values (the
//! defaults, and a case with locked, saved, secret, escaped and missing values), with and without the eye and
//! copy buttons; icons are shortened to `<svg/>` and the cards' own extra blocks to a marker. The kit draws every
//! page from the same schema and values and compares the documents (whitespace between elements and attribute
//! order aside, see `crate::testing`).

use super::schema::{Schema, When};
use super::*;
use crate::testing::{render, tokens, Token};
use serde_json::json;

const SCHEMA_JSON: &str = include_str!("fixtures/settings-schema.json");
const CARDS: &str = include_str!("fixtures/settings-cards.html");

/// The cards of Encore's Downloads page that became plugin cards in 0.32 (`plugins.rs` `MOVED`): Encore's Server
/// pages leave them out.
const MOVED: &[&str] = &["streamrip", "torznab", "slskd", "ytdlp", "youtube"];

fn schema_json() -> Value {
    serde_json::from_str(SCHEMA_JSON).expect("the schema fixture is JSON")
}
fn schema() -> Schema {
    Schema::from_json(&schema_json())
}

fn all_fields(v: &Value) -> Vec<Value> {
    v.a("sections").iter().flat_map(|s| s.a("groups")).flat_map(|g| g.a("fields")).collect()
}

fn field_mut<'a>(v: &'a mut Value, key: &str) -> &'a mut Value {
    v["sections"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|s| s["groups"].as_array_mut().unwrap().iter_mut())
        .flat_map(|g| g["fields"].as_array_mut().unwrap().iter_mut())
        .find(|f| f["key"] == key)
        .expect("a field of the schema")
}

/// `GET /settings` for the cases: (name, values with `_meta`). From `form_tests.rs` `cases`.
fn cases() -> Vec<(&'static str, Value)> {
    let mut values = Map::new();
    let mut meta = Map::new();
    for f in all_fields(&schema_json()) {
        values.insert(f.s("key"), f.g("default").clone());
        meta.insert(f.s("key"), json!({ "env": f.g("env"), "source": "default" }));
    }
    let defaults = {
        let mut v = values.clone();
        v.insert("_meta".into(), Value::Object(meta.clone()));
        Value::Object(v)
    };
    let mut set = |k: &str, v: Value, source: &str| {
        values.insert(k.to_string(), v);
        if let Some(m) = meta.get_mut(k) {
            m["source"] = json!(source);
        }
    };
    set("library_path", json!("/music"), "env");
    set("github_token", json!("•••• (saved)"), "env");
    set("discord_webhook", json!("•••• (saved)"), "ui");
    set("vpn_wireguard_config", json!("•••• (saved)"), "ui");
    set("ytdlp_cookies", json!("•••• (saved)"), "ui");
    set("lastfm_api_key", json!("•••• (saved)"), "ui");
    set("app_name", json!("My <Music> & \"more\""), "ui");
    set("prowlarr_url", json!("http://prowlarr:9696/?a=\"b\"&c<d>"), "ui");
    set("torrent_client", json!("deluge"), "ui");
    set("nzb_client", json!("blackhole"), "ui");
    set("release_types", json!("Album,EP,single"), "ui");
    set("release_skip", json!("Live,Remix,Spoken word"), "ui");
    set("streamrip_quality", json!(4), "ui");
    set("cadenza_max_rate", json!(384000), "ui");
    set("auto_approve", json!(true), "ui");
    set("ytdlp_enabled", json!(false), "ui");
    set("musicbrainz_rate", json!(0.5), "ui");
    set("autotag_threshold", json!(0.85), "ui");
    if let Some(m) = meta.get_mut("acoustid_api_key") {
        m["env"] = Value::Null;
    }
    for k in [
        "sync_interval_minutes",
        "staging_path",
        "roon_enabled",
        "roon_host",
        "smtp_host",
        "smtp_port",
        "smtp_user",
        "smtp_password",
        "smtp_from",
        "smtp_security",
    ] {
        values.remove(k);
    }
    let changed = {
        let mut v = values;
        v.insert("_meta".into(), Value::Object(meta));
        Value::Object(v)
    };
    vec![("defaults", defaults), ("changed", changed)]
}

/// Every `<svg …>…</svg>` as `<svg/>`, as the fixture records icons.
fn svgs_short(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(i) = rest.find("<svg") {
        out.push_str(&rest[..i]);
        match rest[i..].find("</svg>") {
            Some(j) => {
                out.push_str("<svg/>");
                rest = &rest[i + j + "</svg>".len()..];
            }
            None => {
                out.push_str(&rest[i..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// One Server page as the kit draws it (`page_cards` + `cards_html`).
fn page(schema: &Schema, id: &str, values: &Value, reveal: bool, focus: Option<&str>) -> String {
    let sec = schema.section(id).expect("a page of the schema");
    let cards: Vec<_> = visible_cards(&sec.groups, values).into_iter().filter(|(g, _)| !MOVED.contains(&g.id.as_str())).collect();
    let (values, focus) = (values.clone(), focus.map(str::to_string));
    svgs_short(&render(move || view! { <SettingsCards cards=cards values=values reveal=reveal focus=focus/> }))
}

/// The fixture's pages, by their marker line.
fn fixture_pages() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![];
    for line in CARDS.lines() {
        if line.starts_with("<!-- page ") {
            out.push((line.to_string(), String::new()));
        } else if let Some((_, body)) = out.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out
}

#[test]
fn the_cards_are_the_ones_encore_draws() {
    let schema = schema();
    let mut ours = vec![];
    for (case, values) in cases() {
        for reveal in [false, true] {
            for sec in &schema.sections {
                ours.push((
                    format!("<!-- page {}, {case} values, reveal {reveal} -->", sec.id),
                    page(&schema, &sec.id, &values, reveal, None),
                ));
            }
        }
    }
    ours.push((
        "<!-- page downloads, defaults, the Release types card asked for -->".to_string(),
        page(&schema, "downloads", &cases()[0].1, false, Some("releases")),
    ));
    let want = fixture_pages();
    assert_eq!(ours.iter().map(|o| &o.0).collect::<Vec<_>>(), want.iter().map(|w| &w.0).collect::<Vec<_>>(), "the same pages");
    for ((name, got), (_, want)) in ours.iter().zip(&want) {
        let (g, w) = (tokens(got), tokens(want));
        if g != w {
            let at = g.iter().zip(&w).position(|(a, b)| a != b).unwrap_or(g.len().min(w.len()));
            panic!(
                "{name}: the kit draws something else at token {at}:\n--- kit\n{:#?}\n--- Encore\n{:#?}",
                &g[at.saturating_sub(3)..(at + 3).min(g.len())],
                &w[at.saturating_sub(3)..(at + 3).min(w.len())]
            );
        }
    }
}

/// Whether `html` has an element `tag` with all of `attrs`.
fn has(html: &str, tag: &str, attrs: &[(&str, &str)]) -> bool {
    tokens(html)
        .iter()
        .any(|t| matches!(t, Token::Open(n, a) if n == tag && attrs.iter().all(|(k, v)| a.iter().any(|(ak, av)| ak == k && av == v))))
}

/// Ported from `form_tests.rs` `a_not_equal_rule_is_drawn_and_hides_its_field`.
#[test]
fn a_not_equal_rule_is_drawn_and_hides_its_field() {
    let mut v = schema_json();
    field_mut(&mut v, "qbit_url")["when"] = json!("torrent_client!=blackhole");
    let schema = Schema::from_json(&v);
    let draw = |client: &str| {
        let mut values = cases()[0].1.clone();
        values["torrent_client"] = json!(client);
        let sec = schema.section("downloads").unwrap();
        let cards = visible_cards(&sec.groups, &values);
        render(move || view! { <SettingsCards cards=cards values=values/> })
    };
    let rule = [("data-when", "torrent_client"), ("data-when-not", "blackhole"), ("class", "field")];
    let shown = draw("deluge");
    assert!(has(&shown, "label", &rule) && !has(&shown, "label", &[("data-when-not", "blackhole"), ("hidden", "")]), "{shown}");
    let hidden = draw("blackhole");
    assert!(has(&hidden, "label", &[("data-when-not", "blackhole"), ("hidden", "")]), "{hidden}");
}

/// Ported from `form_tests.rs` `every_field_type_of_the_schema_is_drawn`.
#[test]
fn every_field_type_of_the_schema_is_drawn() {
    let mut v = schema_json();
    field_mut(&mut v, "app_name")["type"] = json!("colour");
    let schema = Schema::from_json(&v);
    let values = &cases()[0].1;
    let html = page(&schema, "general", values, false, None);
    assert!(has(&html, "input", &[("class", "input"), ("type", "text"), ("data-k", "app_name")]), "{html}");
    assert!(has(&html, "input", &[("type", "text"), ("data-k", "base_url")]), "url as text");
    let library = page(&schema, "library", values, false, None);
    assert!(has(&library, "input", &[("type", "text"), ("data-k", "staging_path")]), "path as text");
}

/// Ported from `schema_form.rs` `advanced_fields_fold_and_a_plugins_value_is_read_only`.
#[test]
fn advanced_fields_fold_and_a_plugins_value_is_read_only() {
    let url = Field::from_json(&json!({ "key": "gamdl_wrapper_url", "label": "Wrapper URL", "type": "url" }));
    let dec = Field::from_json(&json!({ "key": "gamdl_wrapper_decrypt", "label": "Wrapper decrypt address", "advanced": true }));
    let values = json!({
        "gamdl_wrapper_url": "http://encore-wrapper", "gamdl_wrapper_decrypt": "encore-wrapper:10020",
        "_meta": { "gamdl_wrapper_url": { "source": "plugin", "plugin": "Apple Music wrapper" } }
    });
    let (f2, v2) = (vec![url.clone(), dec], values.clone());
    let h = render(move || view! { <SchemaFields fields=f2 values=v2/> });
    let fold = h.find(SHOW_ADVANCED).expect("a fold");
    assert!(h.find(r#"data-k="gamdl_wrapper_url""#).unwrap() < fold && h.find(r#"data-k="gamdl_wrapper_decrypt""#).unwrap() > fold, "{h}");
    assert!(
        has(&h, "input", &[("value", "http://encore-wrapper"), ("readonly", "")])
            && h.contains("Filled in by the <b>Apple Music wrapper</b> plugin"),
        "{h}"
    );
    let h = render(move || view! { <SchemaFields fields=vec![url] values=values/> });
    assert!(!h.contains(SHOW_ADVANCED));
}

/// Ported from `form_tests.rs` `the_nav_is_the_split_one_and_old_links_still_open` (the schema's part).
#[test]
fn pages_are_found_by_id_alias_or_card() {
    let schema = schema();
    let nav: Vec<(&str, &str)> = schema.sections.iter().map(|s| (s.id.as_str(), s.title.as_str())).collect();
    assert_eq!(
        nav,
        [
            ("general", "General"),
            ("library", "Library"),
            ("downloads", "Downloads"),
            ("integrations", "Integrations"),
            ("processing", "Server processing"),
            ("accounts", "Email & notifications"),
            ("autodownload", "Auto-download"),
            ("contribute", "Contribute"),
            ("blocked", "Blocked"),
            ("tasks", "Scheduled tasks"),
        ]
    );
    for (id, want) in
        [("apps", "downloads"), ("tagging", "library"), ("pipeline", "downloads"), ("cadenza", "processing"), ("nothing", "nothing")]
    {
        assert_eq!(schema.page_of(id), want, "?s={id}");
    }
    let with_tools: Vec<&str> = schema.sections.iter().map(|s| s.id.as_str()).filter(|id| schema.holds(id, "pipeline")).collect();
    assert_eq!(with_tools, ["downloads"]);
    assert!(When::parse("a=b").is_some());
}

#[test]
fn a_test_result_says_how_it_went() {
    let ok = render(|| view! { <TestResult result=json!({ "ok": true })/> });
    // settings/mod.rs `test_result`
    crate::testing::assert_same_html(&ok, &format!(r#"<div class="test-result ok">{}<span>Connected</span></div>"#, icon("check")));
    let err = render(|| view! { <TestResult result=json!({ "ok": false, "message": "refused <x>" })/> });
    crate::testing::assert_same_html(
        &err,
        &format!(
            r#"<div class="test-result err">{}<span>refused &lt;x&gt;</span>{}</div>"#,
            icon("x"),
            crate::details::copy_button_html("refused <x>")
        ),
    );
}
