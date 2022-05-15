# Name-based Navigation

When creating large applications keeping track of what path to navigate /
redirect to can become very tedious.

To solve this, Dioxus Router implements name-based navigation. We can give
routes arbitrary names (they are completely independent of the path) and later
tell the router to go to them.

The router will automatically create the actual path to navigate to, combining
both `fixed` segments and inlining the required parameters.

This has a few advantages over path-based navigation:
- Links can be created without knowing the actual path. This allows us to easily
  change paths later on without having to find all references in our app. (We
  can also localize the path, if we must.)
- The router can find invalid links.
- The router will inline the dynamic parameters. It will take care of encoding
  them as well.

> We will learn about what the router does when it encounters a non-existing
> name or missing parameter in the chapter about
> [Navigation Failures](../advanced/navigation-failures.md).

## Giving names to routes
```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
            .fixed(
                "other",
                Route::new(RcComponent(Other)).name("other_name")
            )
    });

    // ...
    # unimplemented!()
}
#
# fn Home(cx: Scope) -> Element { todo!() }
# fn Other(cx: Scope) -> Element { todo!() }
```

## Navigating to a named route
We now can navigate to those routes with their names. Notice the use of
[`NtName`].

> The path `/` automatically has the name `"root_index"`.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home" }
        Link {
            target: NtName("other_name", vec![], QNone),
            "Go to other"
        }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Other" }
        Link {
            target: NtName("root_index", vec![], QNone),
            "Go to home"
        }
    })
}
```

When you look closely, you will notice that in addition to the name we give
[`NtName`] two other values. The first of those is a `Vec` containing the
parameters to inline into the path. The second handles the query string.

> We will learn more about [query strings](../advanced/query.md) in a later
> chapter.

[`NtName`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NtName
