# History Providers

In order to provide the ability to traverse the navigation history, the router
uses [`HistoryProvider`]s. Those implement the actual back-and-forth
functionality.

The router provides five [`HistoryProvider`]s, but you can also create your own.
The five default implementations are:

- The [`MemoryHistory`] is a custom implementation that works in memory.
- The [`WebHistory`] integrates with the browsers URL.

By default the router uses the [`MemoryHistory`]. It might be changed to use
[`WebHistory`] when the `web` feature is active, but that is not guaranteed.

You can override the default history:

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::{prelude::*, history::WebHashHistory};

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            history: Box::new(WebHashHistory::new(true)),
            ..Default::default()
        },
        &|| Segment::empty()
    );

    render! {
        Outlet { }
    }
}
```

[`HistoryProvider`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/history/trait.HistoryProvider.html
[`MemoryHistory`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/history/struct.MemoryHistory.html
[`WebHistory`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/history/struct.WebHistory.html
