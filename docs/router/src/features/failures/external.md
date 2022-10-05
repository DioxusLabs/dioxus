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

When using [`ControlledHistory`] the router will not be able to detect whether
it can navigate to external targets or not. After each routing operation, you
will need to check the [`HistoryController`] to see if an external redirect has
happened.

## Failure handling
When the router encounters an external navigation it cannot fulfill, it changes
the path to `/` and shows some fallback content.

> You can detect if the router is in the external navigation failure handling
> state by [checking](../navigation/name.md#check-if-a-name-is-present) if the
> [`FallbackExternalNavigation`] name is present.

The default fallback explains to the user that the navigation was unsuccessful
and provides them with a [`Link`] to fulfill it manually. It also allows them to
go back to the previous page.

The default error message can be replaced by setting the
`fallback_external_navigation` prop on the [`Router`] component. The external
URL will be provided via the `url` parameter.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
fn ExternalNavigationFallback(cx: Scope) -> Element {
    let route = use_route(&cx).expect("is nested within a Router component");
    let url = route.parameters.get("url").cloned().unwrap_or_default();

    cx.render(rsx! {
        h1 { "External navigation failure!" }
        Link {
            target: ExternalTarget(url),
            "Go to external site"
        }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, Default::default);

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            fallback_external_navigation: ExternalNavigationFallback,

            Outlet { }
        }
    })
}
```

[`ControlledHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.ControlledHistory.html
[`FallbackExternalNavigation`]: https://docs.rs/dioxus-router/latest/dioxus_router/names/struct.FallbackExternalNavigation.html
[`HistoryController`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.HistoryController.html
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`WebHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHistory.html
[`WebHashHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHashHistory.html
