# Adding the Router to Your Application

In this chapter we will learn how to add the [`Router`] component to our app. By
itself, it is not very useful. However, it is a prerequisite for all the
functionality described in the other chapters.

> Make sure you added the `router` feature to Dioxus as explained in the
> [introduction](../index.md).

In most cases we want to add the [`Router`] to the root component of our app.
This way, we can ensure that we have access to all its functionality everywhere.

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

// This is the component we pass to dioxus when launching our app.
fn App(cx: Scope) -> Element {
    // The Router component requires a Segment. In later chapters, we will use
    // it to define the routes of our app.
    let routes = use_segment(&cx, || Segment::new());

    cx.render(rsx! {
        // We put everything within our Router. All things outside don't have
        // access to its functionality.
        Router {
            // We pass our routes to the router
            routes: routes.clone(),
            # init_only: true,

            h1 { "Our sites title" }
            // The Outlet tells the Router where to render active content.
            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus::ssr::render_vdom(&vdom);
# assert_eq!("<h1>Our sites title</h1><!--placeholder-->", html);
```

[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
