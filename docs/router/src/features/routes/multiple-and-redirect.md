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
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home Page" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            // notice that we use RcRedirect instead of RcComponent
            .index(RcRedirect(InternalTarget(String::from("/home"))))
            .fixed("start", "/home") // short form
            .fixed("home", Home as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<h1>Home Page</h1>", html);
```
