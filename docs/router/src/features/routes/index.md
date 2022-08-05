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
The easiest thing to do is to define an index route. Our apps index route will
be active when we don't specify a route, much like `index.html` works in most
web servers.

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
        Segment::new().index(RcComponent(Index))
    });

    // ...
    # unimplemented!()
}
```

## Fixed routes
It is almost as easy to define a fixed route. A fixed route is active, whenever
we specify it (in this case when the path is `/other`). This is similar to how
regular files work in most web servers.

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
            .index(RcComponent(Index))
            // not the absence of a / prefix
            .fixed("other", Route::new(RcComponent(Other)))
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
# use dioxus_router::history::MemoryHistory;
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
            .index(RcComponent(Index))
            .fixed("other", Route::new(RcComponent(Other)))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # history: &|| MemoryHistory::with_first(String::from("/other")),
            # init_only: true,

            // This tells the router where to put the content of the active
            // route.
            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<p>some other content</p>", html);
```

[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
