# design/lucide — the kit's icons

The kit draws [Lucide](https://lucide.dev) icons (ISC licence, [`LICENSE`](LICENSE)) on every client: the names in
[`icons.txt`](icons.txt), turned by [`gen.mjs`](gen.mjs) into

- `crates/kit-leptos/src/lucide.rs`: each icon's SVG elements (kit-leptos `icons::icon(name)`), and
- `android/kit-compose/src/main/java/dev/krifmaz/kit/compose/icons/KitIcons.kt`: `ImageVector`s (`KitIcons`).

To add an icon: add its Lucide name to `icons.txt`, then

```sh
npm i --no-save lucide-static@0.460.0
node design/lucide/gen.mjs node_modules/lucide-static/icons
cargo fmt --all
```
