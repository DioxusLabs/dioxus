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
When the router encounters an external navigation it cannot fullfil, it will
navigates to [`PATH_FOR_EXTERNAL_NAVIGATION_FAILURE`] (a constant the router
exports) and provides the external URL in a query parameter named `url`. You can
use this to render a [`Link`] to the target.

[`ControlledHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.ControlledHistory.html
[`HistoryController`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.HistoryController.html
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`PATH_FOR_EXTERNAL_NAVIGATION_FAILURE`]: https://docs.rs/dioxus-router/latest/dioxus_router/constant.PATH_FOR_EXTERNAL_NAVIGATION_FAILURE.html
[`WebHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHistory.html
[`WebHashHistory`]: https://docs.rs/dioxus-router/latest/dioxus_router/history/struct.WebHashHistory.html
