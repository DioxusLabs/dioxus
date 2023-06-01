# Navigation Failures

Some specific operations can cause a failure within router operations. The
subchapters contain information on how the router lets us handle such failures.

# External Navigation Failure

> This section doesn't apply when specifying a `target` on a [`Link`]. See the
> chapter about [external navigation](../navigation/external.md) for more
> details.

When we ask the router to navigate to an external target, either through
[programmatic navigation](../navigation/programmatic.md) or a
[redirect](../routes/multiple-and-redirect.md#redirects) the router needs to
navigate to an external target without being able to rely on an anchor element.

This will only work in the browser, when using either [`WebHistory`] or
[`WebHashHistory`].

## Failure handling

When the router encounters an external navigation it cannot fulfill, it changes
the path to `/` and shows some fallback content.

> You can detect if the router is in the external navigation failure handling
> state by [checking](../navigation/name.md#check-if-a-name-is-present) if the
> [`FailureExternalNavigation`] name is present.

The default fallback explains to the user that the navigation was unsuccessful
and provides them with a [`Link`] to fulfill it manually. It also allows them to
go back to the previous page.

You can override it by setting the `failure_external_navigation` value of the
[`RouterConfiguration`]. The external URL will be provided via the
[`FailureExternalNavigation`] parameter.

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
fn ExternalNavigationFallback(cx: Scope) -> Element {
    let route = use_route(cx).expect("is nested within a Router component");
    let url = route
        .parameter::<FailureExternalNavigation>()
        .unwrap_or_default();

    render! {
        h1 { "External navigation failure!" }
        Link {
            target: url,
            "Go to external site"
        }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            failure_external_navigation: comp(ExternalNavigationFallback),
            ..Default::default()
        },
        &|| Segment::empty()
    );

    render! {
        Outlet { }
    }
}
```

[`FailureExternalNavigation`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/prelude/struct.FailureExternalNavigation.html
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`RouterConfiguration`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/struct.RouterConfiguration.html
[`WebHistory`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/history/struct.WebHistory.html
[`WebHashHistory`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/history/struct.WebHashHistory.html
