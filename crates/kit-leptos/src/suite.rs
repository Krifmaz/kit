//! Suite on the web (`contracts/suite.md`): the shared sidebar of every linked app's sections, one search across
//! apps, "related in …" cross-links, the "Sign in with …" button and the admin's Linked apps page.
//!
//! - [`SuiteNav`]: the sidebar entries (in the person's order, hidden ones left out) with Edit sidebar: move up
//!   and down, hide and show, Reset order, Done (kept on the server for the person).
//! - [`SuiteSearch`]: a search field that asks every linked app at once, one group per app.
//! - [`Related`]: the hits other apps have for a text, under "Related in <app>".
//! - [`SsoButton`]: "Sign in with <app>" when an account source is linked, and the reason a sign-in failed
//!   ([`sso_error`]).
//! - [`LinkedApps`]: the admin's page: links (add by address and key, account source, check again, remove) and
//!   the keys this app made for others (make one, shown once with Copy; revoke).
//!
//! The server paths are the suite's fixed ones under the API prefix ([`PREFIX`], `/kit`); the words are
//! [`SuiteWords`] ([`set_words`]). Icons from other apps are SVG path data, drawn as a path attribute, never as
//! markup.

use crate::api;
use crate::clipboard::copy_text;
use crate::html::{opt, seq};
use crate::toasts::toast;
use kit_rules::app::fill;
use kit_rules::suite::{encode, move_item, prefs_of, toggle_hidden, SearchGroup, SidebarItem, SuiteWords};
use leptos::prelude::*;
use serde_json::{json, Value};
use std::cell::Cell;

/// The suite's paths under the API prefix: `/kit/suite/sidebar`, `/kit/sso` …
pub const PREFIX: &str = "/kit";

thread_local! {
    static WORDS: Cell<SuiteWords> = const { Cell::new(SuiteWords::DEFAULT) };
}

/// Use `w` for every suite word from here on.
pub fn set_words(w: SuiteWords) {
    WORDS.with(|c| c.set(w));
}

/// The suite's words now.
pub fn words() -> SuiteWords {
    WORDS.with(Cell::get)
}

/// An icon of the suite (SVG path data in a 24×24 box) as the kit draws its own icons.
pub fn path_icon(d: String) -> impl IntoView {
    view! {
        <svg class="suite-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d=d/>
        </svg>
    }
}

/// The entries of the sidebar while the person is not editing it: the visible ones, `active` (a path) marked.
pub fn nav_view(items: Vec<SidebarItem>, active: String) -> AnyView {
    seq(items.into_iter().filter(|i| !i.hidden).map(move |i| {
        let is_active = i.local && !active.is_empty() && i.path == active;
        let href = if i.local { i.path.clone() } else { i.url.clone() };
        let app = (!i.local).then(|| view! { <span class="suite-app">{i.app_name.clone()}</span> });
        view! {
            <a href=href class="suite-item" class:active=is_active data-suite=i.key.clone() title=i.title.clone()>
                {path_icon(i.icon.clone())}
                <span class="ellipsis">{i.title.clone()}</span>
                {opt(app)}
            </a>
        }
    }))
}

/// The answer of `GET /kit/suite/sidebar` as entries.
fn items_of(v: &Value) -> Vec<SidebarItem> {
    v.get("items").cloned().and_then(|i| serde_json::from_value(i).ok()).unwrap_or_default()
}

