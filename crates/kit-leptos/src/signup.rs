//! Sign-up on the web: the line under the sign-in form ([`SignInLine`]), the sign-up page with an invite link or
//! open sign-up ([`SignUpPage`]), the email confirmation page ([`VerifyPage`]), and the admin's page: who can
//! sign up, invites and waiting sign-ups ([`SignupAdmin`]).
//!
//! The server is kit-server-signup (`contracts/signup.md`), at the configured
//! [`signup`](crate::config::Paths::signup) path. Every decision and word comes from
//! [`kit_rules::signup`] ([`SignupWords`]). Where another app keeps the accounts, the admin page is read-only and
//! the sign-in line points there.

use crate::api;
use crate::clipboard::copy_then;
use crate::diagnostics::date;
use crate::html::{opt, seq};
use crate::modal::{self, ModalOpts};
use crate::toasts::toast;
use kit_rules::app::fill;
use kit_rules::dialogs::Confirm;
use kit_rules::signup::{
    check_form, choices, full_link, invite_status, label_of, parse_uses, sign_up_body, token_from, Choice, InviteForm, InviteRow,
    PendingRow, Policy, Settings, SignInLink, SignupWords, EXPIRY, WHO,
};
use leptos::prelude::*;
use serde_json::{json, Value};

/// The words (the defaults; an app changes them in [`kit_rules::signup::SignupWords`] and passes nothing here).
fn words() -> SignupWords {
    SignupWords::default()
}

/// A sign-up path: the configured base, then `p`.
fn path(p: &str) -> String {
    format!("{}{p}", crate::config::get().paths.signup)
}

/// This page's origin (`https://app.example.com`), for links the server gave without one.
fn origin() -> String {
    crate::rt::window().and_then(|w| w.location().origin().ok()).unwrap_or_default()
}

/// Now, in seconds since 1970.
fn now() -> f64 {
    crate::rt::now_ms() as f64 / 1000.0
}

/// The line under the sign-in form: a link to `sign_up_href` (open sign-up), how to get in (invite only), where
/// accounts are made (another app keeps them), or nothing (closed).
#[component]
pub fn SignInLine(
    /// the app's sign-up page (`#/sign-up`)
    #[prop(into)]
    sign_up_href: String,
) -> impl IntoView {
    let policy = LocalResource::new(|| async { api::get(&path("")).await.ok().map(|v| Policy::from_json(&v)) });
    move || {
        let p = policy.get().flatten()?;
        let (text, link) = p.sign_in_line(&words())?;
        Some(match link {
            SignInLink::SignUp => view! { <p class="small" id="signup-line" style="text-align:center;margin:14px 0 0"><a href=sign_up_href.clone()>{text}</a></p> }.into_any(),
            SignInLink::Source(url) => view! { <p class="small muted" id="signup-line" style="text-align:center;margin:14px 0 0"><a href=url>{text}</a></p> }.into_any(),
            SignInLink::None => view! { <p class="small muted" id="signup-line" style="text-align:center;margin:14px 0 0">{text}</p> }.into_any(),
        })
    }
}

