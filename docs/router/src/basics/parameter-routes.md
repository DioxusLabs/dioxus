# Parameter Routes
When creating a website/app we often have dynamic parameters in the path part of
the URL.

An example of this is GitHub:
```txt
https://github.com/           -> the GitHub homepage
https://github.com/DioxusLabs -> the Dioxus page on GitHub
```

The Dioxus Router allows to do something similar with parameter routes. In this
chapter we will replicate the GitHub example.

## Loading content
Before we can accept a parameter via the path, we need a way to map its value to
content. In a real application, we would load data from a backend, file or other
data source. To keep this example simple, we will use static content and a
`match`.

```rust,no_run
fn load_content(id: &str) -> Option<(&'static str, &'static str)> {
    match id.to_lowercase().as_str() {
        "dioxus" => Some(("DioxusLabs", "This is the page for Dioxus.")),
        "tefiledo" => Some(("TeFiLeDo", "This is the page of TeFiLeDo.")),
        _ => None,
    }
}
```

## Preparing a component for the parameter route
We now create the component the router will render when the parameter route is
active.

We can use the [`use_route`] hook to gain access to the current routing
information. This allows us to get the current value from the `parameters` map.

Note that we ask for the value of a parameter with a key of `id`. When defining
our routes, we need to use the same exact key.

> Parameter keys have this type: `&'static str`. This means we can define them
> as constants, which allows the compiler to check for their existence and
> correctness.
>
> ```rust,no_run
> const CONTENT_ID: &'static str = "id";
> ```

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn ParameterPage(cx: Scope) -> Element {
    // get the parameters value
    let route = use_route(&cx)?;
    let id = route.parameters.get("id")?;

    // use the value to retrieve content
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

## Defining the routes
Now we can define the routes.

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
We can have a `dynamic` and several `fixed` routes within the same [`Segment`].
In that case, `fixed` routes have precedence over the `dynamic` route.

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
