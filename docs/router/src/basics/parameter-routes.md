# Parameter Routes
When creating a website/app we often have dynamic parameters in the path part of
the URL.

> An example of this is GitHub:
>
> - [github.com/][gh] -> The GitHub homepage or, if you are logged in, a
>   personal page.
> - [github.com/DioxusLabs][ghdl] -> The Dioxus page on GitHub.

The Dioxus Router allows to do something similar with parameter routes. In this
chapter we will basically recreate the GitHub example from above.

## Defining the route for the homepage
First, we define a route for the homepage:
```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
    });

    // ...
    # unimplemented!()
}
#
# fn Home(cx: Scope) -> Element { unimplemented!() }
```

## Loading content
Before we can handle dynamic parameters, we create a function to map from a
parameter value to some content. In a real application we would load this data
from a backend server, file or other data source. In this example we will simply
use a `match`.

```rust
fn load_content(id: &str) -> Option<(&'static str, &'static str)> {
    match id.to_lowercase().as_str() {
        "dioxus" => Some(("DioxusLabs", "This is the page for dioxus.")),
        "tefiledo" => Some(("TeFiLeDo", "This is the page of TeFiLeDo.")),
        _ => None,
    }
}
```

## Preparing a component for the parameter route
Now we will create the component that the router will render. It can access the
value of the parameter using the [`use_route`] hook.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn ParameterPage(cx: Scope) -> Element {
    let route = use_route(&cx)?;
    let id = route.parameters.get("id")?;

    let content = load_content(id).unwrap_or(("No content!", "ID unknown"));
    let (title, text) = content;

    cx.render(rsx! {
        h1 { "{title}" }
        p { "{text}" }
    })
}
#
# fn load_content(id: &str) -> Option<(&'static str, &'static str)> { unimplemented!() }
```

## Defining a parameter route
Now we can tell the router about or new route:

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
            .dynamic(DynamicRoute::parameter("id", RcComponent(ParameterPage)))
    });

    // ...
    # unimplemented!()
}
#
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn ParameterPage(cx: Scope) -> Element { unimplemented!() }
```

## Interaction with fixed routes
We can have a `dynamic` and one or more `fixed` routes within the same
[`Segment`]. In that case, the `dynamic` route will only be active, if no
`fixed` one is.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
            .fixed("fixed", Route::new(RcComponent(FixedPage)))
            .dynamic(DynamicRoute::parameter("id", RcComponent(ParameterPage)))
    });

    // ...
    # unimplemented!()
}
#
# fn FixedPage(cx: Scope) -> Element { unimplemented!() }
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn ParameterPage(cx: Scope) -> Element { unimplemented!() }
```

[gh]: https://github.com
[ghdl]: https://github.com/DioxusLabs
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