/// The shared sidebar: every visible entry, then Edit sidebar. Local entries link to their path (an app with
/// hash routes says `/#/things` in its manifest), the others to the other app.
#[component]
pub fn SuiteNav(
    /// The path of the page shown now, to mark its entry.
    #[prop(optional, into)]
    active: Signal<String>,
) -> impl IntoView {
    let w = words();
    let items: RwSignal<Vec<SidebarItem>> = RwSignal::new(Vec::new());
    let editing = RwSignal::new(false);
    let draft: RwSignal<Vec<SidebarItem>> = RwSignal::new(Vec::new());
    crate::rt::spawn(async move {
        if let Ok(v) = api::get(&format!("{PREFIX}/suite/sidebar")).await {
            items.set(items_of(&v));
        }
    });
    let save = move |body: Value| {
        crate::rt::spawn(async move {
            match api::put(&format!("{PREFIX}/suite/sidebar"), body).await {
                Ok(v) => {
                    items.set(items_of(&v));
                    editing.set(false);
                }
                Err(e) => toast(&e.message, "error"),
            }
        })
    };
    let edit_rows = move || {
        seq(draft.get().into_iter().enumerate().map(move |(n, i)| {
            let (k1, k2, k3) = (i.key.clone(), i.key.clone(), i.key.clone());
            let last = draft.with_untracked(|d| d.len()) - 1;
            let app = (!i.local).then(|| view! { <span class="suite-app">{i.app_name.clone()}</span> });
            view! {
                <div class="suite-edit-row" class:hidden=i.hidden data-suite=i.key.clone()>
                    {path_icon(i.icon.clone())}
                    <span class="grow ellipsis">{i.title.clone()}</span>
                    {opt(app)}
                    <button class="btn ghost small" title=w.move_up aria-label=w.move_up disabled=n == 0 on:click=move |_| draft.update(|d| move_item(d, &k1, -1))>"↑"</button>
                    <button class="btn ghost small" title=w.move_down aria-label=w.move_down disabled=n == last on:click=move |_| draft.update(|d| move_item(d, &k2, 1))>"↓"</button>
                    <button class="btn ghost small" on:click=move |_| draft.update(|d| toggle_hidden(d, &k3))>{if i.hidden { w.show } else { w.hide }}</button>
                </div>
            }
        }))
    };
    let body = move || {
        if editing.get() {
            view! {
                <div class="suite-edit">
                    {edit_rows}
                    <div class="row suite-edit-foot">
                        <button class="btn ghost small" on:click=move |_| save(json!({}))>{w.reset}</button>
                        <button class="btn primary small" on:click=move |_| save(serde_json::to_value(prefs_of(&draft.get_untracked())).unwrap_or_default())>{w.done}</button>
                    </div>
                </div>
            }
            .into_any()
        } else {
            let list = items.get();
            let any = !list.is_empty();
            view! {
                {nav_view(list, active.get())}
                <button class="btn ghost small suite-edit-btn" class:hidden=!any on:click=move |_| { draft.set(items.get_untracked()); editing.set(true); }>{w.edit}</button>
            }
            .into_any()
        }
    };
    view! { <div class="suite-nav" id="suite-nav">{body}</div> }
}

/// The groups of a search answer.
fn groups_of(v: &Value) -> Vec<SearchGroup> {
    v.get("groups").cloned().and_then(|g| serde_json::from_value(g).ok()).unwrap_or_default()
}

/// One search's groups: each app's icon and name (or `title`, filled with `{app}`), then its hits; a failed
/// app's reason in place of its hits.
pub fn groups_view(groups: Vec<SearchGroup>, title: &'static str) -> AnyView {
    seq(groups.into_iter().map(move |g| {
        let heading = fill(title, &[("app", &g.app_name)]);
        let hits = seq(g.results.into_iter().map(|h| {
            let sub = h.subtitle.clone().map(|s| view! { <div class="small muted ellipsis">{s}</div> });
            view! { <a class="list-item suite-hit" href=h.url.clone()><div class="grow" style="min-width:0"><div class="ellipsis">{h.title.clone()}</div>{opt(sub)}</div></a> }
        }));
        let err = g.error.clone().map(|e| view! { <div class="error-box small">{e}</div> });
        view! {
            <div class="suite-group" data-app=g.app.clone()>
                <div class="section-head row">{path_icon(g.icon.clone())}<h3>{heading}</h3></div>
                {opt(err)}
                <div class="card flat"><div class="list">{hits}</div></div>
            </div>
        }
    }))
}

