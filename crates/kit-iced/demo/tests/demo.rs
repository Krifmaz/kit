//! The demo drawn headless: every page shows its component, and the main flows work end to end.

use iced::widget::text_editor::{Action, Edit};
use iced_test::simulator;
use kit_iced::details::{COPIED, COPY};
use kit_iced::drop_zone::Event;
use kit_iced::notice::{CHECKING, CHECK_AGAIN};
use kit_iced::toast::ToastKind;
use kit_iced::update::{describe, Phase};
use kit_iced::{bug_report, diagnostics, form, offline, whats_new};
use kit_iced_demo::*;
use std::path::PathBuf;
use std::sync::Arc;

/// Whether the window shows `word`.
fn shows(demo: &Demo, word: &str) -> bool {
    simulator(demo.view()).find(word).is_ok()
}

/// Click `word` in the window and hand what it sent back to the demo.
fn click(demo: &mut Demo, word: &str) -> Vec<Message> {
    let sent: Vec<Message> = {
        let mut ui = simulator(demo.view());
        ui.click(word).unwrap_or_else(|e| panic!("{word}: {e:?}"));
        ui.into_messages().collect()
    };
    for m in sent.clone() {
        let _ = demo.update(m);
    }
    sent
}

/// The demo on a page.
fn on(page: Page) -> Demo {
    let mut demo = Demo::new();
    let _ = demo.update(Message::Go(page));
    demo
}

/// The fake server answers.
fn tick(demo: &mut Demo) {
    let _ = demo.update(Message::Tick);
}

/// Click every toast away (they sit over the bottom right of the page).
fn clear_toasts(demo: &mut Demo) {
    let ids: Vec<u64> = demo.toasts().list().iter().map(|t| t.id).collect();
    for id in ids {
        let _ = demo.update(Message::Dismiss(id));
    }
}

#[test]
fn the_sidebar_opens_every_page() {
    let mut demo = Demo::new();
    for page in Page::ALL {
        click(&mut demo, page.label());
        assert_eq!(demo.page(), page, "{}", page.label());
    }
}

#[test]
fn appearance_shows_every_button_icon_and_badge_and_switches_theme() {
    let mut demo = on(Page::Appearance);
    for word in ["Primary", "Danger", "Ghost", "Play", "Accent", "Skeletons", "Logo", "home", "bug", "video", "A card", "Err"] {
        assert!(shows(&demo, word), "{word}");
    }
    assert!(demo.palette().dark);
    click(&mut demo, "Light");
    assert!(!demo.palette().dark, "the light theme is on");
    let _ = demo.update(Message::Accent("teal".into()));
    assert_eq!(kit_iced::theme::to_hex(demo.palette().accent), "#14b8a6");
    assert!(shows(&demo, "Accent: Teal"));
    let _ = demo.update(Message::Theme("oled".into()));
    assert_eq!(kit_iced::theme::to_hex(demo.palette().bg), "#000000");
    click(&mut demo, "Ghost");
    assert!(shows(&demo, "Ghost pressed"));
}

#[test]
fn an_error_toast_stays_with_copy() {
    let mut demo = on(Page::Toasts);
    assert!(!shows(&demo, COPY));
    click(&mut demo, "Show an error");
    assert!(shows(&demo, ERROR_TEXT));
    let sent = click(&mut demo, COPY);
    assert!(sent.iter().any(|m| matches!(m, Message::CopyToast(_))));
    // the clipboard read back holds the text: the copy took
    let _ = demo.update(Message::ClipboardRead(ERROR_TEXT.into(), Some(ERROR_TEXT.into())));
    assert!(shows(&demo, COPIED));
    tick(&mut demo);
    assert!(shows(&demo, ERROR_TEXT), "an error stays");
}

#[test]
fn a_repeated_error_does_not_pile_up_and_undo_comes_back() {
    let mut demo = on(Page::Toasts);
    click(&mut demo, "Show an error");
    click(&mut demo, "Show the same error again");
    assert_eq!(demo.toasts().list().iter().filter(|t| t.text == ERROR_TEXT).count(), 1);
    click(&mut demo, "Show a success toast");
    assert!(shows(&demo, "Saved"));
    click(&mut demo, "Show an Undo toast");
    assert!(shows(&demo, UNDO_TEXT));
    click(&mut demo, "Undo");
    assert!(shows(&demo, UNDONE_TEXT));
    assert!(!shows(&demo, UNDO_TEXT), "the Undo toast closed");
}

