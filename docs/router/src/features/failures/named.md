# Named Navigation Failure

When using [named navigation](../navigation/name.md), the router runs into a
problem under these circumstances:
1. The name we try to navigate to is not contained within our routes.
2. The route we navigate to requires a parameter that we don't provide when
   triggering the navigation.

The router reacts to these problems differently, depending on our apps build
kind.

## Debug
When running a debug build, the router will `panic` whenever it encounters an
invalid navigation. This ensures that we notice these problems when we are
testing our application.

## Release
When running a release build, the router can't just `panic`, as that would be a
horrible user experience. Instead, it navigates to
[`PATH_FOR_NAMED_NAVIGATION_FAILURE`] (a constant the router exports). We can
define a [_fixed_ route](../routes/index.md#fixed-routes) on our root
[`Segment`] to handle those situations.

[`PATH_FOR_NAMED_NAVIGATION_FAILURE`]: https://docs.rs/dioxus-router/latest/dioxus_router/constant.PATH_FOR_NAMED_NAVIGATION_FAILURE.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
