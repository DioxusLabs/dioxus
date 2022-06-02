# Global Fallback (404 page)

The global fallback refers to the `fallback` prop of the [`Router`] component.
It allows you to handle the case when the router cannot find an active route for
a given path. This is similar to the `404` pages of traditional websites.

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::router::history::MemoryHistory;
use dioxus::prelude::*;

fn Fallback(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Not Found!" }
    })
}

fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        Router {
            routes: use_segment(&cx, Default::default).clone(),
            fallback: RcComponent(Fallback),
            # init_only: true,
            # history: &|| MemoryHistory::with_first(String::from("/invalid")),

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus::ssr::render_vdom(&vdom);
# assert_eq!("<h1>Not Found!</h1>", html);
```

## Fallback routes
The global fallback is inhibited by [fallback routes](./routes/fallback.md). If
a fallback route is active, the global fallback will not be.

[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
