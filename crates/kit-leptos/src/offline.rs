//! The offline banner (offline mode, with Retry) and the connection banner (`#conn-banner`).
//!
//! From Encore's `web-rs/src/offline/mode.rs` (`banner`, `retry_button`, `shell_ready`) and `web-rs/src/app.rs`
//! (`conn_banner` and the `offline` / `online` / `<app>:server-down` / `<app>:server-up` listeners in `init`).
//! The words are kit-rules' [`kit_rules::offline::mode`]. Offline mode itself (when it opens, the retries, the
//! outbox) is the app's: it shows [`OfflineBanner`] (or calls [`show_banner`]) and says when a retry runs
//! ([`set_trying`]).

use crate::html::html;
use crate::icons::paths;
use kit_rules::offline::mode::{banner, server_down, NOT_CONNECTED, RETRY, TRYING};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::rc::Rc;

fn svg(name: &'static str) -> impl IntoView {
    view! { <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" {..html(paths(name))}>{""}</svg> }
}

/// Offline mode's banner: why, and Retry ("Trying…" while it tries). From `mode.rs` `banner`.
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
    view! {
        <div {..()} id="offline-banner" class="offline-banner" role="status">
            {svg("offline")}
            <span>{move || banner(grace_ended.get())}</span>
            <button class="btn sm" type="button" id="offline-retry" disabled=move || trying.get() on:click=move |_| on_retry.run(())>
                {move || if trying.get() { TRYING } else { RETRY }}
            </button>
        </div>
    }
}

thread_local! {
    static TRYING_NOW: ArcRwSignal<bool> = ArcRwSignal::new(false);
    static CONN: ArcRwSignal<String> = ArcRwSignal::new(String::new());
    static OFFLINE_MODE: ArcRwSignal<bool> = ArcRwSignal::new(false);
}

/// Says whether offline mode's retry runs (the banner's button). From `mode.rs` `retry_button`.
pub fn set_trying(on: bool) {
    TRYING_NOW.with(|t| t.set(on));
}

/// Shows offline mode's banner as the first element of `<body>`, as Encore does, and takes the connection banner
/// away (offline mode has its own). `on_retry` runs on Retry. From `mode.rs` `shell_ready` / `banner`.
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
/// connection banner does not show then. From `app.rs` `conn_banner`.
pub fn conn_banner(on: bool, text: &str) {
    if on && OFFLINE_MODE.with(|m| m.get_untracked()) {
        return;
    }
    if !on {
        // at once, as Encore's `remove()`: the banner's view is dropped, so it leaves the page now
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

/// Wires the connection banner to the browser and the app's API events, as Encore's `app.rs` `init` does:
/// [`NOT_CONNECTED`] while offline, "The <app> server isn't answering" on `<app>:server-down`, gone on `online`
/// and `<app>:server-up`.
pub fn watch_connection(app: &str) {
    let lower = app.to_ascii_lowercase();
    let down = server_down(app);
    crate::rt::on_window("offline", |_| conn_banner(true, NOT_CONNECTED));
    crate::rt::on_window("online", |_| conn_banner(false, ""));
    crate::rt::on_window(&format!("{lower}:server-down"), move |_| conn_banner(true, &down));
    crate::rt::on_window(&format!("{lower}:server-up"), |_| conn_banner(false, ""));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::esc;
    use crate::icons::icon;
    use crate::testing::{assert_same_html, render};
    use kit_rules::offline::mode::{BANNER, BANNER_ENDED};

    #[test]
    fn the_offline_banner_is_encores() {
        // mode.rs `banner`
        let want = |text: &str, button: &str| {
            format!(
                r#"<div id="offline-banner" class="offline-banner" role="status">{}<span>{}</span>{button}</div>"#,
                icon("offline"),
                esc(text)
            )
        };
        let retry = r#"<button class="btn sm" type="button" id="offline-retry">Retry</button>"#;
        let html = render(|| view! { <OfflineBanner grace_ended=false trying=false on_retry=Callback::new(|_| {})/> });
        assert_same_html(&html, &want(BANNER, retry));
        let html = render(|| view! { <OfflineBanner grace_ended=true trying=true on_retry=Callback::new(|_| {})/> });
        assert_same_html(
            &html,
            &want(BANNER_ENDED, r#"<button class="btn sm" type="button" id="offline-retry" disabled>Trying…</button>"#),
        );
    }

    #[test]
    fn the_connection_banner_is_encores() {
        // app.rs `conn_banner`
        let html = render(|| view! { <ConnBanner text=server_down("Encore")/> });
        assert_same_html(&html, &format!(r#"<div id="conn-banner">{}<span>The Encore server isn&#39;t answering</span></div>"#, icon("x")));
        assert_same_html(
            &render(|| view! { <ConnBanner text=NOT_CONNECTED/> }),
            &format!(r#"<div id="conn-banner">{}<span>{NOT_CONNECTED}</span></div>"#, icon("x")),
        );
    }
}
