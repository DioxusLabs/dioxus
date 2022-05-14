# Programmatic Navigation
In addition to using links and redirects, we can also employ programmatic
navigation. This allows us to change the current route from within our code.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn SomeComponent(cx: Scope) -> Element {
    // get the navigator
    let nav = use_navigate(&cx)?;

    // navigate to the provided path
    nav.push(NtPath(String::from("/")));

    // we always redirect, we don't need actual content
    None
}
```

> Note that we can use [name-based navigation][nn] withing programmatic
> navigation.

[nn]: ./name-based-navigation.md
