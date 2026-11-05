# kit-iced

Desktop building blocks for any app as Iced 0.14 components, on [kit-rules](../kit-rules) and styled from the
generated Iced tokens (`design/out/iced_tokens.rs`). The app's name, words, routes and logo are parameters;
[`demo`](demo) shows every component.

```toml
kit-iced = { git = "https://github.com/Krifmaz/kit", tag = "v0.1.0" }
```

| Module | What it gives an app |
| --- | --- |
| `theme` | every palette (themes × accents × background), `theme::default_look()`, `set`/`look`, every widget style (`btn(Btn::Primary)`, `input`, `card`, `modal`, `toast`, `warn_box`, `badge` …) |
| `widgets` | text styles (`h1`–`h3`, `muted`, `small`), buttons, cards, `field`, `modal`, badges, skeletons, tabs/segments, the icon set, the app's `Logo`, text area, links |
| `toast` | `Toasts<A>`: errors stay with Copy, a repeated error does not pile up, Undo toasts, 3 errors / 5 toasts (kit-rules' desktop policy); `view` |
| `details` | the details dialog: any text in full, selectable, with Copy, an optional action and links |
| `whats_new` | `Popup` (after an update) and `Changelog` (the full page), with the release notes renderer |
| `update` | the newest full release (hotfixes passed over), the file for this computer, SHA256SUMS checks, the phase words and the Updates block |
| `notice` | the notice banner with Check again (kit-rules' notices) |
| `drop_zone` | drag and drop for every upload, checked like the file dialog |
| `settings` | the pieces of a settings screen: card, help, switch, select, input, test result, links, two columns |
| `form` | the settings form from a const table (`card!`, `field`) or the server's schema (`Schema::from_json`) |
| `bug_report` | report labels and badges, the file picker with its limits, the New report dialog, the reply box |
| `diagnostics` | the checks page: status dots, details, Copy, the copyable report; `view_with` for an app's own groups |
| `offline` | the offline banner |
| `signup` | `SignUp` (the sign-up page: invite link or open sign-up, as the server allows) and `Admin` (who can sign up, waiting sign-ups, invites with the link dialog); no network code: `update` answers `Out::Send(requests)` for the app to run and feed back |
| `confirm` | the confirm dialog: a question, Cancel and the action's button (red when it destroys something) |

Components hold no network code: each keeps its state, draws it, and its `update` returns what the app should
do next (save this body, run this test, send this report, copy this text). The app makes the requests with its
own client.

## Using it

```rust
use kit_iced::{theme, toast::{Toasts, ToastKind}};

theme::set(theme::default_look());         // or theme::palette("dark", "violet", "", "fade", true)
let mut toasts: Toasts<MyMsg> = Toasts::default();
toasts.push("Saved", ToastKind::Ok);
// in view(): kit_iced::toast::view(toasts.list(), 18.0, Msg::Dismiss, Msg::Copy, Msg::Action)
```

## Checks

```sh
cargo test -p kit-iced          # includes headless iced_test widget tests
cargo test -p kit-iced-demo     # every component in the demo app
cargo run -p kit-iced-demo      # the demo window
cargo clippy -p kit-iced --all-targets -- -D warnings
```
