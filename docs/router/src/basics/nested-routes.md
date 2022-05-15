# Nested Routes
Non-trivial applications often nest multiple views within each other. As an
example, consider a settings view, that contains multiple kinds of settings.
```plain
└ Settings
  ├ General Settings (displayed when opening the settings)
  ├ Change Password
  └ Privacy Settings
```

We might want to decide which of those components to render based on the path
like this:
```plain
/settings          -> Settings { GeneralSettings }
/settings/password -> Settings { ChangePassword }
/settings/privacy  -> Settings { PrivacySettings }
```

We can do this using nested routes.

## Defining the root [`Segment`]
First we define the root segment. It is responsible for mounting the `Settings`
component when the path starts with `/settings`

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .fixed(
                "settings",
                Route::new(RcComponent(Settings))
            )
    });

    // ...
    # unimplemented!()
}
#
# fn Settings(cx: Scope) -> Element { unimplemented!() }
```

## Defining a nested [`Segment`]
We now can nest a second segment within the [`Route`] we defined. It contains
routes for the nested components.
```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn App(cx: Scope) -> Element {
    let routes = use_segment(&cx, || {
        Segment::default()
            .fixed(
                "settings",
                Route::new(RcComponent(Settings)).sub(
                    Segment::default()
                        .index(RcComponent(GeneralSettings))
                        .fixed(
                            "password",
                            Route::new(RcComponent(ChangePassword)),
                        )
                        .fixed(
                            "privacy",
                            Route::new(RcComponent(PrivacySettings)),
                        ),
                ),
            )
    });

    // ...
    # unimplemented!()
}
#
# fn Settings(cx: Scope) -> Element { unimplemented!() }
# fn GeneralSettings(cx: Scope) -> Element { unimplemented!() }
# fn ChangePassword(cx: Scope) -> Element { unimplemented!() }
# fn PrivacySettings(cx: Scope) -> Element { unimplemented!() }
```

## [`Outlet`]s
When using nested routes, we need to provide equally nested [`Outlet`]s. In the
above example, we need to have an outlet in our `Settings` component.

```rust
# extern crate dioxus;
# use dioxus::prelude::*;
fn Settings(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Settings" }
        Outlet { }
    })
}
```

[`Outlet`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Outlet.html
[`Route`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Route.html
[`Router`]: https://docs.rs/dioxus-router/latest/dioxus_router/components/fn.Router.html
[`Segment`]: https://docs.rs/dioxus-router/latest/dioxus_router/route_definition/struct.Segment.html
