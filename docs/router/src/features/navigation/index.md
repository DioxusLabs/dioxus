# Links & Navigation

When we split our app into pages, we need to provide our users with a way to
navigate between them. On regular web pages we'd use an anchor element for that,
like this:

```html
<a href="/other">Link to an other page</a>
```

However, we cannot do that when using the router for two reasons:
1. Anchor tags make the browser load a new page from the server. This takes a
   lot of time, and it is much faster to let the router handle the navigation
   client-side.
2. Navigation using anchor tags only works when the app is running inside a
   browser. This means we cannot use them inside apps using Dioxus Desktop.

To solve these problems, the router provides us with a [`Link`] component we can
use like this:
```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
fn SomeComponent(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NavigationTarget::Internal(String::from("/some/path")),
            "Link text"
        }
        Link {
            target: "/some/path", // short form
            "Other link text"
        }
    })
}
```

The `target` in the example above is similar to the `href` of a regular anchor
element. However, it tells the router more about what kind of navigation it
should perform:
- The example uses [`InternalTarget`]. We give it an arbitrary path that will be
  merged with the current URL.
- [`NamedTarget`] allows us to navigate within our app using predefined names.
  See the chapter about [named navigation](./name.md) for more details.
- [`ExternalTarget`] allows us to navigate to URLs outside of our app. See the
  chapter about [external navigation](./external.md) for more details.

> The [`Link`] accepts several props that modify its behavior. See the API docs
> for more details.

[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`Named`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Named
