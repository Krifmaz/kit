//! Drag and drop for every file upload: files dragged from the computer onto the window go to the
//! upload of the page (or dialog) that is open, checked like the file dialog checks them.
//!
//! The window is one drop target (the system tells the app that files are over it or were dropped,
//! not where), so a page takes part by
//! 1. keeping a [`DropZone`] in its state and listening to [`events`] while its upload can be used,
//! 2. handing each [`Event`] to [`DropZone::handle`] with the [`Accept`] rule of its file dialog
//!    (the same extensions as the dialog's filter), which answers what to do with it ([`Outcome`]),
//! 3. drawing the upload's part of the page through [`view`], which outlines it in the accent colour
//!    with a "Drop … here" line while files are held over the window.
//!
//! A path taken from a drop goes into the same message the file dialog's answer goes to, so a dropped
//! file and a picked one take one code path.

use crate::theme;
use crate::widgets::{icon, t, El, Icon};
use iced::widget::{column, container, row};
use iced::{Alignment, Background, Border, Length, Subscription};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub use kit_rules::uploads::Accept;

/// Whether `accept` takes this path (any case). Folders are never taken.
pub fn takes(accept: &Accept, path: &Path) -> bool {
    accept.takes(&path.to_string_lossy(), path.is_dir())
}

/// What the system says about files dragged over the window.
#[derive(Debug, Clone)]
pub enum Event {
    /// Files are held over the window (once per file).
    Hovered,
    /// They left the window without being dropped.
    Left,
    /// One file was dropped (a drop of several sends one per file, one right after the other).
    Dropped(PathBuf),
}

/// The window's drag and drop events, for a page's subscription.
pub fn events() -> Subscription<Event> {
    iced::event::listen_with(|e, _, _| match e {
        iced::Event::Window(iced::window::Event::FileHovered(_)) => Some(Event::Hovered),
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => Some(Event::Left),
        iced::Event::Window(iced::window::Event::FileDropped(p)) => Some(Event::Dropped(p)),
        _ => None,
    })
}

/// What a page does with one [`Event`].
pub type Outcome = kit_rules::uploads::Outcome<PathBuf>;

/// A page's drop place: whether files are held over the window, and how many files the current
/// drop already handed over (an upload of one file takes only the first). The counting is
/// kit-rules' `uploads::DropZone`.
#[derive(Debug, Default, Clone)]
pub struct DropZone {
    /// Files are held over the window: [`view`] shows the highlight (a page may switch it off).
    pub hovering: bool,
    rules: kit_rules::uploads::DropZone,
}

/// The clock of drops: milliseconds since the first drop event of this process.
fn now_ms() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}

impl DropZone {
    /// Takes one event under the upload's rule and answers what to do with it.
    pub fn handle(&mut self, ev: Event, accept: &Accept) -> Outcome {
        self.handle_at(ev, accept, now_ms())
    }

    /// [`DropZone::handle`] with the time given (for the tests).
    fn handle_at(&mut self, ev: Event, accept: &Accept, now: u64) -> Outcome {
        self.rules.hovering = self.hovering;
        let out = match ev {
            Event::Hovered => {
                self.rules.hovered();
                Outcome::Nothing
            }
            Event::Left => {
                self.rules.left();
                Outcome::Nothing
            }
            Event::Dropped(path) => {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let is_dir = path.is_dir();
                self.rules.dropped(path, &name, is_dir, accept, now)
            }
        };
        self.hovering = self.rules.hovering;
        out
    }
}

