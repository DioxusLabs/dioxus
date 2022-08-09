# Sitemap Generation

If you need a list of all routes you have defined (e.g. for statically
generating all pages), Dioxus Router provides functions to extract that
information from a [`Segment`].

## Preparing an app
We will start by preparing an app with some routes like we normally would.
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home" }
    })
}

fn Fixed(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Fixed" }
        Outlet { }
    })
}

fn Nested(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Nested" }
    })
}

fn Parameter(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();
    let param = route.parameters.get("parameter").cloned().unwrap_or_default();

    cx.render(rsx! {
        h1 { "Parameter: {param}" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(Home as Component)
            .fixed(
                "fixed",
                Route::new(Fixed as Component).nested(
                    Segment::new()
                        .fixed("nested", Nested as Component)
                )
            )
            .parameter(("parameter", Parameter as Component))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # initial_path: "/fixed/nested"

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!("<h1>Fixed</h1><h2>Nested</h2>", dioxus_ssr::render_vdom(&mut vdom));
```

## Modifying the app to make using sitemaps easier
Preparing our app for sitemap generation is quite easy. We just need to extract
our segment definition into its own function.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn Fixed(cx: Scope) -> Element { unimplemented!() }
# fn Nested(cx: Scope) -> Element { unimplemented!() }
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, prepare_routes);

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # initial_path: "/fixed/nested"

            Outlet { }
        }
    })
}

fn prepare_routes() -> Segment {
    Segment::new()
        .index(Home as Component)
        .fixed(
            "fixed",
            Route::new(Fixed as Component).nested(
                Segment::new()
                    .fixed("nested", Nested as Component)
            )
        )
        .parameter(("parameter", Parameter as Component))
}
```

## Sitemaps with parameter names
The first variant to generate sitemaps is very simple. It finds all routes
within the [`Segment`] and adds them to the returned `Vec`.

Matching and parameter routes are represented by their `key`, prefixed with `\`.
Besides that `\`, all paths are URL encoded

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn Fixed(cx: Scope) -> Element { unimplemented!() }
# fn Nested(cx: Scope) -> Element { unimplemented!() }
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
# fn prepare_routes() -> Segment {
#     Segment::new()
#         .index(Home as Component)
#         .fixed(
#             "fixed",
#             Route::new(Fixed as Component).nested(
#                 Segment::new()
#                     .fixed("nested", Nested as Component)
#             )
#         )
#         .parameter(("parameter", Parameter as Component))
# }

let expected = vec![
    "/",
    "/fixed/",
    "/fixed/nested/",
    "/\\parameter/",
];
assert_eq!(expected, prepare_routes().sitemap());
```

## Sitemaps with actual parameter values
The second variant to generate sitemaps is a bit more involved. When it
encounters a parameter route, it inserts all values with a matching `key` that
were provided to it.

Matching routes only add their path if the value matches their regex.

All paths are URL encoded.

```rust
# // Hidden lines (like this one) make the documentation tests work.
use std::collections::{BTreeMap, HashSet};
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn Fixed(cx: Scope) -> Element { unimplemented!() }
# fn Nested(cx: Scope) -> Element { unimplemented!() }
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
# fn prepare_routes() -> Segment {
#     Segment::new()
#         .index(Home as Component)
#         .fixed(
#             "fixed",
#             Route::new(Fixed as Component).nested(
#                 Segment::new()
#                     .fixed("nested", Nested as Component)
#             )
#         )
#         .parameter(("parameter", Parameter as Component))
# }

let parameters = {
    let mut parameters = BTreeMap::new();

    parameters.insert("parameter", {
        let mut parameters = HashSet::new();
        parameters.insert(String::from("some-parameter-value"));
        parameters.insert(String::from("other-parameter-value"));
        parameters
    });

    parameters
};

let expected: HashSet<String> = vec![
    "/",
    "/fixed/",
    "/fixed/nested/",
    "/some-parameter-value/",
    "/other-parameter-value/",
].into_iter().map(String::from).collect(); // convert to hashmap
assert_eq!(expected, prepare_routes().sitemap_with_parameters(&parameters));
```
