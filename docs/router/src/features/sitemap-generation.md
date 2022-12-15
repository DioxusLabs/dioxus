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
use dioxus_router::{history::MemoryHistory, prelude::*};
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

struct ParameterName;
fn Parameter(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();
    let param = route.parameter::<ParameterName>().unwrap_or_default();

    cx.render(rsx! {
        h1 { "Parameter: {param}" }
    })
}

fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            # synchronous: true,
            history: Box::new(MemoryHistory::with_initial_path("/fixed/nested").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::content(comp(Home))
                .fixed(
                    "fixed",
                    Route::content(comp(Fixed)).nested(
                        Segment::empty().fixed("nested", comp(Nested))
                    )
                )
                .catch_all((comp(Parameter), ParameterName { }))
        }
    );

    cx.render(rsx! {
        Outlet { }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(dioxus_ssr::render(&mut vdom), "<h1>Fixed</h1><h2>Nested</h2>");
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
# struct ParameterName;
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    use_router(
        &cx,
        &|| RouterConfiguration {
            ..Default::default()
        },
        &prepare_routes
    );

    cx.render(rsx! {
        Outlet { }
    })
}

fn prepare_routes() -> Segment<Component> {
    Segment::content(comp(Home))
        .fixed(
            "fixed",
            Route::content(comp(Fixed)).nested(
                Segment::empty().fixed("nested", comp(Nested))
            )
        )
        .catch_all((comp(Parameter), ParameterName { }))
}
```

## Sitemaps with parameter names
The first variant to generate sitemaps is very simple. It finds all routes
within the [`Segment`] and adds them to the returned `Vec`.

Matching and parameter routes are represented by their `key`, prefixed with `\`.
Besides that `\`, all paths are URL encoded.

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
# struct ParameterName;
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
# fn prepare_routes() -> Segment<Component> {
#     Segment::content(comp(Home))
#         .fixed(
#             "fixed",
#             Route::content(comp(Fixed)).nested(
#                 Segment::empty().fixed("nested", comp(Nested))
#             )
#         )
#         .catch_all((comp(Parameter), ParameterName { }))
# }

let expected = vec![
    "/",
    "/fixed",
    "/fixed/nested",
    // Usually, here would be a fourth result representing the parameter route.
    // However, due to mdbook the name for this file would constantly change,
    // which is why we cannot show it. It would look something like this:
    // "/\\your_crate::ParameterName",
];
let mut sitemap = prepare_routes().gen_sitemap();
sitemap.remove(3); // see above
assert_eq!(sitemap, expected);
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
# struct ParameterName;
# fn Parameter(cx: Scope) -> Element { unimplemented!() }
# fn prepare_routes() -> Segment<Component> {
#     Segment::content(comp(Home))
#         .fixed(
#             "fixed",
#             Route::content(comp(Fixed)).nested(
#                 Segment::empty().fixed("nested", comp(Nested))
#             )
#         )
#         .catch_all((comp(Parameter), ParameterName { }))
# }

let parameters = {
    let mut parameters = BTreeMap::new();

    parameters.insert(
        Name::of::<ParameterName>(),
        vec![
            String::from("some-parameter-value"),
            String::from("other-parameter-value")
        ]
    );

    parameters
};

let expected: Vec<String> = vec![
    "/",
    "/fixed",
    "/fixed/nested",
    "/some-parameter-value",
    "/other-parameter-value",
].into_iter().map(String::from).collect();
assert_eq!(expected, prepare_routes().gen_parameter_sitemap(&parameters));
```

[`Segment`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.Segment.html
