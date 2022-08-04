# History Buttons

Some platforms, like web browsers, provide users with an easy way to navigate
through an apps history. They have UI elements or integrate with the OS.

However, native platforms usually don't provide such amenities, which means that
apps wanting users to have access to them, need to implement them. For this
reason the router comes with two components, which emulate a browsers back and
forward buttons:

- [`GoBackButton`](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.GoBackButton.html)
- [`GoForwardButton`](https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.GoForwardButton.html)

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn HistoryNavigation(cx: Scope) -> Element {
    cx.render(rsx! {
        GoBackButton {
            "Back to the Past"
        }
        GoForwardButton {
            "Back to the Future" /* You see what I did here? 😉 */
        }
    })
}
```

As you might know, browsers usually disable the back and forward buttons if
there is no history to navigate to. The routers history buttons try to do that
too, but depending on the [`HistoryProvider`] that might not be possible.

Importantly, neither [`WebHistory`] nor [`WebHashHistory`] support that feature.
This is due to limitations of the browser History API.

However, in both cases the router will just ignore button presses, if there is
no history to navigate to.

Also, when using [`WebHistory`] or [`WebHashHistory`], the history buttons might
navigate a user to a history entry outside your app.

[`HistoryProvider`]: ./history-providers.md
[`WebHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHistory.html
[`WebHashHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHashHistory.html
