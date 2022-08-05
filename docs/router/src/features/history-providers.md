# History Providers

In order to provide the ability to traverse the navigation history, the router
uses [`HistoryProvider`]s. Those implement the actual back-and-forth
functionality.

The router provides five [`HistoryProvider`]s, but you can also create your own.
The five default implementations are:
- The [`MemoryHistory`] is a custom implementation that works in memory.
- The [`WebHistory`] integrates with the browsers URL.
- The [`WebHashHistory`] also integrates with the browser, but uses the fragment
  part of the URL.
- [`ControlledHistory`] wraps around another [`HistoryProvider`] and can be
  controlled from outside the [`Router`] component by a [`HistoryController`].
- [`HistoryController`] also implements [`HistoryProvider`].

By default the router uses the following implementations:
- When the `web` feature is enabled and the compile target is WebAssembly, the
  default [`HistoryProvider`] is [`WebHistory`].
- Otherwise [`MemoryHistory`].

You can override the default history:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::{prelude::*, history::WebHashHistory};

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            routes: use_segment(&cx, Default::default).clone(),
            history: &|| WebHashHistory::new(),

            Outlet { }
        }
    })
}
```

[`ControlledHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.ControlledHistory.html
[`HistoryController`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.HistoryController.html
[`HistoryProvider`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/trait.HistoryProvider.html
[`MemoryHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.MemoryHistory.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`WebHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHistory.html
[`WebHashHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHashHistory.html
