# Adding the Router to Your Application

In this chapter we will learn how to add the router to our app. By it self, this
is not very useful. However, it is a prerequisite for all the functionality
described in the other chapters.

> Make sure you added the `dioxus-router` dependency as explained in the
> [introduction](../index.md).

In most cases we want to add the router to the root component of our app. This
way, we can ensure that we have access to all its functionality everywhere. We
add it by using the [`use_router`] hook

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

// This is the component we pass to dioxus when launching our app.
fn App(cx: Scope) -> Element {
    // Here we add the router. All components inside `App` have access to its
    // functionality.
    let routes = use_router(
        cx,
        // The router can be configured with this parameter.
        &|| RouterConfiguration {
            # synchronous: true,
            ..Default::default()
        },
        // This tells the router about all the routes in our application. As we
        // don't have any, we pass an empty segment
        &|| Segment::empty()
    );

    render! {
        h1 { "Our sites title" }

        // The Outlet tells the Router where to render active content.
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# let _ = vdom.rebuild();
# assert_eq!(
#     dioxus_ssr::render(&vdom),
#     "<h1>Our sites title</h1>"
# );
```

[`use_router`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_router.html
