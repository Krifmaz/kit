//! The offline banner: while the server can't be reached, a line over the page (not over the sidebar)
//! in the amber box, saying what still works, with the app's buttons (Encore: Offline music and Retry).
//! Behaviour reference: Encore's desktop `App::offline_banner` (clients/desktop/src/app.rs).
//!
//! Why the server can't be reached (no network, DNS, refused, timeout, HTTPS, proxy 502–504) and
//! the outbox of actions to send later are kit-rules' offline rules; this is how a desktop app shows it.

use crate::theme::{self, Btn};
use crate::widgets::{btn, t, El};
use iced::widget::{container, row};
use iced::{Alignment, Length};

/// Encore's banner sentence.
pub const ENCORE_TEXT: &str = "Offline – showing what this computer saved. Songs saved for offline play.";
/// Encore's Retry.
pub const RETRY: &str = "Retry";

/// The banner: `text` in the warning colour, then the buttons, each `(label, primary, message)`
/// (Encore: `[("Offline music", false, …), ("Retry", true, …)]`). Lay it over the page with
/// `container(banner).padding([8, 16])`, as Encore does.
pub fn banner<'a, M: Clone + 'a>(text: &'a str, buttons: Vec<(&'a str, bool, M)>) -> El<'a, M> {
    let p = theme::look();
    let mut r = row![t(text).size(14).color(p.warn).width(Length::Fill)].spacing(12).align_y(Alignment::Center);
    for (label, primary, on) in buttons {
        r = r.push(btn(label, if primary { Btn::Primary } else { Btn::Default }, Some(on)));
    }
    container(r).padding([10, 16]).width(Length::Fill).style(theme::warn_box).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_builds_with_encores_words() {
        assert_eq!(ENCORE_TEXT, "Offline – showing what this computer saved. Songs saved for offline play.");
        let _ = banner(ENCORE_TEXT, vec![("Offline music", false, 1u8), (RETRY, true, 2u8)]);
    }
}
