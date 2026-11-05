//! The kit's design system: the layout and the rich components every app builds its pages from, styled by the kit's
//! stylesheet (the `k-` classes of `style/kit.css`) on the design tokens, so each theme draws them.
//!
//! Mobile first. Below 768 px wide an app gets a top app bar and a bottom tab bar (four or five sections, the rest
//! under More), full-width sheets instead of dialogs, 44 px touch targets and the safe-area insets of notched
//! phones; from 768 px a navigation rail, from 1024 px the full sidebar. Nothing scrolls sideways except the
//! carousels and chip rows that are meant to. Animations are under 200 ms and stop when the system asks for
//! reduced motion.
//!
//! | Module | Components |
//! | --- | --- |
//! | [`shell`] | [`AppShell`] (sidebar · rail · app bar · tab bar · More), [`Page`] (the page transition), [`PageHead`], [`Section`] |
//! | [`sheet`] | [`Sheet`]: a full-width bottom sheet on phones, a centred panel on wider screens |
//! | [`media`] | [`MediaCard`] (poster, square or wide cover with hover and press states), [`MediaGrid`], [`Carousel`], [`Hero`] (blurred backdrop art), [`DetailHeader`], [`Chip`], [`FilterChips`] |
//! | [`stats`] | [`StatCard`], [`ProgressRing`] |
//! | [`feedback`] | [`EmptyState`] with an illustration, [`SkeletonLines`], [`SkeletonGrid`], [`SkeletonRail`] |
//! | [`art`] | [`placeholder_art`]: a cover drawn from a seed and a title when an item has no art |
//!
//! Every word is a parameter; the components hold no app words. Icons are the kit's Lucide set
//! ([`crate::icons::icon`]).

pub mod art;
pub mod feedback;
pub mod media;
pub mod sheet;
pub mod shell;
pub mod stats;

pub use art::placeholder_art;
pub use feedback::{EmptyState, Illustration, SkeletonGrid, SkeletonLines, SkeletonRail};
pub use media::{Carousel, Chip, DetailHeader, FilterChips, Hero, MediaCard, MediaGrid, Shape};
pub use sheet::Sheet;
pub use shell::{AppShell, NavItem, Page, PageHead, Section};
pub use stats::{ProgressRing, StatCard};
