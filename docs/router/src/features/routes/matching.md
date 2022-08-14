# Matching Routes

> Make sure you understand how [parameter routes](./parameter.md) work before
> reading this page.

When accepting parameters via the path, some complex applications might need to
decide what route should be active based on the format of that parameter.
_Matching_ routes make it easy to implement such behavior.

> The parameter will be URL decoded, both for checking if the route is active
> and when it is provided to the application.

> The example below is only for showing _matching route_ functionality. It is
> unfit for all other purposes.

## Code Example
> Notice that the second parameter of a _matching route_ has the same type as a
> [_parameter route_](./parameter.md).

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
# extern crate regex;
use regex::Regex;

fn GreetingFemale(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();
    let name = route.parameters.get("name")
        .map(|name| {
            let mut name = name.to_string();
            name.remove(0);
            name
        })
        .unwrap_or(String::from("Anonymous"));

    cx.render(rsx! {
        p { "Hello Mrs. {name}" }
    })
}

fn GreetingMale(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();
    let name = route.parameters.get("name")
        .map(|name| {
            let mut name = name.to_string();
            name.remove(0);
            name
        })
        .unwrap_or(String::from("Anonymous"));

    cx.render(rsx! {
        p { "Hello Mr. {name}" }
    })
}

fn GreetingWithoutGender(cx: Scope) -> Element {
    let route = use_route(&cx).unwrap();
    let name = route.parameters.get("name")
        .map(|name| name.to_string())
        .unwrap_or(String::from("Anonymous"));

    cx.render(rsx! {
        p { "Hello {name}" }
    })
}

fn GreetingKenobi(cx: Scope) -> Element {
    cx.render(rsx! {
        p { "Hello there." }
        p { "General Kenobi." }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .fixed("kenobi", GreetingKenobi as Component)
            .matching(
                Regex::new("^f").unwrap(),
                ParameterRoute::new("name", GreetingFemale as Component)
            )
            .matching(
                Regex::new("^m").unwrap(),
                ("name", GreetingMale as Component)
            )
            .parameter(("name", GreetingWithoutGender as Component))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # initial_path: "/fAnna",

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<p>Hello Mrs. Anna</p>", html);
```
