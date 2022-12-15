# Query

Some apps use the query part of the URL to encode information. The router allows
you to easily access the query, as well as set it when navigating.

## Accessing the query
The [`use_route`] hook allows us to access the current query in two ways. The
returned `struct` contains a `query` field, that contains the query (without the
leading `?`).

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn SomeComponent(cx: Scope) -> Element {
    let route = use_route(&cx).expect("nested in Router");

    let query = route.query.clone().unwrap();

    // ...
    # unimplemented!()
}
```

## Setting the query
When navigating we can tell the router to change the query. However, the method
we use to do this is very different, depending on how we specify our target.

### [`Internal`] and [`External`]
When using [`Internal`] or [`External`] we have to append our query manually.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn SomeComponent(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NavigationTarget::Internal("/some/path?query=yes".into()),
            "Internal target"
        }
        Link {
            target: NavigationTarget::External("https://dioxuslab.com?query=yes".into()),
            "External target"
        }
    })
}
```

### [`Named`]
When using [named navigation](./navigation/name.md) we can pass the query via
a function.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# struct Target;
#
fn SomeComponent(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: named::<Target>().query("query=yes"),
            "Query String"
        }
        Link {
            target: named::<Target>().query(vec![("query", "yes")]),
            "Query Vec"
        }
    })
}
```

[`External`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.External
[`Internal`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Internal
[`Named`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/navigation/enum.NavigationTarget.html#variant.Named
[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
