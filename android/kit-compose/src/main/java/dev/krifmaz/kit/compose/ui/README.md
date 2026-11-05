# kit-compose `ui`: the design system

The web's design system (kit-leptos `ui`) for Compose, on the same tokens (`KitSpace`, `KitSizes`, `KitMotion` in
the generated `KitTokens.kt`) and the same rules (`DesignRules`, tested with the web's cases).

| Composable | What it draws |
| --- | --- |
| `KitScaffold(brand, nav, current, onNavigate, title, onBack, actions, sideFoot, tabs) { padding -> }` | below 768 dp: app bar and a bottom bar of up to five sections, the rest under More (a sheet); 768–1023 dp: a navigation rail; from 1024 dp: the sidebar. Clear of the status bar, cutout and gesture bar |
| `KitAppBar(title, onBack, actions)` | the app bar on its own |
| `KitSheet(open, onClose, title) { }` | a bottom sheet with a title and a close button |
| `MediaCard(title, onClick, art, seed, subtitle, badge, progress, shape)` | a poster, square or wide cover (`KitShape`), presses in; `PlaceholderArt(seed, title)` when there's no art |
| `MediaGrid(items) { }`, `columnWidth(shape, width)` | as many columns as fit (three posters on a phone) |
| `Carousel(title, items, key, shape, onSeeAll) { }` | a titled `LazyRow` that snaps each card |
| `Hero(title, eyebrow, text, art, seed) { actions }` | a banner with the art blurred behind (Android 12+) |
| `DetailHeader(title, subtitle, art, seed, shape, chips) { actions }` | the top of an item's screen |
| `KitChip(label, icon)`, `FilterChips(options, selected, onSelect)` | metadata pills; a scrolling row of 44 dp filter chips |
| `StatCard(label, value, icon, delta, hint)`, `ProgressRing(value, size, stroke, label, color)` | numbers at a glance |
| `KitEmptyState(title, text, art) { actions }` | an illustration (`KitIllustration`), what's missing, what to do |
| `SkeletonLines`, `SkeletonCard`, `SkeletonGrid`, `SkeletonRail`, `Modifier.shimmer()` | content on its way; still when "Remove animations" is on (`rememberReducedMotion()`) |

The demo app's "Open the design system" button shows every one (`DemoDesign.kt`).