/// The sign-up page: the form (with the invite of the link, if any), then the server's answer.
#[component]
pub fn SignUpPage(
    /// the app's name, at the top
    #[prop(into)]
    app: String,
    /// the invite token from the link (`?invite=`); "" for none
    #[prop(into, optional)]
    invite: String,
    /// where Back to sign in goes (`#/`)
    #[prop(into)]
    sign_in_href: String,
) -> impl IntoView {
    let w = words();
    let invite = RwSignal::new(token_from(&invite));
    let policy = LocalResource::new(|| async { api::get(&path("")).await.map(|v| Policy::from_json(&v)).map_err(|e| e.message) });
    // what a checked invite gives, or why it can't be used
    let checked = LocalResource::new(move || {
        let t = invite.get();
        async move {
            if t.is_empty() {
                return None;
            }
            Some(api::post(&path("/invite"), json!({ "token": t })).await.map_err(|e| e.message))
        }
    });
    let (error, set_error) = signal(None::<String>);
    let (done, set_done) = signal(None::<String>);
    let (busy, set_busy) = signal(false);
    let fields = (
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
        NodeRef::<leptos::html::Input>::new(),
    );
    let (username, email, password, again) = fields;
    let value = |r: NodeRef<leptos::html::Input>| r.get().map(|i| i.value()).unwrap_or_default();
    let submit = move |e: leptos::ev::SubmitEvent| {
        e.prevent_default();
        let w = words();
        let p = policy.get().and_then(Result::ok).unwrap_or_default();
        let has_invite = !invite.get_untracked().is_empty();
        let (u, m, pw, pw2) = (value(username), value(email), value(password), value(again));
        if let Err(msg) = check_form(&u, &pw, &pw2, &m, p.email_required(has_invite), &w) {
            set_error.set(Some(msg));
            return;
        }
        set_busy.set(true);
        set_error.set(None);
        let body = sign_up_body(&u, &pw, &m, &invite.get_untracked());
        crate::rt::spawn(async move {
            match api::post(&path(""), body).await {
                Ok(v) => set_done.set(Some(v.get("message").and_then(Value::as_str).unwrap_or("").to_string())),
                Err(ex) => set_error.set(Some(ex.message)),
            }
            set_busy.set(false);
        });
    };
    let paste = move |e: leptos::ev::Event| invite.set(token_from(&event_target_value(&e)));
    let (back_href, back_label) = (sign_in_href.clone(), w.back);
    let body = move || {
        let w = words();
        if let Some(msg) = done.get() {
            return view! {
                <p id="signup-done" style="text-align:center">{msg}</p>
                <a class="btn primary" href=back_href.clone() style="padding:12px;text-align:center">{back_label}</a>
            }
            .into_any();
        }
        let p = match policy.get() {
            None => return view! { <div class="empty"><span class="spinner"></span></div> }.into_any(),
            Some(Err(e)) => return view! { <div class="error-box">{e}</div> }.into_any(),
            Some(Ok(p)) => p,
        };
        let has_invite = !invite.get().is_empty();
        if let Err(why) = p.form(has_invite, &w) {
            let source = p
                .managed_by
                .clone()
                .map(|m| view! { <a class="btn primary" href=m.url style="padding:12px;text-align:center">{m.name}</a> });
            let paste_field = (p.who == "invite" && p.managed_by.is_none()).then(|| {
                view! { <label class="field"><span>{w.invite_field}</span><input class="input" id="signup-invite" on:input=paste/></label> }
            });
            return view! { <p class="muted" id="signup-why">{why}</p>{opt(paste_field)}{opt(source)} }.into_any();
        }
        let gives = match checked.get() {
            Some(Some(Ok(v))) => {
                let role = v.get("role").and_then(Value::as_str).unwrap_or("").to_string();
                Some(
                    view! { <p class="small muted" id="signup-invite-ok" style="margin:0">{fill(w.invite_gives, &[("role", &role)])}</p> }
                        .into_any(),
                )
            }
            Some(Some(Err(e))) => Some(view! { <div class="error-box" id="signup-invite-err">{e}</div> }.into_any()),
            _ => None,
        };
        let email_hint = if p.email_required(has_invite) { String::new() } else { format!(" ({})", w.email_optional) };
        view! {
            {opt(gives)}
            <div class="error-box" class:hidden=move || error.get().is_none()>{move || error.get()}</div>
            <form autocomplete="on" on:submit=submit id="signup-form">
                <label class="field"><span>"Username"</span><input class="input" name="username" autocomplete="username" required autofocus node_ref=username/></label>
                <label class="field"><span>{format!("{}{email_hint}", w.email)}</span><input class="input" type="email" name="email" autocomplete="email" required=p.email_required(has_invite) node_ref=email/></label>
                <label class="field"><span>"Password"</span><input class="input" type="password" name="password" autocomplete="new-password" required node_ref=password/></label>
                <label class="field"><span>{w.confirm}</span><input class="input" type="password" name="password2" autocomplete="new-password" required node_ref=again/></label>
                <button class="btn primary" type="submit" style="padding:12px" disabled=busy>{move || if busy.get() { words().signing_up } else { words().sign_up }}</button>
            </form>
        }
        .into_any()
    };
    view! {
        <div class="auth-wrap">
            <div class="auth-card" id="signup-page">
                <div class="brand">{app}</div>
                <p class="muted" style="text-align:center;margin:0">{w.title}</p>
                <div class="stack mt">{body}</div>
                <p class="small" style="text-align:center;margin:14px 0 0"><a href=sign_in_href>{w.back}</a></p>
            </div>
        </div>
    }
}

