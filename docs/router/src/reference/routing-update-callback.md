# Routing Update Callback

In some cases we might want to run custom code when the current route changes.
For this reason, the [`RouterConfiguration`] exposes an `on_update` field.

## How does the callback behave?

The `on_update` is called whenever the current routing information changes. It
is called after the router updated its internal state, but before depended
components and hooks are updated.

If the callback returns a [`NavigationTarget`], the router will replace the
current location with the specified target. It will not call the
`on_update` again.

If at any point the router encounters a
[navigation failure](./failures/index.md), it will go to the appropriate state
without calling the `on_update`. It doesn't matter if the invalid target
initiated the navigation, was found as a redirect target or returned by the
`on_update` itself.

## Code Example

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# extern crate dioxus_router;
# extern crate dioxus_ssr;
#
use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_router::prelude::*;

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            on_update: Some(Arc::new(|state| -> Option<NavigationTarget> {
                if state.path == "/" {
                    return Some("/home".into());
                }

                None
            })),
            ..Default::default()
        },
        &|| Segment::empty().fixed("home", comp(Content))
    );

    render! {
        Outlet { }
    }
}

fn Content(cx: Scope) -> Element {
    render! {
        p { "Some content" }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(dioxus_ssr::render(&mut vdom), "<p>Some content</p>");
```

[`NavigationTarget`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html
[`RouterConfiguration`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/struct.RouterConfiguration.html
