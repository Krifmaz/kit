//! The Diagnostics page: every connection, folder and tool check, what is wrong, the raw answers, where each
//! setting comes from, and Copy report.
//!
//! From Encore's `web-rs/src/settings/diagnostics.rs` (`render`, `run`, `check_item`, `copy_report`) and
//! `contracts/diagnostics.md` (`GET /api/diagnostics`). The grouping, words and report are kit-rules'
//! [`kit_rules::diagnostics`]. What Encore's page has of its own (the Updates card, the Server block, the download
//! folders, Backups, App crashes, the VPN row) comes in through [`DiagHooks`] and the page's children, so the kit
//! draws the shared part and Encore keeps its own.

use crate::api;
use crate::clipboard::copy_text;
use crate::html::{html, opt, seq};
use crate::icons::{chevron, icon};
use crate::json::J;
use kit_rules::diagnostics::{
    groups, http_status, intro, js_str, level, or_empty, problems, report_text, saved_in, sources_line, summary_line, truthy, whole_text,
    CHECKING, COPY_REPORT, RAW_RESPONSE, RUN_CHECKS, SOURCES_COLUMNS, SOURCES_TITLE, TITLE,
};
use leptos::prelude::*;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A unix time as the browser's locale writes it (`toLocaleString`), "—" for none. From Encore's `ui.rs`
/// `fmt::date`. Off the browser: the seconds as they are.
pub fn date(ts: f64) -> String {
    if ts <= 0.0 {
        return "—".into();
    }
    if crate::rt::IN_BROWSER {
        js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ts * 1000.0))
            .to_locale_string("default", &wasm_bindgen::JsValue::UNDEFINED)
            .into()
    } else {
        format!("{ts}")
    }
}

/// A part of the page drawn by the app from the diagnostics answer.
pub type Section = Arc<dyn Fn(&Value) -> AnyView + Send + Sync>;

/// The app's sections of the copied report, each a list of lines.
pub type ReportLines = Arc<dyn Fn(&Value) -> Vec<Vec<String>> + Send + Sync>;
/// Changes the diagnostics answer before it is drawn.
pub type Adjust = Arc<dyn Fn(Value) -> Pin<Box<dyn Future<Output = Value>>> + Send + Sync>;

/// Draws one check's detail line (Encore words a Soulseek session its own way).
pub type Detail = Arc<dyn Fn(&Value) -> String + Send + Sync>;
/// Runs with the checks' answer once it is drawn.
pub type Loaded = Arc<dyn Fn(&Value) + Send + Sync>;

/// What the app adds to the page.
#[derive(Clone, Default)]
pub struct DiagHooks {
    /// Drawn between the summary and the check groups (Encore's Server block).
    pub after_summary: Option<Section>,
    /// Drawn between the check groups and the settings (Encore's Download folders).
    pub after_checks: Option<Section>,
    /// More lines for one check, under its detail (Encore's Soulseek session).
    pub check_extra: Option<Section>,
    /// A check's detail line in place of its `detail` (Encore's Soulseek session in words).
    pub check_detail: Option<Detail>,
    /// The app's sections of the copied report, before the settings (Encore's download folders and Server).
    pub report: Option<ReportLines>,
    /// Copy report done by the app itself, with the last answer.
    pub copy: Option<Loaded>,
    /// Changes the answer before it is drawn (Encore adds its VPN row from `/system/vpn`).
    pub adjust: Option<Adjust>,
    /// Runs once an answer is drawn (Encore remembers the machine block for its Settings banner).
    pub loaded: Option<Loaded>,
    /// A failed run goes to the console and the page keeps "Checking…" (Encore's page), instead of an error box.
    pub quiet_errors: bool,
}

