# Navigation Targets

In the previous chapter, we learned how to create links to pages within our app.
We told them where to go using the `target` property. This property takes something that can be converted to a [`NavigationTarget`].

## What is a navigation target?

A [`NavigationTarget`] is similar to the `href` of an HTML anchor element. It
tells the router where to navigate to. The Dioxus Router knows two kinds of
navigation targets:

- [`Internal`]: We used internal links in the previous chapter. It's a link to a page within our
  app represented as a Route enum.
- [`External`]: This works exactly like an HTML anchors' `href`. Don't use this for in-app
  navigation as it will trigger a page reload by the browser.

## External navigation

If we need a link to an external page we can do it like this:

```rust, no_run
{{#include ../../examples/external_link.rs:component}}
```

[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`NavigationTarget`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html
