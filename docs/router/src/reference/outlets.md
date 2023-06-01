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
    render! {
        h1 { "Index" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            ..Default::default()
        },
        &|| Segment::content(comp(Index))
    );

    render! {
        header { "header" }
        Outlet { }
        footer { "footer" }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(
#     html,
#     "<header>header</header><h1>Index</h1><footer>footer</footer>"
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
    render! {
        main { "Main Content" }
    }
}

struct AsideName;
fn Aside(cx: Scope) -> Element {
    render! {
        aside { "Side Content" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            ..Default::default()
        },
        &|| {
            Segment::content(
                multi(Some(comp(Main)))
                    .add_named::<AsideName>(comp(Aside))
            )
        }
    );

    render! {
            Outlet { }
            Outlet {
                name: Name::of::<AsideName>()
            }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(html, "<main>Main Content</main><aside>Side Content</aside>");
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
# use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;
#
fn RootContent(cx: Scope) -> Element {
    render! {
        h1 { "Root" }
        Outlet { }
    }
}

fn NestedContent(cx: Scope) -> Element {
    render! {
        h2 { "Nested" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/root").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::empty().fixed(
                "root",
                Route::content(comp(RootContent)).nested(
                    Segment::content(comp(NestedContent))
                )
            )
        }
    );

    render! {
        Outlet {
            depth: 1
        }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# let html = dioxus_ssr::render(&vdom);
# assert_eq!(html, "<h2>Nested</h2>");
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
    render! {
        h1 { "Heyho!" }
        Outlet {
            depth: 0,
        }
    }
}

fn App(cx: Scope) -> Element {
    use_router(cx, &Default::default, &|| Segment::content(comp(Content)));

    render! {
        Outlet { }
    }
}
```

The [`Outlet`] in the `App` component has no parent [`Outlet`], so its depth
will be `0`. When rendering for the path `/`, it therefore will render the
`Content` component.

The `Content` component will render an `h1` and an [`Outlet`]. That [`OUtlet`]
would usually have a depth of `1`, since it is a descendant of the [`Outlet`] in
the `App` component. However, we override its depth to `0`, so it will render
the `Content` component.

That means the `Content` component will recurse until someone (e.g. the OS) puts
a stop to it.

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
