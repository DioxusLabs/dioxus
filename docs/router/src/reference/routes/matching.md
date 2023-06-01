# Matching Routes

> Make sure you understand how [catch all routes](./catch_all.md) work before
> reading this page.

When accepting parameters via the path, some complex applications might need to
decide what route should be active based on the format of that parameter.
_Matching_ routes make it easy to implement such behavior.

> The parameter will be URL decoded, both for checking if the route is active
> and when it is provided to the application.

> The example below is only for showing _matching route_ functionality. It is
> unfit for all other purposes.

## Code Example

> Notice that the parameter of a _matching route_ has the same type as a
> [_catch all route_](./catch_all.md).

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;
# extern crate regex;
use regex::Regex;

struct Name;

fn GreetingFemale(cx: Scope) -> Element {
    let route = use_route(cx).unwrap();
    let name = route.parameter::<Name>()
        .map(|name| {
            let mut name = name.to_string();
            name.remove(0);
            name
        })
        .unwrap_or(String::from("Anonymous"));

    render! {
        p { "Hello Mrs. {name}" }
    }
}

fn GreetingMale(cx: Scope) -> Element {
    let route = use_route(cx).unwrap();
    let name = route.parameter::<Name>()
        .map(|name| {
            let mut name = name.to_string();
            name.remove(0);
            name
        })
        .unwrap_or(String::from("Anonymous"));

    render! {
        p { "Hello Mr. {name}" }
    }
}

fn GreetingWithoutGender(cx: Scope) -> Element {
    let route = use_route(cx).unwrap();
    let name = route.parameter::<Name>()
        .map(|name| name.to_string())
        .unwrap_or(String::from("Anonymous"));

    render! {
        p { "Hello {name}" }
    }
}

fn GreetingKenobi(cx: Scope) -> Element {
    render! {
        p { "Hello there." }
        p { "General Kenobi." }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/fAnna").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::empty()
                .fixed("kenobi", comp(GreetingKenobi))
                .matching(
                    Regex::new("^f").unwrap(),
                    ParameterRoute::content::<Name>(comp(GreetingFemale))
                )
                .matching(
                    Regex::new("^m").unwrap(),
                    (comp(GreetingMale), Name { })
                )
                .catch_all((comp(GreetingWithoutGender), Name { }))
        }
    );

    render! {
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(html, "<p>Hello Mrs. Anna</p>");
```

## Matcher

In the example above, both _matching routes_ use a regular expression to specify
when they match. However, _matching routes_ are not limited to those. They
accept all types that implement the [`Matcher`] trait.

For example, you could (but probably shouldn't) implement a matcher, that
matches all values with an even number of characters:

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
#[derive(Debug)]
struct EvenMatcher;

impl Matcher for EvenMatcher {
    fn matches(&self, value: &str) -> bool {
        value.len() % 2 == 0
    }
}
```

[`Matcher`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/trait.Matcher.html
