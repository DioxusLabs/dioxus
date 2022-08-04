# Matching Routes

> Matching routes are almost identical to [parameter routes](./parameter.md).
> Make sure you understand how those work before reading this page.

Some complex applications might need to decide what route should be active based
on the format of a path parameter. _Matching_ routes allow us to that.

> The parameter will be decoded, both for checking if the route is active and
> when it is provided to the application.

## Code Example
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
# use dioxus_router::history::MemoryHistory;
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
            .fixed("kenobi", Route::new(RcComponent(GreetingKenobi)))
            .matching(
                Regex::new("^f").unwrap(),
                ParameterRoute::new("name", RcComponent(GreetingFemale))
            )
            .matching(
                Regex::new("^m").unwrap(),
                ParameterRoute::new("name", RcComponent(GreetingMale))
            )
            .parameter(
                ParameterRoute::new("name", RcComponent(GreetingWithoutGender))
            )
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,
            # history: &|| MemoryHistory::with_first(String::from("/fAnna")),

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
