//! The command palette: Ctrl+K (⌘K on macOS) opens one box that finds the app's actions and places by typing.
//!
//! The app hands its commands to [`install`] once at start-up: fixed ones ([`Command::new`], grouped under
//! [`ACTIONS`], [`GO_TO`] or groups of its own) and, if it likes, a source ([`Palette::source`]) that loads more
//! each time the palette opens (its records, its sites…). What shows for a query, and in which order, is
//! [`kit_rules::palette::results`]: with no query every fixed command, then the first six of each loaded group;
//! with one, the label matches, then the hint-only matches.
//!
//! Ctrl+K or ⌘K opens and closes it, and so do [`open`], [`close`] and [`toggle`] (a search button in the app's
//! head calls [`open`]). Escape or a press beside it closes it. In the box, Up and Down move the selection (kept
//! in view), Enter runs the selected command and a click runs the one clicked; the palette closes before the
//! command runs. A new query selects the first result again.
//!
//! The markup (`.palette-bg` › `.palette`, a dialog with a combobox and a listbox of options, see
//! [`palette_view`]) is styled by the kit's stylesheet (`style/kit.css`); its words are the configured
//! [`PaletteWords`](kit_rules::palette::PaletteWords).
//!
//! ```no_run
//! use kit_leptos::palette::{self, Command, Palette, ACTIONS, GO_TO};
//! palette::install(
//!     Palette::new(vec![
//!         Command::new(ACTIONS, "New note", || { /* … */ }).icon("plus"),
//!         Command::new(GO_TO, "Settings", || { /* … */ }).hint("/settings").icon("settings"),
//!     ])
//!     .source(|| async {
//!         // load the app's records; an error is an empty list (the fixed commands still show)
//!         vec![Command::new("Notes", "Shopping list", || { /* … */ }).hint("notes/shopping")]
//!     }),
//! );
//! // a search button: palette::open()
//! ```

use crate::html::{html, opt, seq};
use crate::icons::icon;
use crate::rt;
use kit_rules::palette::{results, step, Item, Move};
use leptos::prelude::*;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use wasm_bindgen::JsCast;

pub use kit_rules::palette::{ACTIONS, GO_TO};

/// What a source gives: the commands it loaded.
pub type SourceFuture = Pin<Box<dyn Future<Output = Vec<Command>>>>;

/// One command: what the palette shows ([`Item`]) and what it does.
#[derive(Clone)]
pub struct Command {
    /// What it shows and where (its group, label, hint and icon).
    pub item: Item,
    run: Rc<dyn Fn()>,
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Command").field("item", &self.item).finish_non_exhaustive()
    }
}

impl Command {
    /// A command labelled `label` in `group` that runs `run`.
    pub fn new(group: impl Into<String>, label: impl Into<String>, run: impl Fn() + 'static) -> Self {
        Command::from_item(Item::new(group, label), run)
    }

    /// A command that shows `item` and runs `run`.
    pub fn from_item(item: Item, run: impl Fn() + 'static) -> Self {
        Command { item, run: Rc::new(run) }
    }

    /// With this hint (a second, quieter line that is also searched).
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.item.hint = hint.into();
        self
    }

    /// With this kit icon ([`crate::icons::icon`]'s names: `"plus"`, `"settings"`, `"search"`…).
    pub fn icon(mut self, name: impl Into<String>) -> Self {
        self.item.icon = name.into();
        self
    }

    /// Runs it.
    pub fn run(&self) {
        (self.run)()
    }
}

/// What the palette offers: the fixed commands and the source of the ones loaded on opening.
#[derive(Default)]
pub struct Palette {
    commands: Vec<Command>,
    source: Option<Rc<dyn Fn() -> SourceFuture>>,
}

impl Palette {
    /// The palette with these fixed commands, listed in this order.
    pub fn new(commands: Vec<Command>) -> Self {
        Palette { commands, source: None }
    }

    /// Loads more commands each time the palette opens. They are listed after the fixed ones; with no query only
    /// the first six of each of their groups show. Until the first answer comes the fixed commands show alone;
    /// later the previous answer shows until the new one comes.
    pub fn source<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + 'static,
        Fut: Future<Output = Vec<Command>> + 'static,
    {
        self.source = Some(Rc::new(move || Box::pin(f()) as SourceFuture));
        self
    }
}

