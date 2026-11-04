# kit-iced-demo

Every [kit-iced](..) component in one desktop window, for an imaginary app called Demo, with fake data.
The sidebar has one page per feature; dialogs and toasts are drawn over the window.

```sh
cargo run -p kit-iced-demo     # the window
cargo test -p kit-iced-demo    # every page drawn headless (iced_test), plus the main flows
```

| Page | What it shows |
| --- | --- |
| Appearance | the themes (`encore`, `dark`, `oled`, `light`), the background, an accent picker over `theme::ACCENTS`, text styles, every `Btn` look and the button helpers, badges, a card with a field, skeletons, `Icon::ALL` and `Logo::KIT` |
| Toasts | an ok toast, an error that stays with Copy, the same error again (it does not pile up), an Undo toast, warning and info |
| Details dialog | a long error text with a lead, an action (Keep anyway) and links (title, subtitle, button) |
| Confirm dialog | "Delete report?" with a red Delete, and a plain one; OK shows a toast |
| What's new | the changelog page from fake releases, and the popup from a fake `GET /api/changelog/new` |
| Updates | the Updates block (Check for updates finds a fake release, passing over a hotfix), Next phase to step through every phase, and the words of each |
| Notices | the notice banner; Check again shows Checking…, then the toast of the answer (still failing, still checking, fixed) |
| Offline | Go offline puts the offline banner over every page until Retry |
| Drag and drop | the drop place for images: files dropped on the window are listed or refused, each with a toast; buttons send the same events |
| Settings | the settings form from a fake schema (a General card, an Email card whose fields follow the Security select, secrets) and the settings pieces; Save and Test are answered with fake results |
| Bug report | a report with its thread and the reply box, every kind and status, and the New report dialog |
| Diagnostics | fake checks (ok, warn, fail) with "1 problem", Run checks and Copy report, and the server facts |

The components hold no network code: their update functions answer what the app should do next. The demo
answers those requests itself with fake data on its next tick (half a second later), so the busy states
show. Copies go to the real clipboard and are read back to check them.

Code: `src/lib.rs` holds the app (`Demo`, `Message`, `Demo::update`, `Demo::view`), `src/main.rs` runs it,
`tests/demo.rs` drives it headless.
