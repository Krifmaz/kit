# kit-compose `icons`

`KitIcons`: the kit's icon set, the same Lucide icons (ISC licence) as the web's (`design/lucide/icons.txt`), as
stroked 24 × 24 `ImageVector`s built once and cached. `Icon(KitIcons.Search, contentDescription = "Search")`, or
by the web's name: `KitIcons.byName("layout-grid")` (null when the kit doesn't ship it).

`KitIcons.kt` is generated: add a name to `design/lucide/icons.txt` and run `node design/lucide/gen.mjs` (see
[`design/lucide/README.md`](../../../../../../../../../../design/lucide/README.md)).