/// The page the confirmation mail's link opens: confirms the email and says what's next.
#[component]
pub fn VerifyPage(
    /// the app's name
    #[prop(into)]
    app: String,
    /// the token from the link (`?token=`)
    #[prop(into)]
    token: String,
    /// where Back to sign in goes
    #[prop(into)]
    sign_in_href: String,
) -> impl IntoView {
    let w = words();
    let token = token_from(&token);
    let answer = LocalResource::new(move || {
        let t = token.clone();
        async move {
            api::post(&path("/verify"), json!({ "token": t }))
                .await
                .map(|v| v.get("message").and_then(Value::as_str).unwrap_or("").to_string())
                .map_err(|e| e.message)
        }
    });
    let body = move || match answer.get() {
        None => view! { <p class="muted"><span class="spinner"></span>" "{words().verifying}</p> }.into_any(),
        Some(Ok(m)) => view! { <p id="verify-done">{m}</p> }.into_any(),
        Some(Err(e)) => view! { <div class="error-box" id="verify-err">{e}</div> }.into_any(),
    };
    view! {
        <div class="auth-wrap">
            <div class="auth-card" id="verify-page">
                <div class="brand">{app}</div>
                <p class="muted" style="text-align:center;margin:0">{w.title}</p>
                <div class="stack mt">{body}</div>
                <a class="btn primary mt" href=sign_in_href style="padding:12px;text-align:center">{w.back}</a>
            </div>
        </div>
    }
}

/// Everything the admin page loads.
#[derive(Clone, Default)]
struct AdminData {
    /// the setting
    settings: Settings,
    /// the roles offered
    roles: Vec<Choice>,
    /// the presets offered
    presets: Vec<Choice>,
    /// whether mail leaves the server
    mail_delivers: bool,
    /// read-only (another app keeps the accounts): its name, and whether it answered
    managed: Option<(String, bool)>,
    /// the invites
    invites: Vec<InviteRow>,
    /// the waiting sign-ups
    pending: Vec<PendingRow>,
}

/// Loads the admin page's data.
async fn load_admin() -> Result<AdminData, String> {
    let s = api::get(&path("/settings")).await.map_err(|e| e.message)?;
    let managed = kit_rules::signup::ManagedBy::from_json(&s).map(|m| (m.name, m.reachable));
    let (invites, pending) = if managed.is_some() {
        (Vec::new(), Vec::new())
    } else {
        let i = api::get(&path("/invites")).await.map_err(|e| e.message)?;
        let p = api::get(&path("/pending")).await.map_err(|e| e.message)?;
        (InviteRow::list(&i), PendingRow::list(&p))
    };
    Ok(AdminData {
        settings: Settings::from_json(s.get("settings").unwrap_or(&Value::Null)),
        roles: choices(&s, "roles"),
        presets: choices(&s, "presets"),
        mail_delivers: s.get("mail_delivers").and_then(Value::as_bool).unwrap_or(false),
        managed,
        invites,
        pending,
    })
}

/// The admin's sign-up page: who can sign up (and how open sign-up is held back), waiting sign-ups, invites.
#[component]
pub fn SignupAdmin() -> impl IntoView {
    let data = LocalResource::new(load_admin);
    let reload = move || data.refetch();
    move || match data.get() {
        None => view! { <div class="empty"><span class="spinner"></span></div> }.into_any(),
        Some(Err(e)) => view! { <div class="error-box">{e}</div> }.into_any(),
        Some(Ok(d)) => {
            let w = words();
            if let Some((name, reachable)) = d.managed.clone() {
                let extra = (!reachable).then(|| view! { " "{fill(w.unreachable, &[("app", &name)])} });
                return view! {
                    <div class="notice-banner info" id="signup-read-only"><span>{fill(w.read_only, &[("app", &name)])}{opt(extra)}</span></div>
                    <SettingsCard data=d.clone() reload=reload read_only=true/>
                }
                .into_any();
            }
            view! {
                <div class="stack" id="signup-admin">
                    <SettingsCard data=d.clone() reload=reload read_only=false/>
                    <PendingCard pending=d.pending.clone() reload=reload/>
                    <InvitesCard data=d.clone() reload=reload/>
                </div>
            }
            .into_any()
        }
    }
}

