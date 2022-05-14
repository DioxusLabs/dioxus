# Basics

In this chapter we will learn how to add a [`Router`] component to our app,
which is the base on which the rest of the router relies.

> Make sure you have added the `router` feature to Dioxus as explained in the
> [introduction].


## Preparing some components
Before we can add the [`Router`] we need to prepare a few components to show
on different routes:

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
To be able to make a decision on what content to show the user, the router needs
to know what routes our app includes. To give it that information, we define
a [`Segment`], which we will later pass to the [`Router`] component.

In our example example, we will define these two routes:
- If the path is `/`, the router will render our `Home` component.
- If the path is `/other`, the router will render our `Other` component.

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
Now we can finally add the [`Router`] component to our app. Note that we give it
an [`Outlet`] as a child. This tells the router where to put the content of the
active route.

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
    })
}

fn Other(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Other Page" }
    })
}
```

[introduction]: /
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
