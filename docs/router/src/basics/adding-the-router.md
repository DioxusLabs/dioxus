# Adding the router

In this chapter we will learn how to add a [`Router`] component to our app,
which is the base on which all routing functionality relies.

> Make sure you have added the `router` feature to Dioxus as explained in the
> [introduction](/).

## Preparing some components
Before we add the [`Router`] we will prepare a few components to show on
different routes:

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
// This is the first page the user will see.
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home Page" }
    })
}

// The user can navigate to this page using a Link.
fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Other Page" }
    })
}
```

## Defining the routes
To determine what components to render, the router needs to know what routes our
app consists of. We provide that information by creating a [`Segment`], which we
will later pass to the [`Router`] component.

In our example example, we will define two routes:
1. When the user opens our app/website we want to show the `Home` component. The
   path for this is `/`.
2. When the user navigates to `/other` we want to show the `Other` component.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
            .fixed(
                "other",
                Route::new(RcComponent(Other))
            )
    });

    // ...
    # todo!()
}
#
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn Other(cx: Scope) -> Element { unimplemented!() }
```

## Adding the [`Router`]
Now we can add the [`Router`] component. We will give it the routes we defined
before as a property.

Also note that we give it an [`Outlet`] as a child. The outlet is the place
where the content of the active route will be rendered.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    # let routes = use_segment(&cx, || {
    #     Segment::default()
    #         .index(RcComponent(Home))
    #         .fixed(
    #             "other",
    #             Route::new(RcComponent(Other))
    #         )
    # });
    // ...

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            Outlet { }
        }
    })
}
#
# fn Home(cx: Scope) -> Element { unimplemented!() }
# fn Other(cx: Scope) -> Element { unimplemented!() }
```

## Links
We now have a router with two different routes. However, our users still need a
way to navigate between them.

In regular HTML we would use an anchor tag for that, like this:
```html
<a href="/other">Link to an other page</a>
```

While this works well on regular web pages, we cannot use this within our app
for a few reasons:
- Anchor tags trigger the browser to perform a full page load. We don't want
  this interruption within our app, as it is much faster to handle in-app
  navigation client-side.
- In some contexts, like Dioxus Desktop, there is no server to load a new page
  from. All navigation must happen client side.

To solve these problems, Dioxus Router provides us with a [`Link`] component we
can use like this:
```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home Page" }
        Link {
            // where the link should point
            target: NtPath(String::from("/other")),
            // the links content
            "Go to the other page"
        }
    })
}
```

> You might have noticed that we wrap the path we navigate to with [`NtPath`].
> We will learn more about this in the chapters about
> [name-based navigation](./name-based-navigation.md) and
> [external navigation targets](../advanced/external-navigation-targets.md).

## Full code
```rust
# extern crate dioxus;
use dioxus::prelude::*;

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .index(RcComponent(Home))
            .fixed(
                "other",
                Route::new(RcComponent(Other))
            )
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            Outlet { }
        }
    })
}

fn Home(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Home Page" }
        Link {
            target: NtPath(String::from("/other")),
            "Go to the other page"
        }
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Other Page" }
        Link {
            target: NtPath(String::from("/")),
            "Go to home"
        }
    })
}
```

[`Link`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Link.html
[`NtPath`]: https://docs.rs/dioxus-router/latest/dioxus_router/navigation/enum.NavigationTarget.html#variant.NtPath
[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