#[derive(Default)]
struct State {
    commands: Vec<Command>,
    loaded: Vec<Command>,
    source: Option<Rc<dyn Fn() -> SourceFuture>>,
    /// Counts the openings, so an answer for an earlier one is dropped.
    opening: u64,
    /// Where the focus was before the palette opened (it goes back there).
    focus: Option<web_sys::HtmlElement>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    static OPEN: ArcRwSignal<bool> = ArcRwSignal::new(false);
    /// Changes whenever the commands do.
    static REV: ArcRwSignal<u64> = ArcRwSignal::new(0);
    static INSTALLED: Cell<bool> = const { Cell::new(false) };
}

fn changed() {
    REV.with(|r| r.update(|n| *n += 1));
}

/// Sets the palette's commands and source, listens for Ctrl+K / ⌘K and Escape, and puts the palette at the
/// end of `<body>` (call once at start-up; calling again replaces the commands and the source).
pub fn install(p: Palette) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.commands = p.commands;
        s.source = p.source;
        s.loaded.clear();
    });
    changed();
    if INSTALLED.with(|i| i.replace(true)) {
        return;
    }
    rt::on_document("keydown", |e| {
        let Some(k) = e.dyn_ref::<web_sys::KeyboardEvent>() else { return };
        if kit_rules::palette::is_toggle(&k.key(), k.ctrl_key(), k.meta_key()) {
            e.prevent_default();
            toggle();
        } else if k.key() == "Escape" && is_open() {
            e.prevent_default();
            close();
        }
    });
    if let Some(body) = rt::document().and_then(|d| d.body()) {
        let open = OPEN.with(Clone::clone);
        rt::keep("palette", rt::mount(&body, move || move || open.get().then(palette_view)));
    }
}

/// Replaces the fixed commands (say, after signing in as an admin); the source stays.
pub fn set_commands(commands: Vec<Command>) {
    STATE.with(|s| s.borrow_mut().commands = commands);
    changed();
}

/// Whether the palette is open.
pub fn is_open() -> bool {
    OPEN.with(|o| o.get_untracked())
}

/// Opens the palette with an empty query and the focus in its box, and asks the source (nothing when it is
/// open already).
pub fn open() {
    if is_open() {
        return;
    }
    let (n, source) = STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.opening += 1;
        s.focus = rt::document().and_then(|d| d.active_element()).and_then(|e| e.dyn_into().ok());
        (s.opening, s.source.clone())
    });
    OPEN.with(|o| o.set(true));
    rt::after(0, || {
        if let Some(i) = rt::qs(".palette-input").and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok()) {
            let _ = i.focus();
        }
    });
    if let Some(source) = source {
        rt::spawn(async move {
            let mut got = source().await;
            for c in &mut got {
                c.item.dynamic = true;
            }
            let fresh = STATE.with(|s| {
                let mut s = s.borrow_mut();
                let fresh = s.opening == n;
                if fresh {
                    s.loaded = got;
                }
                fresh
            });
            if fresh {
                changed();
            }
        });
    }
}

/// Closes the palette; the focus goes back where it was.
pub fn close() {
    if !is_open() {
        return;
    }
    OPEN.with(|o| o.set(false));
    if let Some(f) = STATE.with(|s| s.borrow_mut().focus.take()) {
        let _ = f.focus();
    }
}

/// Opens the palette when it is closed, closes it when it is open.
pub fn toggle() {
    if is_open() {
        close()
    } else {
        open()
    }
}

/// Every command in the order [`results`] indexes them: the fixed ones, then the loaded ones.
fn items() -> Vec<Item> {
    STATE.with(|s| {
        let s = s.borrow();
        s.commands.iter().chain(&s.loaded).map(|c| c.item.clone()).collect()
    })
}

fn command(i: usize) -> Option<Command> {
    STATE.with(|s| {
        let s = s.borrow();
        s.commands.iter().chain(&s.loaded).nth(i).cloned()
    })
}

/// The id of the option at `pos` in the list.
fn opt_id(pos: usize) -> String {
    format!("palette-opt-{pos}")
}

/// Brings the option at `pos` into view inside the list.
fn scroll_to(pos: usize) {
    if let Some(el) = rt::by_id(&opt_id(pos)) {
        let o = web_sys::ScrollIntoViewOptions::new();
        o.set_block(web_sys::ScrollLogicalPosition::Nearest);
        el.scroll_into_view_with_scroll_into_view_options(&o);
    }
}

