//! The notice banner: a plain-language sentence when something on the server is wrong ("the data
//! folder is read-only – check the disk…"), in an amber box with Check again for the people who can
//! fix it (contracts/notices.md); which notices show, when to ask again and Check again's toast are
//! kit-rules' `notices`, re-exported here.
//!
//! The app asks for the notices (`GET /api/notices`, or its own path), keeps them as [`Notice`]s,
//! and draws [`banner`] over its page with the sentences of [`banners`]. Check again sends the app's
//! message; the app posts the check (`POST /api/notices/check`) and shows the toast of [`check_toast`].

use crate::theme::{self, Btn};
use crate::widgets::{btn_sm, icon, t, El, Icon};
use iced::widget::{container, row};
use iced::{Alignment, Length};

pub use kit_rules::notices::{banners, check_toast, CheckWords, Notice, NoticePoll, Severity, CHECKING, CHECK_AGAIN};

/// The notices of `{"notices": [...]}` (the kit's default path); anything unreadable is left out.
pub fn list(v: &serde_json::Value) -> Vec<Notice> {
    v["notices"].as_array().into_iter().flatten().filter_map(|n| serde_json::from_value(n.clone()).ok()).collect()
}

/// The banner: `glyph` and the sentences (one per line) in the amber box, and Check again when
/// `check` is given (`Some((checking, message))`: the button reads [`CHECKING`] and is disabled while
/// a check runs). None when there is nothing to say.
pub fn banner<'a, M: Clone + 'a>(glyph: Icon, lines: Vec<String>, check: Option<(bool, M)>) -> Option<El<'a, M>> {
    let lines: Vec<String> = lines.into_iter().filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return None;
    }
    let p = theme::look();
    let mut r = row![
        icon(glyph, 18.0, p.warn),
        t(lines.join("\n")).size(13.5).width(Length::Fill).wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if let Some((checking, on)) = check {
        r = r.push(btn_sm(if checking { CHECKING } else { CHECK_AGAIN }, Btn::Default, (!checking).then_some(on)));
    }
    Some(container(r).padding([12, 16]).width(Length::Fill).style(theme::warn_box).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn notices_show_for_admins_when_failing_with_words() {
        let all =
            list(&json!({ "notices": [{ "ok": false, "text": "a" }, { "ok": true }, { "ok": false, "text": "b", "severity": "err" }] }));
        let shown: Vec<&str> = banners(&all, true).iter().filter_map(|n| n.banner()).collect();
        assert_eq!(shown, ["a", "b"]);
        assert!(banners(&all, false).is_empty());
    }

    #[test]
    fn the_banner_needs_words() {
        assert!(banner::<()>(Icon::Shield, vec![], Some((false, ()))).is_none());
        assert!(banner::<()>(Icon::Shield, vec![String::new()], None).is_none());
        assert!(banner(Icon::Shield, vec!["The data folder is read-only".into()], Some((true, ()))).is_some());
    }
}
