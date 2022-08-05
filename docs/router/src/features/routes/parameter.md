# Parameter Routes

Many modern web apps store parameters within their current path. This allows
users to share URLs that link to a specific bit of content. We can create this
functionality with parameter routes.

> If you want to change what route is active based on the format of the
> parameter, see [Matching Routes](./matching.md).

> The parameter will be URL decoded.

## Creating a content component
We start by creating a component that uses the parameters value.

We can get the current state of the router using the [`use_route`] hook. From
that state we can extract the current value of our parameter by using a key we
will later also define on our route.

> It is **VERY IMPORTANT** to dropt the object returned by the [`use_route`]
> hook once our component finished rendering. Otherwise the entire router will
> be frozen.

> The [`use_route`] hook can only be used in components nested within a
> [`Router`] component.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn Greeting(cx: Scope) -> Element {
    let route = use_route(&cx).expect("is nested within a Router component");
    let name = route.parameters.get("name")
        .map(|name| name.clone())
        .unwrap_or(String::from("world"));

    cx.render(rsx! {
        p { "Hello, {name}!" }
    })
}
```

## Defining the routes
Now we can define our route. Unlike a fixed [`Route`], a [`ParameterRoute`]
needs to arguments to be created.

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
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .parameter(ParameterRoute::new("name", Greeting as Component))
            .parameter(("name", Greeting as Component)) // same in short
    });

    // ...
    # unimplemented!()
}
```

## Interaction with other routes
Each individual [`Segment`] can only ever have one active route. This means that
when a [`Segment`] has more than just a parameter route, the router has to
decide which is active. It does that this way:

0. If the segment is not specified (i.e. `/`), then the index route will be
   active.
1. If a [_fixed_](./index.md#fixed-routes) route matches the current path, it
   will be active.
2. If a [_matching_ route](./matching.md) matches the current path, it will be
   active. _Matching_ routes are checked in the order they are defined.
3. If neither a _fixed_ nor a _matching_ route is active, the _parameter_ route
   or [_fallback_ route](./fallback.md) will be.

Step 0 means that if we want a parameter to be empty, that needs to be specified
by the path, i.e. `//`.

> Be careful with using parameter routes on the root [`Segment`]. Navigating to
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
# use dioxus_router::{prelude::*, history::MemoryHistory};
# extern crate dioxus_ssr;

fn Greeting(cx: Scope) -> Element {
    let route = use_route(&cx).expect("is nested within a Router component");
    let name = route.parameters.get("name")
        .map(|name| name.clone())
        .unwrap_or(String::from("world"));

    cx.render(rsx! {
        p { "Hello, {name}!" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().parameter(("name", Greeting as Component))
    });

    // ...
    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,
            # history: &|| MemoryHistory::with_first(String::from("/Dioxus")),

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     "<p>Hello, Dioxus!</p>",
#     dioxus_ssr::render_vdom(&vdom)
# );
```

[`ParameterRoute`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.ParameterRoute.html
[`Route`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Route.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
