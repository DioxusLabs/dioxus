# Multiple Components & Redirects

## Multiple Components
When creating complex apps we sometimes want to have multiple pieces of content
side by side. The router allows us to do this. For more details see the section
about [named `Outlet`s](../outlets.md#named-outlets).

## Redirects
In some cases we may want to redirect our users to another page whenever they
open a specific path. We can tell the router to do this when defining our
routes.

> Redirects to external pages only work in certain conditions. For more details
> see the chapter about [external navigation failures](../failures/external.md).

In the following example we will redirect everybody from `/` and `/start` to
`/home`.

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;

fn Home(cx: Scope) -> Element {
    render! {
        h1 { "Home Page" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/home").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::content(comp(Home))
                // notice that we use RouteContent::Redirect instead of
                // RouteContent::Content (which we have been using indirectly)
                .fixed(
                    "home",
                    RouteContent::Redirect(NavigationTarget::Internal("/".into()))
                )
                .fixed("start", "/") // short form
    });

    render! {
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(html, "<h1>Home Page</h1>");
```