#[test]
fn the_details_dialog_shows_the_text_lead_action_and_links() {
    let mut demo = on(Page::Details);
    click(&mut demo, "Open details");
    for word in [DETAILS_TITLE, DETAILS_LEAD, "Keep anyway", "report.zip", "Uploads", "Open diagnostics", COPY] {
        assert!(shows(&demo, word), "{word}");
    }
    assert_eq!(demo.details().map(|d| d.text.as_str()), Some(DETAILS_TEXT));
    assert_eq!(demo.details().unwrap().content().text().trim_end(), DETAILS_TEXT);
    click(&mut demo, "Keep anyway");
    assert!(demo.details().is_none(), "the action closes the dialog");
    tick(&mut demo);
    assert!(shows(&demo, "The first 14 files are kept"));
    // a link opens its page
    click(&mut demo, "Open details");
    click(&mut demo, "Open diagnostics");
    assert_eq!(demo.page(), Page::Diagnostics);
}

#[test]
fn the_confirm_dialog_deletes_on_the_red_button() {
    let mut demo = on(Page::Confirm);
    click(&mut demo, "Delete report…");
    assert!(shows(&demo, "Delete report?"));
    assert!(shows(&demo, "It's gone for everyone. This can't be undone."));
    click(&mut demo, "Delete");
    assert!(!demo.confirm_open());
    assert!(shows(&demo, DELETED));
    // Cancel does nothing
    click(&mut demo, "Sign out everywhere…");
    click(&mut demo, "Cancel");
    assert!(!demo.confirm_open());
    assert!(!shows(&demo, "Signed out on every device"));
}

#[test]
fn whats_new_shows_the_changelog_and_the_popup() {
    let mut demo = on(Page::WhatsNew);
    assert!(shows(&demo, "Demo 0.2") && shows(&demo, "Demo 0.1"));
    assert!(shows(&demo, &whats_new::runs_words(APP, SERVER_VERSION)));
    assert!(shows(&demo, "Upload size"), "the newest release is open, its table drawn");
    click(&mut demo, "Demo 0.1");
    assert!(shows(&demo, "The first release."), "an older release opens");
    click(&mut demo, "Show the popup");
    assert!(demo.popup_open());
    assert!(shows(&demo, "Lists load twice as fast"));
    assert!(shows(&demo, whats_new::FULL_CHANGELOG));
    let _ = demo.update(Message::Popup(whats_new::PopupMsg::Popup(false)));
    assert!(shows(&demo, whats_new::saved_words(false)));
    click(&mut demo, whats_new::FULL_CHANGELOG);
    assert!(!demo.popup_open());
}

#[test]
fn updates_find_the_release_and_step_through_the_phases() {
    let mut demo = on(Page::Updates);
    assert!(shows(&demo, "Version 0.1.0"));
    assert!(shows(&demo, &describe(&Phase::UpToDate, APP)), "the words table");
    click(&mut demo, "Check for updates");
    assert_eq!(demo.phase(), &Phase::Checking);
    tick(&mut demo);
    assert!(matches!(demo.phase(), Phase::Available(r) if r.version == "0.2.0"), "the hotfix is passed over");
    assert!(shows(&demo, "Install 0.2.0 and restart"));
    assert!(shows(&demo, "Demo 0.2.0 is ready to install. Settings → Updates."));
    click(&mut demo, "Install 0.2.0 and restart");
    assert!(shows(&demo, &describe(demo.phase(), APP)));
    // four steps of the download, then the check, the install and a failed check
    for _ in 0..7 {
        click(&mut demo, "Next phase");
    }
    assert!(matches!(demo.phase(), Phase::Failed(e) if e.contains("does not match")));
}

