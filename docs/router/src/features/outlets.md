# Outlets

[`Outlet`]s tell the router where to render content. In the following example
the active routes content will be rendered within the [`Outlet`].

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;

fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Index" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().index(Index as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,

            header { "header" }
            Outlet {}
            footer { "footer" }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!(
#     "<header>header</header><h1>Index</h1><footer>footer</footer>",
#     html
# );
```

The example above will output the following HTML (line breaks added for
readability):
```html
<header>
    header
</header>
<h1>
    Index
</h1>
<footer>
    footer
</footer>
```

## Nested Outlets
When using nested routes, we need to provide equally nested [`Outlet`]s.

> Learn more about [nested routes](./routes/nested.md) in their own chapter.

## Named Outlets
When building complex apps, we often need to display multiple pieces of content
simultaneously. For example, we might have a sidebar that changes its content in
sync with the main part of the page.

When defining our routes, we can use `RouteContentMulti` instead of
`RouteContent::Component` (we've been using this through the `Into` trait) to
tell the router about our content.

We then can use a named [`Outlet`] in our output, to tell the router where to
put the side content.

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
#
fn Main(cx: Scope) -> Element {
    cx.render(rsx! {
        main { "Main Content" }
    })
}

fn Aside(cx: Scope) -> Element {
    cx.render(rsx! {
        aside { "Side Content" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(RouteContent::Multi(Main, vec![("side", Aside)]))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,

            Outlet { }
            Outlet {
                name: "side"
            }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<main>Main Content</main><aside>Side Content</aside>", html);
```

The example above will output the following HTML (line breaks added for
readability):
```html
<main>
    Main Content
</main>
<aside>
    Side Content
</aside>
```

## Outlet depth override
When nesting [`Outlet`]s, they communicate with each other. This allows the
nested [`Outlet`] to render the content of the nested route.

We can override the detected value. Be careful when doing so, it is incredibly
easy to create an unterminated recursion. See below for an example of that.

```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# extern crate dioxus_ssr;
#
fn RootContent(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Root" }
        Outlet { }
    })
}

fn NestedContent(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Nested" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().fixed(
            "root",
            Route::new(RootContent as Component).nested(
                Segment::new().index(NestedContent as Component)
            )
        )
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # initial_path: "/root",

            Outlet {
                depth: 1
            }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<h2>Nested</h2>", html);
```

The example above will output the following HTML (line breaks added for
readability):
```html
<h2>
    Nested
</h2>
```

### Outlet recursion
This code will create a crash due to an unterminated recursion using
[`Outlet`]s.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
#
fn Content(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Heyho!" }
        Outlet {
            depth: 0,
        }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().index(Content as Component)
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),

            Outlet { }
        }
    })
}
```

The [`Outlet`] directly within the [`Router`] has no parent [`Outlet`], so its
depth will be `0`. When rendering for the path `/`, it therefore will render the
`Content` component.

The `Content` component will render an `h1` and an [`Outlet`]. That [`Outlet`]
would usually have a depth of `1`, since its a descendant of the [`Outlet`] in
the [`Router`]. However, we override its depth to `0`, so it will render the
`Content` component.

That means the `Content` component will recurse until someone (e.g. the OS) puts
a stop to it.

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
