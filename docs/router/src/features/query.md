# Query

Some apps use the query part of the URL to encode information. The router allows
you to easily access the query, as well as set it when navigating.

## Accessing the query
The [`use_route`] hook allows us to access the current query in two ways. The
returned `struct` contains a `query` field, that contains the query (without the
leading `?`). Alternatively we can use the `query_params` function to get a
`BTreeMap` containing the query values.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn SomeComponent(cx: Scope) -> Element {
    let route = use_route(&cx).expect("nested in Router");

    let query = route.query.clone().unwrap();
    let query_params = route.query_params().unwrap();

    // ...
    # unimplemented!()
}
```

## Setting the query
When navigating we can tell the router to change the query. However, the method
we use to do this is very different, depending on how we specify our target.

### [`InternalTarget`] and [`ExternalTarget`]
When using [`InternalTarget`] or [`ExternalTarget`] we have to append our query
manually.

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
            target: InternalTarget(String::from("/some/path?query=yes")),
            "Internal target"
        }
        Link {
            target: ExternalTarget(String::from("https://dioxuslab.com?query=yes")),
            "External target"
        }
    })
}
```

### [`NamedTarget`]
When using [named navigation](./navigation/name.md) we can pass the query via
the last tuple field.

Just like with [`NamedTarget`] itself, we can rely on automatic conversion from
various types. We can either provide a `String` or a `Vec` of key-value pairs.
When using a `String`, it doesn't matter if it is prefixed with a `?`.

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
            target: (Target, [], "query=yes"),
            "Query String"
        }
        Link {
            target: (Target, [], vec![("query", "yes")]),
            "Query Vec"
        }
    })
}
```

[`ExternalTarget`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.ExternalTarget
[`InternalTarget`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.InternalTarget
[`NamedTarget`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NamedTarget
[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
root_index