#[test]
fn the_notice_checks_again_then_toasts() {
    let mut demo = on(Page::Notices);
    assert!(shows(&demo, NOTICE_TEXT));
    click(&mut demo, CHECK_AGAIN);
    assert!(shows(&demo, CHECKING), "the button says a check runs");
    tick(&mut demo);
    assert!(shows(&demo, NOTICE_WORDS.failing));
    assert!(shows(&demo, CHECK_AGAIN));
    click(&mut demo, CHECK_AGAIN);
    tick(&mut demo);
    assert!(shows(&demo, NOTICE_WORDS.checking));
    click(&mut demo, CHECK_AGAIN);
    tick(&mut demo);
    assert!(shows(&demo, NOTICE_WORDS.fixed));
    assert!(!shows(&demo, NOTICE_TEXT), "fixed: the banner is gone");
    click(&mut demo, "Bring the notice back");
    assert!(shows(&demo, NOTICE_TEXT));
}

#[test]
fn offline_shows_the_banner_until_retry() {
    let mut demo = on(Page::Offline);
    assert!(!shows(&demo, offline::TEXT));
    click(&mut demo, "Go offline");
    assert!(shows(&demo, offline::TEXT));
    let _ = demo.update(Message::Go(Page::Settings));
    assert!(shows(&demo, offline::TEXT), "the banner is over every page");
    click(&mut demo, offline::RETRY);
    assert!(!shows(&demo, offline::TEXT));
}

#[test]
fn drag_and_drop_takes_images_and_refuses_the_rest() {
    let mut demo = on(Page::Drop);
    assert!(!shows(&demo, DROP_HINT));
    click(&mut demo, "Hold a file over the window");
    assert!(shows(&demo, DROP_HINT), "the zone lights up");
    let _ = demo.update(Message::Drop(Event::Dropped(PathBuf::from("/home/demo/photo.png"))));
    assert_eq!(demo.dropped(), [PathBuf::from("/home/demo/photo.png")]);
    assert!(shows(&demo, "photo.png") && shows(&demo, "photo.png added"));
    click(&mut demo, "Drop notes.txt");
    assert_eq!(demo.dropped().len(), 1);
    assert!(demo.toasts().list().iter().any(|t| t.kind == ToastKind::Err && t.text.starts_with("notes.txt can't be uploaded here")));
    // a drop on another page goes nowhere
    let _ = demo.update(Message::Go(Page::Toasts));
    let _ = demo.update(Message::Drop(Event::Dropped(PathBuf::from("/home/demo/two.png"))));
    assert_eq!(demo.dropped().len(), 1);
}

#[test]
fn the_settings_pieces_and_the_form_draw() {
    let demo = on(Page::Settings);
    for word in [
        "Pieces",
        "A test that passed",
        "A test that failed: connection timed out",
        "General",
        "Email",
        "Services",
        "App name",
        "Send test email",
    ] {
        assert!(shows(&demo, word), "{word}");
    }
    // the first card is open; the base URL is set by the environment
    assert!(shows(&demo, "Demo"), "the app name field holds its value");
    assert!(shows(&demo, "🔒 Set by DEMO_BASE_URL in your compose/env – change it there (environment always wins)"));
    assert!(!shows(&demo, "STARTTLS port"), "the Email card is folded");
}

#[test]
fn the_form_saves_and_tests_with_fake_answers() {
    let mut demo = on(Page::Settings);
    click(&mut demo, "Save");
    assert!(shows(&demo, "Saving…"));
    tick(&mut demo);
    assert!(shows(&demo, form::SAVED));
    clear_toasts(&mut demo);
    click(&mut demo, "Send test email");
    assert!(shows(&demo, form::TESTING));
    tick(&mut demo);
    assert!(shows(&demo, "Test email sent through smtp.example.com"));
    // the when rule: the port fields follow the Security select
    assert!(shows(&demo, "STARTTLS port") && !shows(&demo, "TLS port"));
    let _ = demo.update(Message::Form(form::Msg::Edit("smtp_security".into(), "tls".into())));
    assert!(shows(&demo, "TLS port") && !shows(&demo, "STARTTLS port"));
    // no SMTP server: the test fails, with Copy
    let _ = demo.update(Message::Form(form::Msg::Edit("smtp_host".into(), String::new())));
    clear_toasts(&mut demo);
    click(&mut demo, "Send test email");
    tick(&mut demo);
    assert!(shows(&demo, "No SMTP server is set, so no email could be sent"));
}

