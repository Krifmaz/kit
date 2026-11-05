//! First-run setup on the web: the "Create your account" page a new server shows instead of the sign-in page
//! while it has no owner ([`SetupPage`]), and the check whether it shows ([`setup_needed`]).
//!
//! The server is kit-server-setup (`contracts/setup.md`), at the configured [`setup`](crate::config::Paths::setup)
//! path. The page asks for the one-time setup code from the server's log, a username, the password twice and an
//! optional email. Once the server made the account, the page hands the name and password to the app
//! (`on_done`), which signs in with them as its sign-in page does. Every decision and word comes from
//! [`kit_rules::setup`] ([`SetupWords`]).

use crate::api;
use kit_rules::setup::{body, check_form, needed, SetupWords};
use leptos::prelude::*;

/// The setup route: the configured path under the API.
fn path() -> String {
    crate::config::get().paths.setup.clone()
}

/// Whether the server wants its owner made on this page (`GET /api/setup`). A server without setup, or one that
/// can't be reached, gives `false`: the app shows its sign-in page as before.
pub async fn setup_needed() -> bool {
    api::get(&path()).await.map(|v| needed(&v)).unwrap_or(false)
}

/// The "Create your account" page: setup code, username, password twice, optional email; then `on_done` with
/// the name and password, for the app to sign in.
#[component]
pub fn SetupPage(
    /// the app's name, at the top
    #[prop(into)]
    app: String,
    /// called once the account is made, with its username and password
    on_done: Callback<(String, String)>,
) -> impl IntoView {
    let w = SetupWords::default();
    let (error, set_error) = signal(None::<String>);
    let (busy, set_busy) = signal(false);
    let fields = (
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
    );
    let (code, username, email, password, again) = fields;
    let value = |r: NodeRef<leptos::html::Input>| r.get().map(|i| i.value()).unwrap_or_default();
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let (c, u, m, pw, pw2) = (value(code), value(username), value(email), value(password), value(again));
        if let Err(msg) = check_form(&c, &u, &pw, &pw2, &SetupWords::default()) {
            set_error.set(Some(msg));
            return;
        }
        set_busy.set(true);
        set_error.set(None);
        let req = body(&c, &u, &pw, &m);
        crate::rt::spawn(async move {
            match api::post(&path(), req).await {
                Ok(_) => on_done.run((u.trim().to_string(), pw)),
                Err(ex) => set_error.set(Some(ex.message)),
            }
            set_busy.set(false);
        });
    };
    view! {
        <div class="auth-wrap">
            <div class="auth-card" id="setup-page">
                <div class="brand">{app}</div>
                <p style="text-align:center;margin:0"><b>{w.title}</b></p>
                <p class="small muted" style="text-align:center;margin:6px 0 0">{w.intro}</p>
                <div class="stack mt">
                    <div class="error-box" id="setup-error" class:hidden=move || error.get().is_none()>{move || error.get()}</div>
                    <form autocomplete="on" on:submit=submit id="setup-form">
                        <label class="field">
                            <span>{w.code}</span>
                            <input class="input" name="setup-code" id="setup-code" autocomplete="one-time-code" autocapitalize="characters" spellcheck="false" required autofocus node_ref=code/>
                            <span class="small muted">{w.code_hint}</span>
                        </label>
                        <label class="field"><span>{w.username}</span><input class="input" name="username" autocomplete="username" required node_ref=username/></label>
                        <label class="field"><span>{format!("{} ({})", w.email, w.email_optional)}</span><input class="input" type="email" name="email" autocomplete="email" node_ref=email/></label>
                        <label class="field"><span>{w.password}</span><input class="input" type="password" name="password" autocomplete="new-password" required node_ref=password/></label>
                        <label class="field"><span>{w.again}</span><input class="input" type="password" name="password2" autocomplete="new-password" required node_ref=again/></label>
                        <button class="btn primary" type="submit" id="setup-create" style="padding:12px" disabled=busy>{move || if busy.get() { SetupWords::default().creating } else { SetupWords::default().create }}</button>
                    </form>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::render;

    #[test]
    fn the_page_asks_for_the_code_name_email_and_password_twice() {
        let html = render(|| view! { <SetupPage app="Notes" on_done=Callback::new(|_| ())/> });
        for want in [
            "id=\"setup-page\"",
            "Notes",
            "Create your account",
            "id=\"setup-code\"",
            "name=\"username\"",
            "name=\"email\"",
            "name=\"password\"",
            "name=\"password2\"",
            "Email (optional)",
            "docker logs",
            "id=\"setup-create\"",
        ] {
            assert!(html.contains(want), "{want} in {html}");
        }
    }
}
