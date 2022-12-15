# Catch All Routes

Many modern web apps store parameters within their current path. This allows
users to share URLs that link to a specific bit of content. We can create this
functionality with catch all routes.

> If you want to change what route is active based on the format of the
> parameter, see [Matching Routes](./matching.md).

> The parameter will be URL decoded.

## Creating a content component
We start by creating a component that uses the parameters value.

We can get the current state of the router using the [`use_route`] hook. From
that state we can extract the current value of our parameter by using a key we
will later also define on our route.

> It is **VERY IMPORTANT** to drop the object returned by the [`use_route`]
> hook once our component finished rendering. Otherwise the entire router will
> be frozen.

> The [`use_route`] hook can only be used in components nested within a
> component that called [`use_router`].

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

struct Name;

fn Greeting(cx: Scope) -> Element {
    let route = use_route(&cx).expect("is nested within a Router component");
    let name = route.parameter::<Name>()
        .map(|name| name.clone())
        .unwrap_or(String::from("world"));

    cx.render(rsx! {
        p { "Hello, {name}!" }
    })
}
```

## Defining the routes
Now we can define our route. Unlike a fixed [`Route`], a [`ParameterRoute`]
needs two arguments to be created.

> Also note that each [`Segment`] can have exactly one parameter or
> [fallback route](./fallback.md).
>
> For that reason, the example below would not work in practice, but showing
> both forms (explicit and short) is more important for this example.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Greeting(cx: Scope) -> Element { unimplemented!() }
#
struct Name;

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            ..Default::default()
        },
        &|| {
           Segment::empty()
            .catch_all(ParameterRoute::content::<Name>(comp(Greeting)))
            .catch_all((comp(Greeting), Name { })) // same in short
        }
    );

    // ...
    # unimplemented!()
}
```

## Interaction with other routes
Each individual [`Segment`] can only ever have one active route. This means that
when a [`Segment`] has more than just a catch all route, the router has to
decide which is active. It does that this way:

0. If the segment is not specified (i.e. `/`), then the index route will be
   active.
1. If a [_fixed_](./index.md#fixed-routes) route matches the current path, it
   will be active.
2. If a [_matching_ route](./matching.md) matches the current path, it will be
   active. _Matching_ routes are checked in the order they are defined.
3. If neither a _fixed_ nor a _matching_ route is active, the _catch all_ route
   or [_fallback_ route](./fallback.md) will be.

Step 0 means that if we want a parameter to be empty, that needs to be specified
by the path, i.e. `//`.

> Be careful with using catch all routes on the root [`Segment`]. Navigating to
> paths starting with `//` will **NOT** work. This is not a limitation of the
> router, but rather of how relative URLs work.
>
> If you absolutely need an empty parameter on the root [`Segment`], a URL like
> this _could_ work:
> - `https://your-site.example//` for web sites
> - `dioxus://index.html//` for desktop apps

## Full Code
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;
#
struct Name;

fn Greeting(cx: Scope) -> Element {
    let route = use_route(&cx).expect("is nested within a Router component");
    let name = route.parameter::<Name>()
        .map(|name| name.clone())
        .unwrap_or(String::from("world"));

    cx.render(rsx! {
        p { "Hello, {name}!" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_router(
        &cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/Dioxus").unwrap()),
            ..Default::default()
        },
        &|| Segment::empty().catch_all((comp(Greeting), Name { }))
    );
    // ...
    cx.render(rsx! {
        Outlet { }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     dioxus_ssr::render(&vdom),
#     "<p>Hello, Dioxus!</p>"
# );
```

[`ParameterRoute`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.ParameterRoute.html
[`Route`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.Route.html
[`Segment`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.Segment.html
[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
[`use_router`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_router.html
