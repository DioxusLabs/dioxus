# Defining Routes

When creating a [`Router`] we need to pass it a [`Segment`]. It tells the router
about all the routes of our app.

## Example content
To get a good understanding of how we define routes we first need to prepare
some example content, so we can see the routing in action.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to our test site!" }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "some other content" }
    })
}
```

## Index routes
The easiest thing to do is to define an index route.

Index routes act very similar to `index.html` files in most web servers. They
are active, when we don't specify a route.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Index(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().index(Index as Component)
    });

    // ...
    # unimplemented!()
}
```

## Fixed routes
It is almost as easy to define a fixed route.

Fixed routes work similar to how web servers treat files. They are active, when
specified in the path. In the example, the path must be `/other`.

> The path will be URL decoded before checking if it matches our route.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Index(cx: Scope) -> Element { unimplemented!() }
# fn Other(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Index as Component)
            // note the absence of a / prefix
            .fixed("other", Other as Component)
    });

    // ...
    # unimplemented!()
}
```

## Full Code
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to our test site!" }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "some other content" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Index as Component)
            .fixed("other", Other as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # initial_path: "/other"

            // This tells the router where to put the content of the active
            // route.
            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     "<p>some other content</p>",
#     dioxus_ssr::render_vdom(&vdom)
# );
```

[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
