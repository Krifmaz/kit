//! The Diagnostics page: every check the server runs (connections, folders, tools and the app's own kinds), what
//! is wrong, the raw answers, where each setting comes from, and Copy report.
//!
//! The checks come from the configured [`diagnostics`](crate::config::Paths::diagnostics) path
//! (`contracts/diagnostics.md`); the kit's three kinds are grouped first, then any kind the app's server adds, in
//! the order it sends them. The words are the configured [`DiagWords`]; the app adds parts of its own through
//! [`DiagHooks`].

use crate::api;
use crate::clipboard::copy_text;
use crate::html::{html, opt, seq};
use crate::icons::{chevron, icon};
use crate::json::J;
use kit_rules::diagnostics::{
    groups_with, http_status, js_str, level, or_empty, problems, report_text, saved_in, sources_line, summary_line, truthy, whole_text,
    DiagWords,
};
use leptos::prelude::*;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// A unix time as the browser's locale writes it (`toLocaleString`), "—" for none.
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

/// Draws one check's detail line (an app words a check of its own its own way).
pub type Detail = Arc<dyn Fn(&Value) -> String + Send + Sync>;
/// Runs with the checks' answer once it is drawn.
pub type Loaded = Arc<dyn Fn(&Value) + Send + Sync>;

/// What the app adds to the page.
#[derive(Clone, Default)]
pub struct DiagHooks {
    /// Drawn between the summary and the check groups (a block about the server).
    pub after_summary: Option<Section>,
    /// Drawn between the check groups and the settings (a table of the app's own).
    pub after_checks: Option<Section>,
    /// More lines for one check, under its detail.
    pub check_extra: Option<Section>,
    /// A check's detail line in place of its `detail`.
    pub check_detail: Option<Detail>,
    /// The app's sections of the copied report, before the settings.
    pub report: Option<ReportLines>,
    /// Copy report done by the app itself, with the last answer.
    pub copy: Option<Loaded>,
    /// Changes the answer before it is drawn (adds a check the app asks for elsewhere).
    pub adjust: Option<Adjust>,
    /// Runs once an answer is drawn (the app remembers part of it).
    pub loaded: Option<Loaded>,
    /// A failed run goes to the console and the page keeps "Checking…", instead of an error box.
    pub quiet_errors: bool,
}

fn words() -> DiagWords {
    crate::config::get().words.diag
}

