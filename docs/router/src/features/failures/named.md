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
horrible user experience. Instead, it changes shows some fallback content.

> You can detect if the router is in the named navigation failure handling state
> by [checking](../navigation/name.md#check-if-a-name-is-present) if the
> [`FailureNamedNavigation`] name is present.

The default fallback explains to the user that an error occurred and asks them
to report the bug to the app developer.

You can override it by setting the `failure_named_navigation` value of the
[`RouterConfiguration`].

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
fn NamedNavigationFallback(cx: Scope) -> Element {
    render! {
        h1 { "Named navigation failure!" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            failure_named_navigation: comp(NamedNavigationFallback),
            ..Default::default()
        },
        &|| Segment::empty()
    );

    render! {
        Outlet { }
    }
}
```

[`FailureNamedNavigation`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/prelude/struct.FailureNamedNavigation.html
[`RouterConfiguration`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/struct.RouterConfiguration.html