/// A role `<select>` over `roles`, `value` its signal.
fn role_select(roles: &[Choice], value: RwSignal<String>, disabled: bool, id: &'static str) -> impl IntoView {
    let opts = seq(roles.iter().map(|r| {
        let id = r.id.clone();
        let sel = id == value.get_untracked();
        view! { <option value=id selected=sel>{r.label.clone()}</option> }
    }));
    view! { <select class="input" id=id disabled=disabled on:change=move |e| value.set(event_target_value(&e))>{opts}</select> }
}

/// A switch per preset, `value` the picked ids.
fn preset_switches(presets: &[Choice], value: RwSignal<Vec<String>>, disabled: bool) -> impl IntoView {
    seq(presets.iter().map(|p| {
        let id = p.id.clone();
        let on = value.get_untracked().contains(&id);
        let toggle = move |e: leptos::ev::Event| {
            let checked = event_target_checked(&e);
            value.update(|v| {
                v.retain(|x| x != &id);
                if checked {
                    v.push(id.clone());
                }
            });
        };
        view! { <label class="switch" data-preset=p.id.clone()><input type="checkbox" checked=on disabled=disabled on:change=toggle/><i>{""}</i><span>{p.label.clone()}</span></label> }
    }))
}

/// Who can sign up, and the open sign-up block; Save.
#[component]
fn SettingsCard(data: AdminData, reload: impl Fn() + Copy + Send + Sync + 'static, read_only: bool) -> impl IntoView {
    let w = words();
    let s = data.settings.clone();
    let who = RwSignal::new(s.who.clone());
    let role = RwSignal::new(if s.open_role.is_empty() {
        data.roles.first().map(|r| r.id.clone()).unwrap_or_default()
    } else {
        s.open_role.clone()
    });
    let presets = RwSignal::new(s.open_presets.clone());
    let verify = RwSignal::new(s.verify_email);
    let approval = RwSignal::new(s.approval);
    let per_hour = RwSignal::new(s.per_hour.to_string());
    let per_day = RwSignal::new(s.per_day.to_string());
    let choices = seq(WHO.iter().map(|(id, name, line)| {
        let id = id.to_string();
        let on = {
            let id = id.clone();
            move || who.get() == id
        };
        let pick = {
            let id = id.clone();
            move |_| who.set(id.clone())
        };
        view! {
            <label class="list-item" style="align-items:flex-start;cursor:pointer" data-who=id.clone()>
                <input type="radio" name="signup-who" checked=on disabled=read_only on:change=pick style="margin-top:4px"/>
                <div class="grow"><b>{*name}</b><div class="small muted">{*line}</div></div>
            </label>
        }
    }));
    let no_mail = (!data.mail_delivers).then(|| view! { <div class="small muted" id="signup-no-mail">{w.no_mail}</div> });
    let open_block = view! {
        <div class="stack" id="signup-open" style="gap:10px">
            <h3 style="margin:0">{w.open_title}</h3>
            <label class="field"><span>{w.open_role}</span>{role_select(&data.roles, role, read_only, "signup-open-role")}</label>
            {(!data.presets.is_empty()).then(|| view! { <div class="field"><span>{w.open_presets}</span><div class="row wrap" style="gap:14px">{preset_switches(&data.presets, presets, read_only)}</div></div> })}
            <label class="switch"><input type="checkbox" id="signup-verify" checked=verify.get_untracked() disabled=read_only on:change=move |e| verify.set(event_target_checked(&e))/><i>{""}</i><span>{w.verify_email}</span></label>
            {opt(no_mail)}
            <label class="switch"><input type="checkbox" id="signup-approval" checked=approval.get_untracked() disabled=read_only on:change=move |e| approval.set(event_target_checked(&e))/><i>{""}</i><span>{w.approval}</span></label>
            <div class="form-grid">
                <label class="field"><span>{w.per_hour}</span><input class="input" type="number" min="1" id="signup-per-hour" prop:value=per_hour disabled=read_only on:input=move |e| per_hour.set(event_target_value(&e))/></label>
                <label class="field"><span>{w.per_day}</span><input class="input" type="number" min="0" id="signup-per-day" prop:value=per_day disabled=read_only on:input=move |e| per_day.set(event_target_value(&e))/></label>
            </div>
        </div>
    };
    let (busy, set_busy) = signal(false);
    let save = move |_| {
        let body = Settings {
            who: who.get_untracked(),
            open_role: role.get_untracked(),
            open_presets: presets.get_untracked(),
            verify_email: verify.get_untracked(),
            approval: approval.get_untracked(),
            per_hour: per_hour.get_untracked().trim().parse().unwrap_or(5),
            per_day: per_day.get_untracked().trim().parse().unwrap_or(50),
        };
        set_busy.set(true);
        crate::rt::spawn(async move {
            match api::put(&path("/settings"), body.to_json()).await {
                Ok(_) => {
                    toast(words().saved, "ok");
                    reload();
                }
                Err(e) => toast(&e.message, "err"),
            }
            set_busy.set(false);
        });
    };
    let save_btn = (!read_only).then(
        || view! { <div class="row"><button class="btn primary" id="signup-save" disabled=busy on:click=save>{w.save}</button></div> },
    );
    view! {
        <div class="card stack" id="signup-settings">
            <h2 style="margin:0">{w.who_title}</h2>
            <div class="list">{choices}</div>
            <div class:hidden=move || who.get() != "open">{open_block}</div>
            {opt(save_btn)}
        </div>
    }
}

