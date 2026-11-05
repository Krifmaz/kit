//! The offline banner (offline mode, with Retry) and the connection banner (`#conn-banner`).
//!
//! Their words are the configured [`OfflineWords`](kit_rules::offline::mode::OfflineWords); the connection banner
//! follows the browser's `online`/`offline` and the API's `<app id>:server-down`/`server-up` events
//! ([`watch_connection`]).

use crate::html::html;
use crate::icons::paths;
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::rc::Rc;

fn svg(name: &'static str) -> impl IntoView {
    view! { <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(paths(name))}>{""}</svg> }
}

/// Offline mode's banner: why, and Retry ("Trying…" while it tries).
#[component]
pub fn OfflineBanner(
    /// The 30 days of grace have ended (the banner asks to sign in again).
    #[prop(into)]
    grace_ended: Signal<bool>,
    /// A retry runs.
    #[prop(into)]
    trying: Signal<bool>,
    /// Retry was pressed.
    on_retry: Callback<()>,
) -> impl IntoView {
    let w = crate::config::get().words.offline;
    view! {
        <div {..()} id="offline-banner" class="offline-banner" role="status">
            {svg("offline")}
            <span>{move || w.banner(grace_ended.get())}</span>
            <button class="btn sm" type="button" id="offline-retry" disabled=move || trying.get() on:click=move |_| on_retry.run(())>
                {move || if trying.get() { w.trying } else { w.retry }}
            </button>
        </div>
    }
}

thread_local! {
    static TRYING_NOW: ArcRwSignal<bool> = ArcRwSignal::new(false);
    static CONN: ArcRwSignal<String> = ArcRwSignal::new(String::new());
    static OFFLINE_MODE: ArcRwSignal<bool> = ArcRwSignal::new(false);
}

/// Says whether offline mode's retry runs (the banner's button).
pub fn set_trying(on: bool) {
    TRYING_NOW.with(|t| t.set(on));
}

/// Shows offline mode's banner as the first element of `<body>`, and takes the connection banner
/// away (offline mode has its own). `on_retry` runs on Retry.
pub fn show_banner(grace_ended: bool, on_retry: Rc<dyn Fn()>) {
    OFFLINE_MODE.with(|m| m.set(true));
    conn_banner(false, "");
    let Some(body) = crate::rt::document().and_then(|d| d.body()) else { return };
    if crate::rt::by_id("offline-banner").is_some() {
        return;
    }
    let Some(tmp) = crate::rt::document().and_then(|d| d.create_element("div").ok()) else { return };
    let trying = TRYING_NOW.with(Clone::clone);
    let on_retry = SendWrapper::new(on_retry);
    let m = crate::rt::mount(&tmp, move || {
        view! { <OfflineBanner grace_ended=grace_ended trying=Signal::derive(move || trying.get()) on_retry=Callback::new(move |_| (*on_retry)())/> }
    });
    if let Some(b) = tmp.first_element_child() {
        let _ = body.insert_before(&b, body.first_child().as_ref());
    }
    crate::rt::keep("offline-banner", m);
}

/// The connection banner: `<div id="conn-banner">` with × and the reason.
#[component]
pub fn ConnBanner(
    /// The reason.
    #[prop(into)]
    text: String,
) -> impl IntoView {
    view! { <div id="conn-banner">{svg("x")}<span>{text}</span></div> }
}

/// Shows the connection banner with `text` (`on`), or takes it away. Offline mode has its own banner, so the
/// connection banner does not show then.
pub fn conn_banner(on: bool, text: &str) {
    if on && OFFLINE_MODE.with(|m| m.get_untracked()) {
        return;
    }
    if !on {
        // at once: the banner's view is dropped, so it leaves the page now
        crate::rt::unkeep("conn-banner");
        return;
    }
    CONN.with(|c| c.set(text.to_string()));
    if crate::rt::by_id("conn-banner").is_none() {
        let Some(body) = crate::rt::document().and_then(|d| d.body()) else { return };
        let conn = CONN.with(Clone::clone);
        let m = crate::rt::mount(&body, move || {
            view! { <div id="conn-banner">{svg("x")}<span>{move || conn.get()}</span></div> }
        });
        crate::rt::keep("conn-banner", m);
    }
}

/// Wires the connection banner to the browser and the configured app's API events: "Not connected to the
/// server" while offline, "The <app> server isn't answering" on `<id>:server-down`, gone on `online` and
/// `<id>:server-up`.
pub fn watch_connection() {
    let c = crate::config::get();
    let id = c.app.id.to_ascii_lowercase();
    let w = c.words.offline;
    let down = w.server_down(&c.app.name);
    crate::rt::on_window("offline", move |_| conn_banner(true, w.not_connected));
    crate::rt::on_window("online", |_| conn_banner(false, ""));
    crate::rt::on_window(&format!("{id}:server-down"), move |_| conn_banner(true, &down));
    crate::rt::on_window(&format!("{id}:server-up"), |_| conn_banner(false, ""));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};
    use kit_rules::offline::mode::{server_down, NOT_CONNECTED};

    const OFFLINE_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h.01"/><path d="M8.5 16.429a5 5 0 0 1 7 0"/><path d="M5 12.859a10 10 0 0 1 5.17-2.69"/><path d="M19 12.859a10 10 0 0 0-2.007-1.523"/><path d="M2 8.82a15 15 0 0 1 4.177-2.643"/><path d="M22 8.82a15 15 0 0 0-11.288-3.764"/><path d="m2 2 20 20"/></svg>"#;
    const X_SVG: &str = r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>"#;

    #[test]
    fn the_offline_banner_says_why_and_retries() {
        let html = render(|| view! { <OfflineBanner grace_ended=false trying=false on_retry=Callback::new(|_| {})/> });
        let want = format!(
            r#"<div id="offline-banner" class="offline-banner" role="status">{OFFLINE_SVG}<span>Offline – the server can&#39;t be reached. What this device saved still works.</span><button class="btn sm" type="button" id="offline-retry">Retry</button></div>"#
        );
        assert_same_html(&html, &want);
        let html = render(|| view! { <OfflineBanner grace_ended=true trying=true on_retry=Callback::new(|_| {})/> });
        assert!(html.contains(r#"disabled"#) && html.contains("Trying…"), "{html}");
        assert!(html.contains(&crate::html::esc(kit_rules::offline::mode::BANNER_ENDED)), "{html}");
    }

    #[test]
    fn the_connection_banner_names_the_app() {
        let html = render(|| view! { <ConnBanner text=server_down("Kit Demo")/> });
        assert_same_html(&html, &format!(r#"<div id="conn-banner">{X_SVG}<span>The Kit Demo server isn&#39;t answering</span></div>"#));
        assert_same_html(
            &render(|| view! { <ConnBanner text=NOT_CONNECTED/> }),
            &format!(r#"<div id="conn-banner">{X_SVG}<span>{NOT_CONNECTED}</span></div>"#),
        );
    }

    #[test]
    fn an_app_words_the_banner_its_own_way() {
        let mut cfg = crate::config::AppConfig::default();
        cfg.words.offline.banner = "Keine Verbindung";
        cfg.words.offline.retry = "Nochmal";
        crate::config::set(cfg);
        let html = render(|| view! { <OfflineBanner grace_ended=false trying=false on_retry=Callback::new(|_| {})/> });
        assert!(html.contains("<span>Keine Verbindung</span>") && html.contains(">Nochmal</button>"), "{html}");
        crate::config::set(crate::config::AppConfig::default());
    }
}