/// One `.list-item` of a check group, with the line breaks of Encore's template. From `diagnostics.rs`
/// `check_item`.
pub fn check_item(c: &Value, hooks: &DiagHooks) -> impl IntoView {
    let ok = truthy(c, "ok");
    let okc = if ok { "ok" } else { "err" };
    let detail = hooks.check_detail.as_ref().map(|f| f(c)).unwrap_or_else(|| or_empty(c, "detail"));
    let status = http_status(c).map(|s| view! { <span class=format!("badge {okc}")>{format!("HTTP {s}")}</span> });
    let ms = c.get("ms").filter(|m| !m.is_null()).map(|_| view! { <span class="small faint">{format!("{} ms", js_str(c, "ms"))}</span> });
    let body = truthy(c, "body").then(|| {
        view! { <details><summary {..()} class="small muted" style="cursor:pointer">{RAW_RESPONSE}</summary><div class="code diag-raw">{or_empty(c, "body")}</div></details> }
    });
    // the cut-off request line opens the whole check, to read and copy
    let (title, whole) = if truthy(c, "request") { (Some(c.s("name")), Some(whole_text(c, &detail))) } else { (None, None) };
    let (t2, w2) = (title.clone().unwrap_or_default(), whole.clone());
    let open = move |e: web_sys::MouseEvent| {
        if let Some(w) = &w2 {
            if !crate::details::selecting() {
                e.stop_propagation();
                crate::details::show(&t2, w);
            }
        }
    };
    let fix = c.get("fix").and_then(Value::as_str).filter(|f| !f.is_empty()).map(|f| view! { <div class="small">{f.to_string()}</div> });
    let extra = hooks.check_extra.as_ref().map(|f| f(c));
    view! {
        {"\n          "}
        <div {..()} class="list-item" style="align-items:flex-start">
            {"\n            "}
            <span {..()} class=format!("dot {}", level(c)) style="margin-top:7px">{""}</span>
            {"\n            "}
            <div class="grow">
                {"\n              "}
                <div class="row wrap"><b>{c.s("name")}</b>{opt(status)}{opt(ms)}</div>
                {"\n              "}
                <div class="small mono faint ellipsis" data-details=whole data-details-title=title on:click=open>{or_empty(c, "request")}</div>
                {"\n              "}
                <div {..()} class="small" style=format!("color:{}", if ok { "var(--muted)" } else { "var(--err)" })>{detail.clone()}</div>
                {opt(fix)}
                {"\n              "}
                {opt(extra)}
                {"\n              "}
                {opt(body)}
                {"\n            "}
            </div>
            {"\n          "}
        </div>
    }
}

/// The checks' answer drawn: the summary, the groups, the settings table, with the line breaks of Encore's
/// template. From `diagnostics.rs` `run`.
#[component]
pub fn DiagnosticsView(
    /// The app's name ("Encore 0.36.4 · running as uid …").
    #[prop(into)]
    app: String,
    /// The answer of `GET /api/diagnostics`.
    data: Value,
    /// What the app adds.
    #[prop(optional)]
    hooks: DiagHooks,
    /// "Where settings come from" starts unfolded (it was before Run checks).
    #[prop(optional, into)]
    sources_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let checks = data.a("checks");
    let bad = kit_rules::diagnostics::bad_count(&checks);
    let group_views = seq(groups(&checks).into_iter().map(|(title, g)| {
        let items = seq(g.into_iter().map(|c| check_item(c, &hooks)));
        view! {
            {"\n        "}<div class="section-head"><h2>{title}</h2></div>
            {"\n        "}<div class="card flat"><div class="list">{items}</div></div>
        }
    }));
    let settings = data.a("settings");
    let rows = seq(settings.iter().map(|s| {
        let source = if s.get("source").and_then(Value::as_str) == Some("env") {
            view! { <span class="badge info">{format!("env {}", s.s("env"))}</span> }.into_any()
        } else {
            view! { <span class="badge">{saved_in(&app)}</span> }.into_any()
        };
        view! {
            <tr><td class="mono small">{s.s("key")}</td>
            {"\n          "}<td>{source}</td>
            {"\n          "}<td {..()} class="small mono ellipsis" style="max-width:360px">{js_str(s, "value")}</td></tr>
        }
    }));
    let open = sources_open.unwrap_or_else(|| RwSignal::new(false));
    let (after_summary, after_checks) = (hooks.after_summary.as_ref().map(|f| f(&data)), hooks.after_checks.as_ref().map(|f| f(&data)));
    let [c1, c2, c3] = SOURCES_COLUMNS;
    view! {
        {"\n      "}
        <div {..()} class="card row" style="margin-bottom:16px">
            {"\n        "}
            <span class={if bad > 0 { "badge err" } else { "badge ok" }}>{problems(bad)}</span>
            {"\n        "}
            <span class="muted small">{summary_line(&app, &data.s("version"), &js_str(&data, "uid"), &date(data.f("time")))}</span>
            {"\n      "}
        </div>
        {"\n      "}{opt(after_summary)}
        {"\n      "}{group_views}
        {"\n      "}{opt(after_checks)}
        {"\n      "}
        <details {..()} class="card settings-card diag-sources" style="margin-top:22px" open=open.get_untracked()
            on:toggle=move |e| open.set(e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok()).is_some_and(|d| d.has_attribute("open")))>
            {"\n        "}
            <summary {..()} class="row" style="justify-content:space-between"><div><h2 {..()} style="margin:0">{SOURCES_TITLE}</h2><div class="small muted">{sources_line(&app, settings.len())}</div></div><span class="chev" {..html(chevron())}>{""}</span></summary>
            {"\n        "}
            <div {..()} class="table-wrap" style="margin-top:12px"><table class="table"><thead><tr><th>{c1}</th><th>{c2}</th><th>{c3}</th></tr></thead><tbody>
                {"\n        "}{rows}{"\n      "}
            </tbody></table></div>
        </details>
    }
}