/// The sign-ups that wait, with Approve and Reject.
#[component]
fn PendingCard(pending: Vec<PendingRow>, reload: impl Fn() + Copy + Send + Sync + 'static) -> impl IntoView {
    let w = words();
    let act = move |id: i64, what: &'static str, name: String| {
        crate::rt::spawn(async move {
            let w = words();
            if what == "reject" {
                let q = Confirm::new(fill(w.reject_question, &[("username", &name)])).text(w.reject_text).ok(w.reject).danger(true);
                if !modal::confirm(q).await {
                    return;
                }
            }
            match api::post_empty(&path(&format!("/pending/{id}/{what}"))).await {
                Ok(_) => reload(),
                Err(e) => toast(&e.message, "err"),
            }
        })
    };
    let rows = if pending.is_empty() {
        view! { <div class="empty small muted" id="signup-no-pending">{w.no_pending}</div> }.into_any()
    } else {
        seq(pending.into_iter().map(|p| {
            let (id, name, name2) = (p.id, p.username.clone(), p.username.clone());
            let email = (!p.email.is_empty()).then(|| format!(" · {}", p.email));
            view! {
                <div class="list-item" data-pending=p.username.clone() style="flex-wrap:wrap;gap:10px">
                    <div class="grow" style="min-width:200px">
                        <b>{p.username.clone()}</b><span class="small muted">{email}</span>
                        <div class="small faint">{format!("{} · {} · {}", p.waiting_line(&words()), date(p.created_at), p.address)}</div>
                    </div>
                    <button class="btn" data-approve=id on:click=move |_| act(id, "approve", name.clone())>{words().approve}</button>
                    <button class="btn danger" data-reject=id on:click=move |_| act(id, "reject", name2.clone())>{words().reject}</button>
                </div>
            }
        }))
        .into_any()
    };
    view! {
        <div class="card stack" id="signup-pending">
            <h2 style="margin:0">{w.pending_title}</h2>
            <div class="list">{rows}</div>
        </div>
    }
}

/// Shows a new invite's link once, with Copy link.
fn show_link(link: String) {
    let w = words();
    let (l1, l2) = (link.clone(), link);
    modal::open(
        ModalOpts::with_view(w.link_title, move || {
            view! {
                <p class="small muted" style="margin-top:0">{words().link_once}</p>
                <div class="code" id="invite-link">{l1}</div>
            }
        })
        .footer_view(move || {
            view! {
                <button class="btn primary" type="button" id="invite-copy" on:click=move |_| copy_then(&l2, words().copied)>{words().copy_link}</button>
                <button class="btn" type="button" data-close>{crate::config::get().words.dialogs.close}</button>
            }
        }),
    );
}

