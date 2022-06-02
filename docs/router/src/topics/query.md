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

### `NtPath` and `NtExternal`
When using `NtPath` or `NtExternal` we have to append our query manually.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

fn SomeComponent(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtPath(String::from("/some/path?query=yes")),
            "Internal target"
        }
        Link {
            target: NtExternal(String::from("https://dioxuslab.com?query=yes")),
            "External target"
        }
    })
}
```

### `NtName`
When using [named navigation](./navigation/name.md) we can pass the query via
the last tuple field.

We can either provide a query string, or a `Vec<(String, String)>`.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

fn SomeComponent(cx: Scope) -> Element {
    cx.render(rsx! {
        Link {
            target: NtName("target", vec![], QString(String::from("query=yes"))),
            "Query String"
        }
        Link {
            target: NtName(
                "target",
                vec![],
                QVec(vec![(String::from("query"), String::from("yes"))])
            ),
            "Query Vec"
        }
    })
}
```

[`use_route`]: https://docs.rs/dioxus-router/latest/dioxus_router/hooks/fn.use_route.html
root_index
