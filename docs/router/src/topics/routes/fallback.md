# Fallback Routes

> Fallback routes are almost identical to [parameter routes](./parameter.md).
> Make sure you understand how those work before reading this page.
>
> The main differences are:
> - _Fallback_ routes are for handling invalid paths.
> - They don't give you easy access to the segments value.
> - They don't allow nested routes.

When defining routes you might want to handle the possibility of invalid paths.
This is especially important for web apps, where the user can manually change
the path via the URL bar.

The router allows you to handle such invalid path using two methods:
- The [global fallback] allows you to handle invalid
  paths within your entire application. This is similar to how some web servers
  treat a `/404.html` file.
- The fallback routes described in this chapter provide a more local approach.

> To learn more about the interaction between the [global fallback] and
> _fallback_ routes, see the [global fallback] chapter.

A fallback route is active in two cases:
- **No local route is active**: the path specifies the current [`Segment`], but
  no [_fixed_](./index.md#fixed-routes) or [_matching_](./matching.md) route is
  active.
- **Too specific path**: The active route on the current [`Segment`] has no
  [_nested_ segment](./nested.md), but the path specifies one.

## Example
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::router::history::MemoryHistory;
use dioxus::prelude::*;

fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Index" }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Other" }
    })
}

fn Fallback(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Content not found" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(RcComponent(Index))
            .fixed("other", Route::new(RcComponent(Other)))
            .fallback(RcComponent(Fallback))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,
            # history: &|| {
            #     MemoryHistory::with_first(String::from("/other/invalid"))
            # }

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus::ssr::render_vdom(&vdom);
# assert_eq!("<h1>Content not found</h1>", html);
```

In the above example the following routes will be active when:
- The [_index_ route](./index.md#index-routes) is active when the path is `/`.
- The [_fixed_ route](./index.md#fixed-routes) is active when the path is
  `/other`.
- The _fallback_ route is active, when the first segment is anything other than
  `/` or `/other`, e.g. `/test` or `/invalid`.
- The _fallback_ route is also active, when the path specifies a nested route
  for `/other`, e.g. `/other/test`, `/other/invalid`, or `/other//` (notice the
  second slash).

[global fallback]: ../global_fallback.md
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