/// The open palette: `.palette-bg` › `.palette` (a dialog) with the search box (a combobox) and the results (a
/// listbox of options under their groups, or the empty line). [`install`] draws it while the palette is open.
pub fn palette_view() -> impl IntoView {
    let w = crate::config::get().words.palette;
    let query = RwSignal::new(String::new());
    let sel = RwSignal::new(0usize);
    let rev = REV.with(Clone::clone);
    // (index into the commands, what it shows), in the order shown
    let shown = Memo::new(move |_| {
        rev.track();
        let all = items();
        results(&all, &query.get()).into_iter().map(|i| (i, all[i].clone())).collect::<Vec<_>>()
    });
    let run_at = move |pos: usize| {
        let Some(c) = shown.with_untracked(|r| r.get(pos).map(|(i, _)| *i)).and_then(command) else { return };
        close();
        c.run();
    };
    let on_key = move |e: web_sys::KeyboardEvent| {
        let m = match e.key().as_str() {
            "ArrowDown" => Move::Down,
            "ArrowUp" => Move::Up,
            "Enter" => {
                e.prevent_default();
                run_at(sel.get_untracked());
                return;
            }
            _ => return,
        };
        e.prevent_default();
        let len = shown.with_untracked(Vec::len);
        let next = step(sel.get_untracked(), len, m);
        sel.set(next);
        scroll_to(next);
    };
    let list = move || {
        let rows = shown.get();
        if rows.is_empty() {
            return view! { <div class="palette-empty">{w.empty}</div> }.into_any();
        }
        // consecutive rows of one group go under one heading
        let mut groups: Vec<(String, Vec<(usize, Item)>)> = Vec::new();
        for (pos, (_, it)) in rows.into_iter().enumerate() {
            match groups.last_mut() {
                Some((g, v)) if *g == it.group => v.push((pos, it)),
                _ => groups.push((it.group.clone(), vec![(pos, it)])),
            }
        }
        let groups = groups.into_iter().enumerate().map(|(n, (g, opts))| {
            let (gid, labelled) = (format!("palette-g-{n}"), format!("palette-g-{n}"));
            let opts = opts.into_iter().map(|(pos, it)| {
                let svg = icon(&it.icon);
                let ic = (!svg.is_empty()).then(|| view! { <span class="palette-icon" {..html(svg)}>{""}</span> });
                let hint = (!it.hint.is_empty()).then(|| view! { <span class="palette-hint">{it.hint.clone()}</span> });
                view! {
                    <div class="palette-item" id=opt_id(pos) role="option"
                        aria-selected=move || if sel.get() == pos { "true" } else { "false" }
                        on:mousemove=move |_| if sel.get_untracked() != pos { sel.set(pos) }
                        on:click=move |_| run_at(pos)>
                        {opt(ic)}<span class="palette-label">{it.label.clone()}</span>{opt(hint)}
                    </div>
                }
            });
            view! {
                <div class="palette-section" role="group" aria-labelledby=labelled>
                    <div class="palette-group" id=gid role="presentation">{g}</div>
                    {seq(opts)}
                </div>
            }
        });
        view! { <div class="palette-list" id="palette-list" role="listbox" aria-label={w.title}>{seq(groups)}</div> }.into_any()
    };
    view! {
        <div class="palette-bg" on:mousedown=move |e| {
            if e.target() == e.current_target() {
                close();
            }
        }>
            <div class="palette" role="dialog" aria-modal="true" aria-label={w.title}>
                <div class="palette-search">
                    <span class="palette-search-icon" {..html(icon("search"))}>{""}</span>
                    <input class="palette-input" type="text" role="combobox" aria-expanded="true" aria-controls="palette-list"
                        aria-autocomplete="list" autocomplete="off" spellcheck="false" placeholder={w.placeholder}
                        aria-activedescendant=move || opt_id(sel.get())
                        on:input=move |e| {
                            query.set(event_target_value(&e));
                            sel.set(0);
                        }
                        on:keydown=on_key/>
                </div>
                {list}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{assert_same_html, render};

    fn reset(commands: Vec<Command>, loaded: Vec<Command>) {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            s.commands = commands;
            s.loaded = loaded.into_iter().map(|mut c| (c.item.dynamic = true, c).1).collect();
        });
    }

    fn head(rest: &str) -> String {
        format!(
            r#"<div class="palette-bg"><div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
            <div class="palette-search"><span class="palette-search-icon">{search}</span>
            <input class="palette-input" type="text" role="combobox" aria-expanded="true" aria-controls="palette-list"
              aria-autocomplete="list" autocomplete="off" spellcheck="false" placeholder="Type a command or search…"
              aria-activedescendant="palette-opt-0"></div>{rest}</div></div>"#,
            search = icon("search"),
        )
    }

    #[test]
    fn the_palette_lists_its_commands_under_their_groups() {
        let mut loaded: Vec<Command> = (1..=7).map(|n| Command::new("Sites", format!("Site {n}"), || {})).collect();
        loaded[0] = loaded[0].clone().hint("one.example");
        reset(
            vec![
                Command::new(ACTIONS, "New <note>", || {}).icon("plus"),
                Command::new(GO_TO, "Settings", || {}).hint("/settings").icon("no-such-icon"),
            ],
            loaded,
        );
        let html = render(palette_view);
        let mut sites = String::new();
        for n in 1..=6 {
            let hint = if n == 1 { r#"<span class="palette-hint">one.example</span>"# } else { "" };
            sites.push_str(&format!(
                r#"<div class="palette-item" id="palette-opt-{p}" role="option" aria-selected="false"><span class="palette-label">Site {n}</span>{hint}</div>"#,
                p = n + 1
            ));
        }
        let want = head(&format!(
            r#"<div class="palette-list" id="palette-list" role="listbox" aria-label="Command palette">
            <div class="palette-section" role="group" aria-labelledby="palette-g-0"><div class="palette-group" id="palette-g-0" role="presentation">Actions</div>
              <div class="palette-item" id="palette-opt-0" role="option" aria-selected="true"><span class="palette-icon">{plus}</span><span class="palette-label">New &lt;note&gt;</span></div></div>
            <div class="palette-section" role="group" aria-labelledby="palette-g-1"><div class="palette-group" id="palette-g-1" role="presentation">Go to</div>
              <div class="palette-item" id="palette-opt-1" role="option" aria-selected="false"><span class="palette-label">Settings</span><span class="palette-hint">/settings</span></div></div>
            <div class="palette-section" role="group" aria-labelledby="palette-g-2"><div class="palette-group" id="palette-g-2" role="presentation">Sites</div>{sites}</div>
            </div>"#,
            plus = icon("plus"),
        ));
        assert_same_html(&html, &want);
        reset(Vec::new(), Vec::new());
    }

    #[test]
    fn nothing_to_show_says_so_in_the_configured_words() {
        reset(Vec::new(), Vec::new());
        assert_same_html(&render(palette_view), &head(r#"<div class="palette-empty">Nothing matches</div>"#));
        let mut cfg = crate::config::AppConfig::default();
        cfg.words.palette = kit_rules::palette::PaletteWords { title: "Befehle", placeholder: "Suchen…", empty: "Nichts gefunden" };
        crate::config::set(cfg);
        let html = render(palette_view);
        assert!(
            html.contains(r#"aria-label="Befehle""#) && html.contains(r#"placeholder="Suchen…""#) && html.contains("Nichts gefunden"),
            "{html}"
        );
        crate::config::set(crate::config::AppConfig::default());
    }

    #[test]
    fn a_command_runs_its_action_and_keeps_its_item() {
        let hits = Rc::new(Cell::new(0));
        let h = hits.clone();
        let c = Command::new(GO_TO, "Settings", move || h.set(h.get() + 1)).hint("/settings").icon("settings");
        c.run();
        c.clone().run();
        assert_eq!(hits.get(), 2);
        assert_eq!(c.item, Item::new(GO_TO, "Settings").hint("/settings").icon("settings"));
        assert!(format!("{c:?}").contains("Settings"));
        // the fixed commands come first, then the loaded ones, as `results` indexes them
        reset(vec![c], vec![Command::new("Sites", "Home", || {})]);
        assert_eq!(items().iter().map(|i| (i.label.as_str(), i.dynamic)).collect::<Vec<_>>(), [("Settings", false), ("Home", true)]);
        assert_eq!(command(1).map(|c| c.item.label), Some("Home".to_string()));
        assert!(command(2).is_none());
        reset(Vec::new(), Vec::new());
    }

    #[test]
    fn open_and_close_off_the_browser_only_flip_the_state() {
        reset(Vec::new(), Vec::new());
        assert!(!is_open());
        toggle();
        assert!(is_open());
        open();
        assert!(is_open());
        toggle();
        assert!(!is_open());
        close();
        assert!(!is_open());
    }
}