/// One `.list-item` of a check group.
pub fn check_item(c: &Value, hooks: &DiagHooks) -> impl IntoView {
    let raw = words().raw_response;
    let ok = truthy(c, "ok");
    let okc = if ok { "ok" } else { "err" };
    let detail = hooks.check_detail.as_ref().map(|f| f(c)).unwrap_or_else(|| or_empty(c, "detail"));
    let status = http_status(c).map(|s| view! { <span class=format!("badge {okc}")>{format!("HTTP {s}")}</span> });
    let ms = c.get("ms").filter(|m| !m.is_null()).map(|_| view! { <span class="small faint">{format!("{} ms", js_str(c, "ms"))}</span> });
    let body = truthy(c, "body").then(|| {
        view! { <details><summary {..()} class="small muted" style="cursor:pointer">{raw}</summary><div class="code diag-raw">{or_empty(c, "body")}</div></details> }
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
        <div {..()} class="list-item" style="align-items:flex-start">
            <span {..()} class=format!("dot {}", level(c)) style="margin-top:7px">{""}</span>
            <div class="grow">
                <div class="row wrap"><b>{c.s("name")}</b>{opt(status)}{opt(ms)}</div>
                <div class="small mono faint ellipsis" data-details=whole data-details-title=title on:click=open>{or_empty(c, "request")}</div>
                <div {..()} class="small" style=format!("color:{}", if ok { "var(--muted)" } else { "var(--err)" })>{detail.clone()}</div>
                {opt(fix)}
                {opt(extra)}
                {opt(body)}
            </div>
        </div>
    }
}

/// The checks' answer drawn: the summary, the groups, the settings table.
#[component]
pub fn DiagnosticsView(
    /// The app's name ("Kit Demo 1.4.0 · running as uid …"); the configured name when left out.
    #[prop(optional, into)]
    app: Option<String>,
    /// The answer of `GET /api/diagnostics`.
    data: Value,
    /// What the app adds.
    #[prop(optional)]
    hooks: DiagHooks,
    /// "Where settings come from" starts unfolded (it was before Run checks).
    #[prop(optional, into)]
    sources_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let w = words();
    let app = app.unwrap_or_else(crate::config::app_name);
    let checks = data.a("checks");
    let bad = kit_rules::diagnostics::bad_count(&checks);
    let group_views = seq(groups_with(&checks, &w.kinds).into_iter().map(|(title, g)| {
        let items = seq(g.into_iter().map(|c| check_item(c, &hooks)));
        view! {
            <div class="section-head"><h2>{title}</h2></div>
            <div class="card flat"><div class="list">{items}</div></div>
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
            <tr><td class="mono small">{s.s("key")}</td><td>{source}</td><td {..()} class="small mono ellipsis" style="max-width:360px">{js_str(s, "value")}</td></tr>
        }
    }));
    let open = sources_open.unwrap_or_else(|| RwSignal::new(false));
    let (after_summary, after_checks) = (hooks.after_summary.as_ref().map(|f| f(&data)), hooks.after_checks.as_ref().map(|f| f(&data)));
    let [c1, c2, c3] = w.sources_columns;
    let badge = if bad == 0 { w.all_good.to_string() } else { problems(bad) };
    view! {
        <div {..()} class="card row" style="margin-bottom:16px">
            <span class={if bad > 0 { "badge err" } else { "badge ok" }}>{badge}</span>
            <span class="muted small">{summary_line(&app, &data.s("version"), &js_str(&data, "uid"), &date(data.f("time")))}</span>
        </div>
        {opt(after_summary)}
        {group_views}
        {opt(after_checks)}
        <details {..()} class="card settings-card diag-sources" style="margin-top:22px" open=open.get_untracked()
            on:toggle=move |e| open.set(e.target().and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok()).is_some_and(|d| d.has_attribute("open")))>
            <summary {..()} class="row" style="justify-content:space-between"><div><h2 {..()} style="margin:0">{w.sources_title}</h2><div class="small muted">{sources_line(&app, settings.len())}</div></div><span class="chev" {..html(chevron())}>{""}</span></summary>
            <div {..()} class="table-wrap" style="margin-top:12px"><table class="table"><thead><tr><th>{c1}</th><th>{c2}</th><th>{c3}</th></tr></thead><tbody>
                {rows}
            </tbody></table></div>
        </details>
    }
}