/// The search across apps: a field, then one group per app that found something (or failed).
#[component]
pub fn SuiteSearch(
    /// The text to start with (searched at once when not empty).
    #[prop(optional, into)]
    initial: String,
) -> impl IntoView {
    let w = words();
    let text = RwSignal::new(initial.clone());
    let groups: RwSignal<Option<Vec<SearchGroup>>> = RwSignal::new(None);
    let busy = RwSignal::new(false);
    let seq_no = std::rc::Rc::new(Cell::new(0u32));
    let run = move |q: String, n: u32, current: std::rc::Rc<Cell<u32>>| {
        if q.trim().is_empty() {
            groups.set(None);
            return;
        }
        busy.set(true);
        crate::rt::spawn(async move {
            let r = api::get(&format!("{PREFIX}/suite/search?q={}", encode(q.trim()))).await;
            if current.get() != n {
                return;
            }
            busy.set(false);
            match r {
                Ok(v) => groups.set(Some(groups_of(&v))),
                Err(e) => toast(&e.message, "error"),
            }
        });
    };
    if !initial.trim().is_empty() {
        run(initial, 0, seq_no.clone());
    }
    let on_input = move |e: leptos::ev::Event| {
        let q = event_target_value(&e);
        text.set(q.clone());
        let n = seq_no.get() + 1;
        seq_no.set(n);
        let current = seq_no.clone();
        crate::rt::after(250, move || {
            if current.get() == n {
                run(q, n, current);
            }
        });
    };
    let results = move || match groups.get() {
        None if busy.get() => view! { <div class="empty"><span class="spinner">{""}</span></div> }.into_any(),
        None => "".into_any(),
        Some(g) if g.is_empty() => {
            view! { <div class="empty muted">{fill(w.no_results, &[("query", &text.get_untracked())])}</div> }.into_any()
        }
        Some(g) => groups_view(g, "{app}"),
    };
    view! {
        <div class="suite-search">
            <input class="input" type="search" id="suite-search" placeholder=w.search_placeholder prop:value=move || text.get() on:input=on_input/>
            <div class="suite-results">{results}</div>
        </div>
    }
}

/// What other apps have for `query`, under "Related in <app>"; nothing while they have nothing.
#[component]
pub fn Related(
    /// The text to look for (a title, a name).
    #[prop(into)]
    query: String,
    /// The most hits per app.
    #[prop(optional)]
    limit: Option<usize>,
) -> impl IntoView {
    let w = words();
    let groups: RwSignal<Vec<SearchGroup>> = RwSignal::new(Vec::new());
    let limit = limit.unwrap_or(5);
    if !query.trim().is_empty() {
        crate::rt::spawn(async move {
            if let Ok(v) = api::get(&format!("{PREFIX}/suite/search?scope=others&limit={limit}&q={}", encode(query.trim()))).await {
                groups.set(groups_of(&v).into_iter().filter(|g| !g.results.is_empty()).collect());
            }
        });
    }
    view! { <div class="suite-related">{move || groups_view(groups.get(), w.related_in)}</div> }
}

/// The reason a sign-in through the account source failed: the page address's `sso_error`, if any.
pub fn sso_error() -> Option<String> {
    let search = crate::rt::window()?.location().search().ok()?;
    sso_error_in(&search)
}

/// The `sso_error` of a query string (`?next=%2F&sso_error=…`), decoded.
pub fn sso_error_in(search: &str) -> Option<String> {
    let raw = search.trim_start_matches('?').split('&').find_map(|p| p.strip_prefix("sso_error="))?;
    let text = js_decode(&raw.replace('+', " "));
    (!text.is_empty()).then_some(text)
}

/// A percent-encoded text decoded (as `decodeURIComponent`; the text as it is when it can't be).
fn js_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

