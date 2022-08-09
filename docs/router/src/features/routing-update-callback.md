# Routing Update Callback

In some cases we might want to run custom code when the current route changes.
For this purpose, the [`Router`] exposes the `update_callback` prop.

## How does the callback behave?
The `update_callback` is called whenever the current routing information
changes. It is called after the router updated its internal state, but before
depended components and hooks are updated.

If the callback returns a [`NavigationTarget`], the router will replace the
current location with the specified target. It will then call the
`update_callback` again. This repeats until the callback returns `None`.

If at any point the router encounters a
[navigation failure](./failures/index.md), it will go to the appropriate state
without calling the `update_callback`. It doesn't matter if the invalid target
initiated the navigation, was found as a redirect target or returned by the
`update_callback` itself.

## Code Example
```rust
# // Hidden lines (like this one) make the documentation tests work.
use std::sync::{Arc, RwLockReadGuard};

# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().fixed("home", Content as Component)
    });

    // defining the actual callback
    let update_fn = cx.use_hook(|| {
        Arc::new(|state: RwLockReadGuard<RouterState>| -> Option<NavigationTarget> {
            if state.path == "/" {
                return Some("/home".into());
            }

            None
        })
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            update_callback: update_fn.clone(),
            # init_only: true,

            Outlet { }
        }
    })
}

fn Content(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Some content" }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!("<p>Some content</p>", dioxus_ssr::render_vdom(&mut vdom));
```

[`NavigationTarget`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
