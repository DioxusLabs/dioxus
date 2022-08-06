# Named Navigation Failure

When using [named navigation](../navigation/name.md), the router runs into a
problem under these circumstances:
1. The name we try to navigate to is not contained within our routes.
2. The route we navigate to requires a parameter that we don't provide when
   triggering the navigation.

> Users cannot directly interact with named navigation. If a named navigation
> failure occurs, your app (or the router) has a bug.

The router reacts to this problem differently, depending on our apps build kind.


## Debug
When running a debug build, the router will `panic` whenever it encounters an
invalid navigation. This ensures that we notice these problems when we are
testing our application.

## Release
When running a release build, the router can't just `panic`, as that would be a
horrible user experience. Instead, it changes the path to `/` and shows some
fallback content.

The default fallback explains to the user that an error occurred and asks them
to report the bug to the app developer.

We can replace the default error message by setting the
`fallback_named_navigation` prop on our [`Router`] component.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#[allow(non_snake_case)]
fn NamedNavigationFallback(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Named navigation failure!" }
    })
}

#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, Default::default);

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            fallback_named_navigation: NamedNavigationFallback,

            Outlet { }
        }
    })
}
```

[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
