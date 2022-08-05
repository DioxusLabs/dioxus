# Nested Routes

When developing bigger applications we often want to nest routes within each
other. As an example, we might want to organize a settings menu using this
pattern:

```plain
└ Settings
  ├ General Settings (displayed when opening the settings)
  ├ Change Password
  └ Privacy Settings
```

We might want to map this structure to these paths and components:

```plain
/settings          -> Settings { GeneralSettings }
/settings/password -> Settings { PWSettings }
/settings/privacy  -> Settings { PrivacySettings }
```

Nested routes allow us to do this.

## Route Depth
With nesting routes, the router manages content on multiple levels. In our
example, when the path is `/settings`, there are two levels of content:

0. The `Settings` component
1. The `GeneralSettings` component

Dioxus Router uses the [`Outlet`] component to actually render content, but each
[`Outlet`] can only render content from one level. This means that for the
content of nested routes to actually be rendered, we also need nested
[`Outlet`]s.

## Defining the content components
We start by creating the components we want the router to render.

Take a look at the `Settings` component. When it gets rendered by an [`Outlet`],
it will render a second [`Outlet`]. Thus the second [`Outlet`] is nested within
the first one, and will in turn render our nested content.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;

fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Settings" }
        Outlet { }
    })
}

fn GeneralSettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "General Settings" }
    })
}

fn PWSettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Password Settings" }
    })
}

fn PrivacySettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Privacy Settings" }
    })
}
```

## Defining the root [`Segment`]
Now we create the [`Segment`] that we will pass to the [`Router`].

Note that we wrap `Settings as Component` within a [`Route`]. For this exact
code that is unnecessary, as this would be done automatically. However, in the
next step we'll use a method of [`Route`], so we might as well add this now.

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Settings(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new().fixed("settings", Route::new(Settings as Component))
    });

    // ...
    # unimplemented!()
}
```

## Defining the nested [`Segment`]
In order to create nested routes we need to create a nested [`Segment`]. We then
pass it to the [`Route`] on the root segment.

> A [`Segment`] always refers to one exact segment of the path.
>
> https://router.example/`root_segment`/`first_nested_segment`/`second_nested_segment`/...

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# fn Settings(cx: Scope) -> Element { unimplemented!() }
# fn GeneralSettings(cx: Scope) -> Element { unimplemented!() }
# fn PWSettings(cx: Scope) -> Element { unimplemented!() }
# fn PrivacySettings(cx: Scope) -> Element { unimplemented!() }
#
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .fixed("settings", Route::new(Settings as Component).nested(
                Segment::new()
                    .index(GeneralSettings as Component)
                    .fixed("password", PWSettings as Component)
                    .fixed("privacy", PrivacySettings as Component)
            ))
    });

    // ...
    # unimplemented!()
}
```

## Full Code
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
# use dioxus::prelude::*;
# extern crate dioxus_router;
# use dioxus_router::prelude::*;
# use dioxus_router::history::MemoryHistory;
# extern crate dioxus_ssr;

fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Settings" }
        Outlet { }
    })
}

fn GeneralSettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "General Settings" }
    })
}

fn PWSettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Password Settings" }
    })
}

fn PrivacySettings(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "Privacy Settings" }
    })
}

fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .fixed("settings", Route::new(Settings as Component).nested(
                Segment::new()
                    .index(GeneralSettings as Component)
                    .fixed("password", PWSettings as Component)
                    .fixed("privacy", PrivacySettings as Component)
            ))
    });

    cx.render(rsx! {
        Router {
            routes: routes.clone(),
            # init_only: true,
            # history: &|| {
            #     MemoryHistory::with_first(String::from("/settings/privacy"))
            # },

            Outlet { }
        }
    })
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     "<h1>Settings</h1><h2>Privacy Settings</h2>",
#     dioxus_ssr::render_vdom(&vdom)
# );
```

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Route`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Route.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