#[test]
fn a_new_report_is_checked_sent_and_shown() {
    let mut demo = on(Page::BugReport);
    assert!(shows(&demo, "The list jumps to the top after a save"));
    assert!(shows(&demo, "by sam · 18 Oct 2026 · page #/items · app 0.2.0 · Demo desktop · 1280x800"));
    assert!(shows(&demo, "Won't fix") && shows(&demo, "Crash log"));
    click(&mut demo, "Write a report");
    assert!(demo.new_report_open());
    assert!(shows(&demo, "New report"));
    assert!(shows(&demo, "Sent along: the page you came from (#/items), the app version (0.2.0) and this computer."));
    let _ = demo.update(Message::New(bug_report::NewMsg::Send));
    assert!(shows(&demo, "Give the report a title"));
    let _ = demo.update(Message::New(bug_report::NewMsg::Title("Search misses accents".into())));
    let _ = demo.update(Message::New(bug_report::NewMsg::Body(Action::Edit(Edit::Paste(Arc::new("Searching cafe misses café.".into()))))));
    let _ = demo.update(Message::New(bug_report::NewMsg::AddFiles));
    tick(&mut demo);
    assert!(shows(&demo, "recording.gif is over 8 MB"), "the picked file over the limit is refused");
    let _ = demo.update(Message::New(bug_report::NewMsg::Send));
    assert!(shows(&demo, "Sending…"));
    tick(&mut demo);
    assert!(!demo.new_report_open());
    assert!(shows(&demo, "Report sent. Thanks."));
    assert!(shows(&demo, "Search misses accents"));
    assert_eq!(demo.report()["id"], 8);
}

#[test]
fn a_reply_is_sent_and_joins_the_thread() {
    let mut demo = on(Page::BugReport);
    click(&mut demo, "Send");
    assert!(shows(&demo, "Write a reply or add a file"));
    let _ = demo.update(Message::Reply(bug_report::ReplyMsg::Text(Action::Edit(Edit::Paste(Arc::new("Still happens in 0.2.0.".into()))))));
    click(&mut demo, "Send");
    assert!(shows(&demo, "Sending…"));
    tick(&mut demo);
    assert!(shows(&demo, "Reply sent"));
    assert!(shows(&demo, "Still happens in 0.2.0."));
}

#[test]
fn diagnostics_counts_the_problem_and_copies_the_report() {
    let mut demo = on(Page::Diagnostics);
    for word in [
        "1 problem",
        "Mail server",
        "Database",
        "Data folder",
        "Connections",
        "Folders & permissions",
        "Tools in the container",
        "uid 1000",
    ] {
        assert!(shows(&demo, word), "{word}");
    }
    assert!(shows(&demo, "Demo 0.2.0 · running as uid 1000"));
    let sent = click(&mut demo, diagnostics::COPY_REPORT);
    assert!(sent.iter().any(|m| matches!(m, Message::Diag(diagnostics::Msg::CopyReport))));
    click(&mut demo, diagnostics::RUN);
    assert!(shows(&demo, diagnostics::RUNNING));
    tick(&mut demo);
    assert!(shows(&demo, diagnostics::RUN) && shows(&demo, "1 problem"));
}

#[test]
fn linked_apps_show_in_one_sidebar_and_one_search() {
    let mut demo = on(Page::Suite);
    for word in ["Things", "Items", "Other demo", "Edit sidebar"] {
        assert!(shows(&demo, word), "{word}");
    }
    assert!(!shows(&demo, "More"), "a hidden entry shows only while editing");
    click(&mut demo, "Items");
    assert!(demo.toasts().list().iter().any(|t| t.text.contains("http://other.test/#items")), "another app opens in the browser");
    clear_toasts(&mut demo);
    click(&mut demo, "Edit sidebar");
    assert!(shows(&demo, "More") && shows(&demo, "Done"));
    click(&mut demo, "Show");
    click(&mut demo, "Done");
    tick(&mut demo);
    assert!(shows(&demo, "More"), "shown again after Done");
    let _ = demo.update(Message::Search(kit_iced::suite::SearchMsg::Text("thing".into())));
    let _ = demo.update(Message::Search(kit_iced::suite::SearchMsg::Submit));
    tick(&mut demo);
    for word in [
        "First thing",
        "A related item",
        "Third demo",
        "http://third.test can't be reached. Check the address and that the app is running.",
    ] {
        assert!(shows(&demo, word), "{word}");
    }
}