/// The Diagnostics page: its heading, Copy report and Run checks, then the checks (run at once). `children` are
/// drawn after the checks (Encore's Backups and App crashes, as an HTML fragment of its own); `top` in
/// `#updates`. The line breaks are Encore's. From `diagnostics.rs` `render` and `run`.
#[component]
pub fn DiagnosticsPage(
    /// The app's name.
    #[prop(into)]
    app: String,
    /// What the app adds.
    #[prop(optional)]
    hooks: DiagHooks,
    /// Drawn in `#updates`.
    #[prop(optional)]
    top: Option<AnyView>,
    /// Drawn after the checks.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let last: RwSignal<Option<Value>> = RwSignal::new(None);
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let running = RwSignal::new(false);
    let sources_open = RwSignal::new(false);
    let (adjust, loaded, quiet) = (hooks.adjust.clone(), hooks.loaded.clone(), hooks.quiet_errors);
    let run = move || {
        running.set(true);
        let (adjust, loaded) = (adjust.clone(), loaded.clone());
        crate::rt::spawn(async move {
            match api::get("/diagnostics").await {
                Ok(mut d) => {
                    if let Some(f) = &adjust {
                        d = f(d).await;
                    }
                    error.set(None);
                    if let Some(f) = &loaded {
                        f(&d);
                    }
                    last.set(Some(d));
                    running.set(false);
                }
                Err(e) if quiet => web_sys::console::error_1(&e.message.into()),
                Err(e) => {
                    error.set(Some(e.message));
                    running.set(false);
                }
            }
        });
    };
    run();
    let (report, own_copy) = (hooks.report.clone(), hooks.copy.clone());
    let app2 = app.clone();
    let copy = move |_| {
        if let Some(d) = last.get_untracked() {
            if let Some(f) = &own_copy {
                return f(&d);
            }
            let extra = report.as_ref().map(|f| f(&d)).unwrap_or_default();
            copy_text(&report_text(&app2, &d, &extra));
        }
    };
    let app3 = app.clone();
    let diag = move || {
        if running.get() {
            return view! { <div class="empty"><span class="spinner">{""}</span><p class="muted">{CHECKING}</p></div> }.into_any();
        }
        if let Some(e) = error.get() {
            return view! { <div class="error-box">{e}</div> }.into_any();
        }
        match last.get() {
            Some(d) => view! { <DiagnosticsView app=app3.clone() data=d hooks=hooks.clone() sources_open=sources_open/> }.into_any(),
            None => view! { <div class="empty"><span class="spinner">{""}</span></div> }.into_any(),
        }
    };
    view! {
        {"\n    "}
        <div {..()} class="row wrap between" style="margin-bottom:18px">
            {"\n      "}
            <div><h1>{TITLE}</h1><div class="small muted">{intro(&app)}</div></div>
            {"\n      "}
            <div class="row">
                <button class="btn" id="copy" disabled=move || last.get().is_none() on:click={copy} {..html(format!("{}{COPY_REPORT}", icon("copy")))}>{""}</button>
                <button class="btn primary" id="run" on:click=move |_| run() {..html(format!("{}{RUN_CHECKS}", icon("refresh")))}>{""}</button>
            </div>
            {"\n    "}
        </div>
        {"\n    "}
        <div id="updates">{opt(top)}</div>
        {"\n    "}
        <div id="diag">{diag}</div>
        {children.map(|c| c()).unwrap_or_else(|| "".into_any())}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::esc;
    use crate::testing::{assert_same_html, render};
    use serde_json::json;

    // ---- Encore's `run` and `check_item` (diagnostics.rs), without its own blocks ---------------------------

    fn esck(v: &Value, k: &str) -> String {
        esc(&v.s(k))
    }
    fn encore_check_item(c: &Value) -> String {
        let ok = c.truthy("ok");
        let detail = or_empty(c, "detail");
        let okc = if ok { "ok" } else { "err" };
        let status =
            if c.truthy("status") { format!(r#"<span class="badge {okc}">HTTP {}</span>"#, js_str(c, "status")) } else { String::new() };
        let ms = if !c.is_nullish("ms") { format!(r#"<span class="small faint">{} ms</span>"#, js_str(c, "ms")) } else { String::new() };
        let body = if c.truthy("body") {
            format!(
                r#"<details><summary class="small muted" style="cursor:pointer">Raw response</summary><div class="code diag-raw">{}</div></details>"#,
                esck(c, "body")
            )
        } else {
            String::new()
        };
        format!(
            r#"
          <div class="list-item" style="align-items:flex-start">
            <span class="dot {okc}" style="margin-top:7px"></span>
            <div class="grow">
              <div class="row wrap"><b>{name}</b>{status}{ms}</div>
              <div class="small mono faint ellipsis"{whole}>{request}</div>
              <div class="small" style="color:{color}">{detail}</div>
              {body}
            </div>
          </div>"#,
            name = esck(c, "name"),
            whole = if c.truthy("request") {
                crate::details::attrs(
                    &c.s("name"),
                    &[c.s("name"), or_empty(c, "request"), detail.clone(), or_empty(c, "body")]
                        .iter()
                        .filter(|x| !x.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            } else {
                String::new()
            },
            request = esc(&or_empty(c, "request")),
            color = if ok { "var(--muted)" } else { "var(--err)" },
            detail = esc(&detail),
        )
    }
    fn encore_run(data: &Value) -> String {
        const KIND: [(&str, &str); 3] = [("service", "Connections"), ("path", "Folders & permissions"), ("tool", "Tools in the container")];
        let checks = data.a("checks");
        let bad = checks.iter().filter(|c| !c.truthy("ok")).count();
        let groups: String = KIND
            .iter()
            .map(|(k, title)| {
                let g: Vec<&Value> = checks.iter().filter(|c| c.s("kind") == *k).collect();
                if g.is_empty() {
                    return String::new();
                }
                let items: String = g.iter().map(|c| encore_check_item(c)).collect();
                format!(
                    r#"
        <div class="section-head"><h2>{title}</h2></div>
        <div class="card flat"><div class="list">{items}</div></div>"#
                )
            })
            .collect();
        let rows: String = data
            .a("settings")
            .iter()
            .map(|s| {
                format!(
                    r#"<tr><td class="mono small">{key}</td>
          <td>{source}</td>
          <td class="small mono ellipsis" style="max-width:360px">{value}</td></tr>"#,
                    key = esck(s, "key"),
                    source = if s.g("source").as_str() == Some("env") {
                        format!(r#"<span class="badge info">env {}</span>"#, esck(s, "env"))
                    } else {
                        r#"<span class="badge">saved in Encore</span>"#.to_string()
                    },
                    value = esc(&js_str(s, "value")),
                )
            })
            .collect();
        format!(
            r#"
      <div class="card row" style="margin-bottom:16px">
        <span class="badge {badc}">{badl}</span>
        <span class="muted small">Encore {version} · running as uid {uid} · {time}</span>
      </div>
      {groups}
      <details class="card settings-card diag-sources" style="margin-top:22px" {open}>
        <summary class="row" style="justify-content:space-between"><div><h2 style="margin:0">Where settings come from</h2><div class="small muted">Environment variables always win over values saved in Encore · {count} settings</div></div><span class="chev">{chev}</span></summary>
        <div class="table-wrap" style="margin-top:12px"><table class="table"><thead><tr><th>Setting</th><th>Source</th><th>Value</th></tr></thead><tbody>
        {rows}
      </tbody></table></div></details>"#,
            count = data.a("settings").len(),
            open = "",
            chev = chevron(),
            badc = if bad > 0 { "err" } else { "ok" },
            badl = if bad > 0 { format!("{bad} problem{}", if bad > 1 { "s" } else { "" }) } else { "All good".to_string() },
            version = esck(data, "version"),
            uid = js_str(data, "uid"),
            time = date(data.f("time")),
        )
    }

    fn answer() -> Value {
        json!({
            "version": "0.36.4", "uid": 1000, "time": 1_793_800_000,
            "checks": [
                { "name": "MusicBrainz", "kind": "service", "request": "GET https://musicbrainz.org/ws/2/?fmt=json", "ok": true, "detail": "200 OK", "status": 200, "ms": 180 },
                { "name": "Prowlarr", "kind": "service", "request": "GET http://prowlarr:9696/api", "ok": false, "detail": "connection refused <tcp>", "status": 502, "body": "<html>Bad gateway</html>" },
                { "name": "Music library", "kind": "path", "ok": true, "detail": "/music · writable" },
                { "name": "ffmpeg", "kind": "tool", "ok": true, "detail": "6.1", "ms": null }
            ],
            "settings": [
                { "key": "library_path", "value": "/music", "source": "env", "env": "ENCORE_LIBRARY_PATH" },
                { "key": "app_name", "value": null, "source": "ui" },
                { "key": "session_days" , "source": "default" }
            ]
        })
    }

    #[test]
    fn the_checks_are_encores() {
        let d = answer();
        let html = render(|| view! { <DiagnosticsView app="Encore" data=d.clone()/> });
        assert_same_html(&html, &encore_run(&answer()));
        // every check passed
        let good = json!({ "version": "1", "uid": 0, "checks": [{ "name": "a", "kind": "tool", "ok": true }], "settings": [] });
        let html = render(|| view! { <DiagnosticsView app="Encore" data=good.clone()/> });
        assert_same_html(&html, &encore_run(&good));
    }

    #[test]
    fn the_head_is_encores() {
        let html = render(|| view! { <DiagnosticsPage app="Encore"/> });
        let head = format!(
            r#"<div class="row wrap between" style="margin-bottom:18px">
      <div><h1>Diagnostics</h1><div class="small muted">Runs every connection, folder and tool check from inside the Encore container and shows the raw answers. Keys are hidden.</div></div>
      <div class="row"><button class="btn" id="copy" disabled>{copy}Copy report</button><button class="btn primary" id="run">{refresh}Run checks</button></div>
    </div>
    <div id="updates"></div>
    <div id="diag"><div class="empty"><span class="spinner"></span><p class="muted">Checking…</p></div></div>"#,
            copy = icon("copy"),
            refresh = icon("refresh")
        );
        assert_same_html(&html, &head);
    }

    #[test]
    fn a_kit_server_may_say_warn_and_how_to_fix_it() {
        let c = json!({ "name": "Disk", "kind": "path", "ok": true, "status": "warn", "detail": "92% full", "fix": "Free some space" });
        let html = render(|| check_item(&c, &DiagHooks::default()));
        assert!(html.contains("dot warn") && html.contains("Free some space") && !html.contains("HTTP"), "{html}");
    }
}