/// "Sign in with <app>" when people sign in here through an account source, and the reason the last try failed.
#[component]
pub fn SsoButton(
    /// Where to go after signing in (a path in this app).
    #[prop(optional, into)]
    next: Option<String>,
) -> impl IntoView {
    let w = words();
    let info: RwSignal<Option<Value>> = RwSignal::new(None);
    crate::rt::spawn(async move {
        if let Ok(v) = api::get(&format!("{PREFIX}/sso")).await {
            info.set(Some(v));
        }
    });
    let base = crate::config::get().paths.api.clone();
    let next = next.unwrap_or_else(|| "/".into());
    let error = sso_error().map(|e| view! { <div class="error-box" id="sso-error">{e}</div> });
    let button = move || {
        let v = info.get()?;
        if v.get("enabled").and_then(Value::as_bool) != Some(true) {
            return None;
        }
        let name = v.pointer("/source/name").and_then(Value::as_str).unwrap_or("").to_string();
        let icon = v.pointer("/source/icon").and_then(Value::as_str).unwrap_or("").to_string();
        let href = format!("{base}{PREFIX}/sso/start?next={}", encode(&next));
        Some(
            view! { <a class="btn suite-sso" id="sso-btn" href=href>{path_icon(icon)}<span>{fill(w.sign_in_with, &[("app", &name)])}</span></a> },
        )
    };
    view! { {opt(error)}{button} }
}

/// The value of a text field.
fn value(r: NodeRef<leptos::html::Input>) -> String {
    r.get().map(|i| i.value()).unwrap_or_default()
}

