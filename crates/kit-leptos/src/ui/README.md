# kit-leptos `ui`: the design system

The layout and the rich components an app builds its pages from, drawn by the `k-` classes of
[`style/kit.css`](../../style/kit.css) on the tokens of `design/out/tokens.css`. Every theme draws them. The
words are parameters (English defaults: "More", "Back", "See all", "Close"). The demo (`crates/kit-leptos/demo`)
shows every one, and CI screenshots its pages at a phone, a tablet and a desktop size (`scripts/visual-qa.cjs`).

| Component | What it draws | Classes |
| --- | --- | --- |
| `AppShell` | phones (below 768 px): glass app bar (back button or logo, title, `actions`) and glass tab bar (`tabs`, up to 5; more sections go under More, a sheet); 768–1023 px: a rail; from 1024 px: the sidebar with `side_foot` | `.k-shell`, `.k-appbar`, `.k-tabbar`, `.k-tab`, `.k-side`, `.k-side-link`, `.k-main` |
| `Page` | one page; fades and rises in (180 ms) each time it is drawn | `.k-page` |
| `PageHead`, `Section` | a page's title, line and actions; a part's heading with "See all" | `.k-pagehead`, `.k-section`, `.k-section-head`, `.k-link` |
| `Sheet` | a full-width bottom sheet on phones, a centred panel from 768 px; Escape, ✕ or a tap outside closes | `.k-sheet-bg`, `.k-sheet` |
| `MediaCard` | a poster (2:3), square or wide cover with title, line, badge and progress; lifts under a mouse, presses in under a finger | `.k-card`, `.k-poster`/`.k-square`/`.k-wide` |
| `MediaGrid` | as many columns as fit (three posters on a phone) | `.k-grid` |
| `Carousel` | a titled row that scrolls sideways and snaps; arrow buttons on screens with a mouse | `.k-rail`, `.k-rail-track` |
| `Hero` | a banner: the art blurred behind, the cover, eyebrow, title, a line, actions | `.k-hero` |
| `DetailHeader` | an item's page top: backdrop (or the blurred cover), cover, title, metadata chips, actions | `.k-detail` |
| `Chip`, `FilterChips` | a metadata pill; a scrolling row of filter chips with one on | `.k-chip`, `.k-chips` |
| `StatCard` | a value, its label, icon and change (`+12%` green, `−3%` red) | `.k-stats`, `.k-stat` |
| `ProgressRing` | a ring with its percentage; tones `accent`, `ok`, `warn`, `err`, `info` | `.k-ring` |
| `EmptyState` | an illustration (`Empty`, `Search`, `Offline`, `Error`, `Done`), title, line, actions | `.k-empty` |
| `SkeletonLines`, `SkeletonGrid`, `SkeletonRail` | content on its way, in its shape; shimmers unless motion is reduced | `.k-skel` |
| `placeholder_art` | a cover for an item without one, from a seed and its title (`data:` SVG) | |

Helpers: `.k-icon-btn` (a 44 px icon button), `.k-actions` (a wrapping row of buttons), `.k-eyebrow`, `.k-sub`.

Rules every page keeps: nothing scrolls sideways but carousels and chip rows; every tap target is 44 px on phones
and touch screens (`.btn`, inputs and segmented controls grow to it); inputs are 16 px on phones so iOS doesn't zoom;
the bars and sheets keep clear of notches and the home indicator.
