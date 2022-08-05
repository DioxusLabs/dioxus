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

## Defining the content components

We start by creating the components we want the router to render.

Much like the [`Router`] component, our `Settings` component will contain
content controlled by the router. Therefore it also needs to contain an
[`Outlet`].

```rust,no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;

fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Settings" }

        // tell the router where to put the actual settings
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
        Segment::new().fixed("settings", Route::new(RcComponent(Settings)))
    });

    // ...
    # unimplemented!()
}
```

## Defining the nested [`Segment`]

In order to create nested routes we need to create a nested [`Segment`]. We then
pass it to the [`Route`] on the root segment.

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
            .fixed("settings", Route::new(RcComponent(Settings)).nested(
                Segment::new()
                    .index(RcComponent(GeneralSettings))
                    .fixed("password", Route::new(RcComponent(PWSettings)))
                    .fixed("privacy", Route::new(RcComponent(PrivacySettings)))
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
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::prelude::*;
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
            .fixed("settings", Route::new(RcComponent(Settings)).nested(
                Segment::new()
                    .index(RcComponent(GeneralSettings))
                    .fixed("password", Route::new(RcComponent(PWSettings)))
                    .fixed("privacy", Route::new(RcComponent(PrivacySettings)))
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
# let html = dioxus_ssr::render_vdom(&vdom);
# assert_eq!("<h1>Settings</h1><h2>Privacy Settings</h2>", html);
```

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Route`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Route.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