/// The Diagnostics page: its heading, Copy report and Run checks, then the checks (run at once). `children` are
/// drawn after the checks (the app's own parts); `top` in `#updates`.
#[component]
pub fn DiagnosticsPage(
    /// The app's name; the configured name when left out.
    #[prop(optional, into)]
    app: Option<String>,
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
    let w = words();
    let app = app.unwrap_or_else(crate::config::app_name);
    let path = crate::config::get().paths.diagnostics.clone();
    let last: RwSignal<Option<Value>> = RwSignal::new(None);
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let running = RwSignal::new(false);
    let sources_open = RwSignal::new(false);
    let (adjust, loaded, quiet) = (hooks.adjust.clone(), hooks.loaded.clone(), hooks.quiet_errors);
    let run = move || {
        running.set(true);
        let (adjust, loaded, path) = (adjust.clone(), loaded.clone(), path.clone());
        crate::rt::spawn(async move {
            match api::get(&path).await {
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
            return view! { <div class="empty"><span class="spinner">{""}</span><p class="muted">{w.checking}</p></div> }.into_any();
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
        <div {..()} class="row wrap between" style="margin-bottom:18px">
            <div><h1>{w.title}</h1><div class="small muted">{w.intro(&app)}</div></div>
            <div class="row">
                <button class="btn" id="copy" disabled=move || last.get().is_none() on:click={copy} {..html(format!("{}{}", icon("copy"), w.copy_report))}>{""}</button>
                <button class="btn primary" id="run" on:click=move |_| run() {..html(format!("{}{}", icon("refresh"), w.run_checks))}>{""}</button>
            </div>
        </div>
        <div id="updates">{opt(top)}</div>
        <div id="diag">{diag}</div>
        {children.map(|c| c()).unwrap_or_else(|| "".into_any())}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};
    use serde_json::json;

    fn answer() -> Value {
        json!({
            "version": "1.4.0", "uid": 1000, "time": 1_793_800_000,
            "checks": [
                { "name": "Mail server", "kind": "service", "request": "EHLO mail.example.org:587", "ok": true, "detail": "250 OK", "status": 200, "ms": 180 },
                { "name": "Search index", "kind": "service", "request": "GET http://search:7700/health", "ok": false, "detail": "connection refused <tcp>", "status": 502, "body": "<html>Bad gateway</html>" },
                { "name": "Job queue", "kind": "queue", "ok": true, "detail": "0 waiting" },
                { "name": "Data folder", "kind": "path", "ok": true, "detail": "/data · writable" },
                { "name": "convert", "kind": "tool", "ok": true, "detail": "7.1", "ms": null }
            ],
            "settings": [
                { "key": "data_dir", "value": "/data", "source": "env", "env": "APP_DATA_DIR" },
                { "key": "app_name", "value": null, "source": "ui" }
            ]
        })
    }

    #[test]
    fn the_checks_are_grouped_by_kind_with_the_apps_own_last() {
        let d = answer();
        let html = render(|| view! { <DiagnosticsView app="Kit Demo" data=d.clone()/> });
        let refused = crate::details::attrs(
            "Search index",
            "Search index\nGET http://search:7700/health\nconnection refused <tcp>\n<html>Bad gateway</html>",
        );
        let mail = crate::details::attrs("Mail server", "Mail server\nEHLO mail.example.org:587\n250 OK");
        let want = format!(
            r#"<div class="card row" style="margin-bottom:16px"><span class="badge err">1 problem</span><span class="muted small">Kit Demo 1.4.0 · running as uid 1000 · {time}</span></div>
            <div class="section-head"><h2>Connections</h2></div>
            <div class="card flat"><div class="list">
              <div class="list-item" style="align-items:flex-start"><span class="dot ok" style="margin-top:7px"></span><div class="grow">
                <div class="row wrap"><b>Mail server</b><span class="badge ok">HTTP 200</span><span class="small faint">180 ms</span></div>
                <div class="small mono faint ellipsis"{mail}>EHLO mail.example.org:587</div>
                <div class="small" style="color:var(--muted)">250 OK</div></div></div>
              <div class="list-item" style="align-items:flex-start"><span class="dot err" style="margin-top:7px"></span><div class="grow">
                <div class="row wrap"><b>Search index</b><span class="badge err">HTTP 502</span></div>
                <div class="small mono faint ellipsis"{refused}>GET http://search:7700/health</div>
                <div class="small" style="color:var(--err)">connection refused &lt;tcp&gt;</div>
                <details><summary class="small muted" style="cursor:pointer">Raw response</summary><div class="code diag-raw">&lt;html&gt;Bad gateway&lt;/html&gt;</div></details></div></div>
            </div></div>
            <div class="section-head"><h2>Folders &amp; permissions</h2></div>
            <div class="card flat"><div class="list"><div class="list-item" style="align-items:flex-start"><span class="dot ok" style="margin-top:7px"></span><div class="grow">
              <div class="row wrap"><b>Data folder</b></div><div class="small mono faint ellipsis"></div><div class="small" style="color:var(--muted)">/data · writable</div></div></div></div></div>
            <div class="section-head"><h2>Tools in the container</h2></div>
            <div class="card flat"><div class="list"><div class="list-item" style="align-items:flex-start"><span class="dot ok" style="margin-top:7px"></span><div class="grow">
              <div class="row wrap"><b>convert</b></div><div class="small mono faint ellipsis"></div><div class="small" style="color:var(--muted)">7.1</div></div></div></div></div>
            <div class="section-head"><h2>Queue</h2></div>
            <div class="card flat"><div class="list"><div class="list-item" style="align-items:flex-start"><span class="dot ok" style="margin-top:7px"></span><div class="grow">
              <div class="row wrap"><b>Job queue</b></div><div class="small mono faint ellipsis"></div><div class="small" style="color:var(--muted)">0 waiting</div></div></div></div></div>
            <details class="card settings-card diag-sources" style="margin-top:22px">
              <summary class="row" style="justify-content:space-between"><div><h2 style="margin:0">Where settings come from</h2><div class="small muted">Environment variables always win over values saved in Kit Demo · 2 settings</div></div><span class="chev">{chev}</span></summary>
              <div class="table-wrap" style="margin-top:12px"><table class="table"><thead><tr><th>Setting</th><th>Source</th><th>Value</th></tr></thead><tbody>
                <tr><td class="mono small">data_dir</td><td><span class="badge info">env APP_DATA_DIR</span></td><td class="small mono ellipsis" style="max-width:360px">/data</td></tr>
                <tr><td class="mono small">app_name</td><td><span class="badge">saved in Kit Demo</span></td><td class="small mono ellipsis" style="max-width:360px">null</td></tr>
              </tbody></table></div>
            </details>"#,
            time = date(1_793_800_000.0),
            chev = chevron(),
        );
        assert_same_html(&html, &want);
        // every check passed
        let good = json!({ "version": "1", "uid": 0, "checks": [{ "name": "a", "kind": "tool", "ok": true }], "settings": [] });
        let html = render(|| view! { <DiagnosticsView data=good.clone()/> });
        assert!(html.contains(r#"<span class="badge ok">All good</span>"#) && html.contains("App 1 · running as uid 0"), "{html}");
    }

    #[test]
    fn the_head_runs_the_checks_at_once() {
        let html = render(|| view! { <DiagnosticsPage app="Kit Demo"/> });
        let head = format!(
            r#"<div class="row wrap between" style="margin-bottom:18px">
              <div><h1>Diagnostics</h1><div class="small muted">Runs every check from inside the Kit Demo server and shows the raw answers. Keys are hidden.</div></div>
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
    fn a_check_may_say_warn_and_how_to_fix_it() {
        let c = json!({ "name": "Disk", "kind": "path", "ok": true, "status": "warn", "detail": "92% full", "fix": "Free some space" });
        let html = render(|| check_item(&c, &DiagHooks::default()));
        assert!(html.contains("dot warn") && html.contains("Free some space") && !html.contains("HTTP"), "{html}");
    }

    #[test]
    fn an_app_names_its_groups_and_words() {
        let mut cfg = crate::config::AppConfig::default();
        cfg.words.diag.title = "Diagnose";
        cfg.words.diag.kinds = [("service", "Dienste"), ("path", "Ordner"), ("tool", "Werkzeuge")];
        crate::config::set(cfg);
        let d = answer();
        let html = render(|| view! { <DiagnosticsView data=d.clone()/> });
        assert!(html.contains("<h2>Dienste</h2>") && html.contains("<h2>Ordner</h2>") && html.contains("<h2>Queue</h2>"), "{html}");
        assert!(render(|| view! { <DiagnosticsPage/> }).contains("<h1>Diagnose</h1>"));
        crate::config::set(crate::config::AppConfig::default());
    }
}