/// Draws `content` as a drop place: while files are held over the window it gets an accent outline,
/// a soft accent wash and a line saying `hint` ("Drop a .zip here to upload it"). The outline's room is
/// kept when there is none, so nothing moves when it appears.
pub fn view<'a, M: 'a>(content: impl Into<El<'a, M>>, zone: &DropZone, hint: &'a str) -> El<'a, M> {
    let p = theme::look();
    let on = zone.hovering;
    let mut body = column![].spacing(10);
    if on {
        body = body
            .push(row![icon(Icon::Plus, 16.0, p.accent), t(hint).size(13).font(theme::semibold())].spacing(8).align_y(Alignment::Center));
    }
    body = body.push(content);
    container(body)
        .width(Length::Fill)
        .padding(if on { 10 } else { 0 })
        .style(move |_| container::Style {
            background: on.then(|| Background::Color(p.accent.scale_alpha(0.08))),
            border: Border {
                color: if on { p.accent } else { iced::Color::TRANSPARENT },
                width: if on { 2.0 } else { 0.0 },
                radius: 10.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZIP: Accept = Accept::one(&["zip"]);
    const PICS: Accept = Accept::many(&["png", "jpg", "txt", "log"]);

    #[test]
    fn endings_are_checked_in_any_case() {
        assert!(takes(&ZIP, Path::new("/tmp/Plugin.ZIP")));
        assert!(!takes(&ZIP, Path::new("/tmp/plugin.tar.gz")));
        assert!(!takes(&ZIP, Path::new("/tmp/zip")));
        assert!(takes(&Accept::one(&[]), Path::new("/tmp/anything.bin")));
        assert_eq!(PICS.hint(), ".png, .jpg, .txt, .log");
    }

    #[test]
    fn folders_are_never_taken() {
        assert!(!takes(&Accept::one(&[]), &std::env::temp_dir()));
    }

    #[test]
    fn hover_lights_up_and_leaving_turns_it_off() {
        let mut z = DropZone::default();
        assert_eq!(z.handle(Event::Hovered, &ZIP), Outcome::Nothing);
        assert!(z.hovering);
        z.handle(Event::Left, &ZIP);
        assert!(!z.hovering);
    }

    #[test]
    fn a_single_upload_takes_the_first_good_file_of_a_drop() {
        let mut z = DropZone::default();
        let now = 10_000u64;
        z.handle_at(Event::Hovered, &ZIP, now);
        assert!(
            matches!(z.handle_at(Event::Dropped("/a/notes.txt".into()), &ZIP, now), Outcome::Refused(m) if m == "notes.txt can't be uploaded here (it takes .zip)")
        );
        assert_eq!(z.handle_at(Event::Dropped("/a/one.zip".into()), &ZIP, now), Outcome::Take("/a/one.zip".into()));
        assert!(matches!(z.handle_at(Event::Dropped("/a/two.zip".into()), &ZIP, now), Outcome::Refused(_)));
        assert!(!z.hovering);
        // the next drag is a new drop
        z.handle_at(Event::Hovered, &ZIP, now);
        assert_eq!(z.handle_at(Event::Dropped("/a/two.zip".into()), &ZIP, now), Outcome::Take("/a/two.zip".into()));
    }

    #[test]
    fn a_pause_starts_a_new_drop_without_a_hover() {
        let mut z = DropZone::default();
        let now = 10_000u64;
        assert!(matches!(z.handle_at(Event::Dropped("/a/one.zip".into()), &ZIP, now), Outcome::Take(_)));
        let later = now + 2000;
        assert!(matches!(z.handle_at(Event::Dropped("/a/two.zip".into()), &ZIP, later), Outcome::Take(_)));
    }

    #[test]
    fn a_multiple_upload_takes_every_good_file() {
        let mut z = DropZone::default();
        let now = 10_000u64;
        z.handle_at(Event::Hovered, &PICS, now);
        for f in ["/a/1.png", "/a/2.JPG", "/a/log.log"] {
            assert_eq!(z.handle_at(Event::Dropped(f.into()), &PICS, now), Outcome::Take(f.into()));
        }
        assert!(matches!(z.handle_at(Event::Dropped("/a/x.exe".into()), &PICS, now), Outcome::Refused(_)));
    }
}