/// The admin's Linked apps page: the links, a form to link another app, and the keys this app made for others.
#[component]
pub fn LinkedApps() -> impl IntoView {
    let w = words();
    let links: RwSignal<Vec<Value>> = RwSignal::new(Vec::new());
    let keys: RwSignal<Vec<Value>> = RwSignal::new(Vec::new());
    let new_key: RwSignal<Option<String>> = RwSignal::new(None);
    let busy = RwSignal::new(false);
    let load = move || {
        crate::rt::spawn(async move {
            match api::get(&format!("{PREFIX}/suite/links")).await {
                Ok(v) => links.set(v.get("links").and_then(Value::as_array).cloned().unwrap_or_default()),
                Err(e) => toast(&e.message, "error"),
            }
            if let Ok(v) = api::get(&format!("{PREFIX}/suite/keys")).await {
                keys.set(v.get("keys").and_then(Value::as_array).cloned().unwrap_or_default());
            }
        })
    };
    load();
    let (url, key, name, redirect) = (NodeRef::new(), NodeRef::new(), NodeRef::new(), NodeRef::new());
    let source = NodeRef::<leptos::html::Input>::new();
    let add = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let body = json!({ "url": value(url), "key": value(key), "account_source": source.get().map(|i| i.checked()).unwrap_or(false) });
        busy.set(true);
        crate::rt::spawn(async move {
            match api::post(&format!("{PREFIX}/suite/links"), body).await {
                Ok(_) => {
                    for r in [url, key] {
                        if let Some(i) = r.get() {
                            i.set_value("");
                        }
                    }
                    load();
                }
                Err(e) => toast(&e.message, "error"),
            }
            busy.set(false);
        });
    };
    let make_key = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let body = json!({ "name": value(name), "redirect_url": value(redirect) });
        crate::rt::spawn(async move {
            match api::post(&format!("{PREFIX}/suite/keys"), body).await {
                Ok(v) => {
                    new_key.set(v.get("key").and_then(Value::as_str).map(str::to_string));
                    for r in [name, redirect] {
                        if let Some(i) = r.get() {
                            i.set_value("");
                        }
                    }
                    load();
                }
                Err(e) => toast(&e.message, "error"),
            }
        });
    };
    let link_rows = move || {
        let list = links.get();
        if list.is_empty() {
            return view! { <div class="empty muted">{w.no_links}</div> }.into_any();
        }
        seq(list.into_iter().map(move |l| {
            let id = l.get("id").and_then(Value::as_i64).unwrap_or(0);
            let name = l.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let ok = l.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let is_source = l.get("account_source").and_then(Value::as_bool).unwrap_or(false);
            let offers_sso = l.pointer("/manifest/sso").is_some_and(|s| !s.is_null());
            let error = l.get("error").and_then(Value::as_str).map(|e| view! { <div class="small" style="color:var(--err)">{e.to_string()}</div> });
            let icon = l.pointer("/manifest/icon").and_then(Value::as_str).unwrap_or("").to_string();
            let name2 = name.clone();
            let toggle = move |_| {
                crate::rt::spawn(async move {
                    if let Err(e) = api::patch(&format!("{PREFIX}/suite/links/{id}"), json!({ "account_source": !is_source })).await {
                        toast(&e.message, "error");
                    }
                    load();
                })
            };
            let refresh = move |_| {
                crate::rt::spawn(async move {
                    if let Err(e) = api::post_empty(&format!("{PREFIX}/suite/links/{id}/refresh")).await {
                        toast(&e.message, "error");
                    }
                    load();
                })
            };
            let remove = move |_| {
                let name = name2.clone();
                crate::rt::spawn(async move {
                    let q = kit_rules::dialogs::Confirm::new(w.remove_link).text(fill(w.remove_link_confirm, &[("app", &name)])).ok(w.remove_link).danger(true);
                    if crate::modal::confirm(q).await {
                        if let Err(e) = api::del(&format!("{PREFIX}/suite/links/{id}")).await {
                            toast(&e.message, "error");
                        }
                        load();
                    }
                })
            };
            let source_switch = offers_sso.then(|| {
                view! { <label class="row small suite-source"><input type="checkbox" prop:checked=is_source on:change=toggle/>{w.account_source}</label> }
            });
            view! {
                <div class="list-item suite-link" data-link=id>
                    {path_icon(icon)}
                    <div class="grow" style="min-width:0">
                        <div class="row"><b class="ellipsis">{name}</b><span class=if ok { "badge ok" } else { "badge err" }>{if ok { "OK" } else { "!" }}</span></div>
                        <div class="small mono faint ellipsis">{l.get("base_url").and_then(Value::as_str).unwrap_or("").to_string()}</div>
                        {opt(error)}
                        {opt(source_switch)}
                    </div>
                    <button class="btn ghost small" on:click=refresh>{w.refresh}</button>
                    <button class="btn ghost small danger" on:click=remove>{w.remove_link}</button>
                </div>
            }
        }))
    };
    let key_rows = move || {
        seq(keys.get().into_iter().map(move |k| {
            let id = k.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let name = k.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let redirect = k.get("redirect_url").and_then(Value::as_str).unwrap_or("").to_string();
            let (id2, name2) = (id.clone(), name.clone());
            let revoke = move |_| {
                let (id, name) = (id2.clone(), name2.clone());
                crate::rt::spawn(async move {
                    let q = kit_rules::dialogs::Confirm::new(w.revoke_key).text(fill(w.revoke_key_confirm, &[("app", &name)])).ok(w.revoke_key).danger(true);
                    if crate::modal::confirm(q).await {
                        if let Err(e) = api::del(&format!("{PREFIX}/suite/keys/{id}")).await {
                            toast(&e.message, "error");
                        }
                        load();
                    }
                })
            };
            view! {
                <div class="list-item suite-key" data-key=id.clone()>
                    <div class="grow" style="min-width:0"><b>{name}</b><div class="small mono faint ellipsis">{format!("{id} {redirect}")}</div></div>
                    <button class="btn ghost small danger" on:click=revoke>{w.revoke_key}</button>
                </div>
            }
        }))
    };
    let shown_key = move || {
        new_key.get().map(|k| {
            let k2 = k.clone();
            view! {
                <div class="card suite-new-key">
                    <div class="small muted">{w.key_once}</div>
                    <div class="row"><code class="code grow mono" id="suite-new-key">{k}</code><button class="btn small" on:click=move |_| copy_text(&k2)>"Copy"</button></div>
                </div>
            }
        })
    };
    view! {
        <div class="suite-admin">
            <div class="section-head"><h2>{w.linked_apps}</h2></div>
            <div class="card flat"><div class="list" id="suite-links">{link_rows}</div></div>
            <form class="card form-grid suite-add" on:submit=add>
                <div class="field"><label>{w.address}</label><input class="input" id="suite-url" node_ref=url placeholder="https://"/></div>
                <div class="field"><label>{w.api_key}</label><input class="input mono" id="suite-key" node_ref=key placeholder="ks_…"/></div>
                <label class="row small"><input type="checkbox" node_ref=source/>{w.account_source}" — "{w.account_source_help}</label>
                <div><button class="btn primary" type="submit" disabled=move || busy.get()>{w.add_link}</button></div>
            </form>
            <div class="section-head"><h2>{w.keys}</h2></div>
            <div class="small muted">{w.keys_help}</div>
            {shown_key}
            <div class="card flat"><div class="list" id="suite-keys">{key_rows}</div></div>
            <form class="card form-grid suite-add-key" on:submit=make_key>
                <div class="field"><label>{w.key_name}</label><input class="input" id="suite-key-name" node_ref=name/></div>
                <div class="field"><label>{w.key_redirect}</label><input class="input" id="suite-key-redirect" node_ref=redirect placeholder="https://"/></div>
                <div><button class="btn" type="submit">{w.new_key}</button></div>
            </form>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kit_rules::suite::SearchHit;

    fn item(key: &str, local: bool, hidden: bool) -> SidebarItem {
        let (app, section) = key.split_once('/').unwrap();
        SidebarItem {
            key: key.into(),
            app: app.into(),
            app_name: app.to_uppercase(),
            section: section.into(),
            title: section.to_uppercase(),
            icon: "M1 1h2\"><script>".into(),
            url: format!("https://{app}.test/{section}"),
            path: format!("/#/{section}"),
            hidden,
            local,
        }
    }

    #[test]
    fn the_nav_shows_visible_entries_and_names_other_apps() {
        let html =
            nav_view(vec![item("me/a", true, false), item("me/b", true, true), item("other/c", false, false)], "/#/a".into()).to_html();
        assert!(html.contains(r##"href="/#/a""##), "{html}");
        assert!(html.contains(r#"href="https://other.test/c""#));
        assert!(!html.contains("B</span>"), "hidden entries are left out: {html}");
        assert!(html.contains(r#"<span class="suite-app">OTHER</span>"#));
        assert_eq!(html.matches("active").count(), 1);
        // another app's icon is an attribute, never markup
        assert!(!html.contains("<script>") && html.contains("&quot;&gt;&lt;script&gt;"), "{html}");
    }

    #[test]
    fn search_groups_show_hits_and_reasons() {
        let g = vec![
            SearchGroup {
                app: "other".into(),
                app_name: "Other".into(),
                results: vec![SearchHit {
                    title: "Found".into(),
                    subtitle: Some("Where".into()),
                    url: "https://o.test/1".into(),
                    ..SearchHit::default()
                }],
                ..SearchGroup::default()
            },
            SearchGroup {
                app: "down".into(),
                app_name: "Down".into(),
                error: Some("Down can't be reached.".into()),
                ..SearchGroup::default()
            },
        ];
        let html = groups_view(g, SuiteWords::DEFAULT.related_in).to_html();
        assert!(html.contains("Related in Other") && html.contains("Related in Down"), "{html}");
        assert!(html.contains(r#"href="https://o.test/1""#) && html.contains("Where"));
        assert!(
            html.contains(r#"<div class="error-box small">Down can&#x27;t be reached.</div>"#) || html.contains("Down can't be reached."),
            "{html}"
        );
    }

    #[test]
    fn sign_in_errors_come_from_the_address() {
        assert_eq!(sso_error_in("?next=%2F&sso_error=The%20code%20is%20old.").as_deref(), Some("The code is old."));
        assert_eq!(sso_error_in("?sso_error=a+b%C3%A9").as_deref(), Some("a bé"));
        assert_eq!(sso_error_in("?next=%2F"), None);
        assert_eq!(sso_error_in("?sso_error="), None);
        assert_eq!(js_decode("100%"), "100%");
    }

    #[test]
    fn words_can_be_changed() {
        assert_eq!(words().edit, "Edit sidebar");
        set_words(SuiteWords { edit: "Arrange", ..SuiteWords::DEFAULT });
        assert_eq!(words().edit, "Arrange");
        set_words(SuiteWords::DEFAULT);
    }
}