/// New invite, and every invite with its status, uses, expiry, who used it, and Revoke.
#[component]
fn InvitesCard(data: AdminData, reload: impl Fn() + Copy + Send + Sync + 'static) -> impl IntoView {
    let w = words();
    let open = RwSignal::new(false);
    let note = RwSignal::new(String::new());
    let role = RwSignal::new(data.roles.first().map(|r| r.id.clone()).unwrap_or_default());
    let presets = RwSignal::new(Vec::<String>::new());
    let uses_kind = RwSignal::new("one".to_string());
    let uses_n = RwSignal::new("5".to_string());
    let hours = RwSignal::new(kit_rules::signup::DEFAULT_EXPIRY_HOURS);
    let create = move |_| {
        let max_uses = match uses_kind.get_untracked().as_str() {
            "one" => 1,
            "any" => 0,
            _ => match parse_uses(&uses_n.get_untracked()) {
                Some(n) => n,
                None => {
                    toast("Enter a number of people from 2 to 100000", "err");
                    return;
                }
            },
        };
        let form = InviteForm {
            note: note.get_untracked(),
            role: role.get_untracked(),
            presets: presets.get_untracked(),
            max_uses,
            expires_in_hours: hours.get_untracked(),
        };
        crate::rt::spawn(async move {
            match api::post(&path("/invites"), form.to_json()).await {
                Ok(v) => {
                    open.set(false);
                    note.set(String::new());
                    show_link(full_link(v.get("link").and_then(Value::as_str).unwrap_or(""), &origin()));
                    reload();
                }
                Err(e) => toast(&e.message, "err"),
            }
        });
    };
    let seg = |id: &'static str, label: &'static str| {
        view! { <button type="button" data-uses=id class:on=move || uses_kind.get() == id on:click=move |_| uses_kind.set(id.to_string())>{label}</button> }
    };
    let expiry = seq(EXPIRY.iter().map(|(h, label)| {
        let sel = *h == kit_rules::signup::DEFAULT_EXPIRY_HOURS;
        view! { <option value=h.to_string() selected=sel>{*label}</option> }
    }));
    let form = view! {
        <div class="card flat stack" id="invite-form" style="gap:10px">
            <label class="field"><span>{w.note}</span><input class="input" id="invite-note" placeholder=w.note_placeholder maxlength="200" prop:value=note on:input=move |e| note.set(event_target_value(&e))/></label>
            <div class="form-grid">
                <label class="field"><span>{w.role}</span>{role_select(&data.roles, role, false, "invite-role")}</label>
                <label class="field"><span>{w.expires}</span><select class="input" id="invite-expiry" on:change=move |e| hours.set(event_target_value(&e).parse().unwrap_or(0))>{expiry}</select></label>
            </div>
            {(!data.presets.is_empty()).then(|| view! { <div class="field"><span>{w.presets}</span><div class="row wrap" style="gap:14px">{preset_switches(&data.presets, presets, false)}</div></div> })}
            <div class="field"><span>{w.uses}</span>
                <div class="row wrap" style="gap:10px">
                    <div class="seg" id="invite-uses">{seg("one", w.one_time)}{seg("some", w.several)}{seg("any", w.no_limit)}</div>
                    <input class="input" type="number" min="2" id="invite-uses-n" style="width:110px" class:hidden=move || uses_kind.get() != "some" prop:value=uses_n on:input=move |e| uses_n.set(event_target_value(&e))/>
                </div>
            </div>
            <div class="row"><button class="btn primary" id="invite-create" on:click=create>{w.create}</button></div>
        </div>
    };
    let revoke = move |id: i64, title: String| {
        crate::rt::spawn(async move {
            let w = words();
            let q = Confirm::new(fill(w.revoke_question, &[("note", &title)])).text(w.revoke_text).ok(w.revoke).danger(true);
            if !modal::confirm(q).await {
                return;
            }
            match api::post_empty(&path(&format!("/invites/{id}/revoke"))).await {
                Ok(_) => reload(),
                Err(e) => toast(&e.message, "err"),
            }
        })
    };
    let (roles, all_presets) = (data.roles.clone(), data.presets.clone());
    let rows = if data.invites.is_empty() {
        view! { <div class="empty small muted" id="no-invites">{w.no_invites}</div> }.into_any()
    } else {
        seq(data.invites.into_iter().map(|i| {
            let w = words();
            let (label, class) = invite_status(&i.status);
            let title = i.title(&w);
            let gives = std::iter::once(label_of(&roles, &i.role))
                .chain(i.presets.iter().map(|p| label_of(&all_presets, p)))
                .collect::<Vec<_>>()
                .join(", ");
            let used = if i.used_by.is_empty() {
                w.unused.to_string()
            } else {
                format!("{}: {}", w.used_by, i.used_by.iter().map(|(n, at)| format!("{n} ({})", date(*at))).collect::<Vec<_>>().join(", "))
            };
            let (id, t2) = (i.id, title.clone());
            let btn = i.can_revoke().then(
                || view! { <button class="btn danger" data-revoke=id on:click=move |_| revoke(id, t2.clone())>{words().revoke}</button> },
            );
            view! {
                <div class="list-item" data-invite=i.id style="flex-wrap:wrap;gap:10px;align-items:flex-start">
                    <div class="grow" style="min-width:220px">
                        <div class="row wrap"><b>{title}</b><span class=format!("badge {class}")>{label}</span></div>
                        <div class="small muted">{format!("{gives} · {} · {}", i.uses_line(), i.expiry_line(now()))}</div>
                        <div class="small faint">{format!("{} · {}", i.created_by, date(i.created_at))}</div>
                        <div class="small" data-used-by="">{used}</div>
                    </div>
                    {opt(btn)}
                </div>
            }
        }))
        .into_any()
    };
    view! {
        <div class="card stack" id="signup-invites">
            <div class="row wrap between"><h2 style="margin:0">{w.invites_title}</h2><button class="btn" id="invite-new" on:click=move |_| open.update(|o| *o = !*o)>{w.new_invite}</button></div>
            <div class:hidden=move || !open.get()>{form}</div>
            <div class="list">{rows}</div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::render;

    fn data() -> AdminData {
        AdminData {
            settings: Settings { who: "open".into(), approval: true, ..Settings::default() },
            roles: vec![Choice { id: "member".into(), label: "Member".into() }, Choice { id: "viewer".into(), label: "Viewer".into() }],
            presets: vec![Choice { id: "shared".into(), label: "Shared folder".into() }],
            mail_delivers: false,
            managed: None,
            invites: InviteRow::list(&json!({ "invites": [
                { "id": 2, "note": "team", "role": "viewer", "presets": ["shared"], "max_uses": 0, "uses": 1, "status": "active",
                  "created_by_name": "admin", "used_by": [{ "username": "ana", "used_at": 0 }] },
                { "id": 1, "note": "", "role": "member", "max_uses": 1, "uses": 1, "status": "used_up", "created_by_name": "admin" }
            ] })),
            pending: PendingRow::list(
                &json!({ "pending": [{ "id": 4, "username": "vic", "waiting_for": "approval", "address": "10.0.0.1" }] }),
            ),
        }
    }

    #[test]
    fn the_setting_card_shows_every_choice_and_the_open_block() {
        let html = render(|| view! { <SettingsCard data=data() reload=|| () read_only=false/> });
        for want in [
            "data-who=\"closed\"",
            "data-who=\"invite\"",
            "data-who=\"open\"",
            "Invite only",
            "id=\"signup-verify\"",
            "id=\"signup-no-mail\"",
            "id=\"signup-save\"",
            "Shared folder",
        ] {
            assert!(html.contains(want), "{want} in {html}");
        }
        let ro = render(|| view! { <SettingsCard data=data() reload=|| () read_only=true/> });
        assert!(!ro.contains("signup-save") && ro.contains("disabled"));
    }

    #[test]
    fn invites_show_status_uses_who_used_them_and_revoke_while_active() {
        let html = render(|| view! { <InvitesCard data=data() reload=|| ()/> });
        assert!(html.contains("team") && html.contains("Invite 1"));
        assert!(html.contains("Viewer, Shared folder · 1 use, no limit · Never expires"));
        assert!(html.contains("badge ok") && html.contains("Used up"));
        assert!(html.contains("Used by: ana"));
        assert_eq!(html.matches("data-revoke").count(), 1, "only the active one");
        assert!(html.contains("id=\"invite-uses\""));
    }

    #[test]
    fn waiting_sign_ups_have_approve_and_reject() {
        let d = data();
        let html = render(move || view! { <PendingCard pending=d.pending.clone() reload=|| ()/> });
        assert!(
            html.contains("vic") && html.contains("Waiting for approval") && html.contains("data-approve") && html.contains("data-reject")
        );
        let empty = render(|| view! { <PendingCard pending=vec![] reload=|| ()/> });
        assert!(empty.contains("Nobody is waiting."));
    }
}
