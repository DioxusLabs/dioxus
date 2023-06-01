# Fallback Routes

Sometimes the router might be unable to find a route for the provided path. We
might want it to show a prepared error message to our users in that case.
Fallback routes allow us to do that.

> This is especially important for use cases where users can manually change the
> path, like web apps running in the browser.

## A single global fallback

To catch all cases of invalid paths within our app, we can simply add a fallback
route to our root [`Segment`].

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;

fn Index(cx: Scope) -> Element {
    render! {
        h1 { "Index" }
    }
}

fn Fallback(cx: Scope) -> Element {
    render! {
        h1 { "Error 404 - Not Found" }
        p { "The page you asked for doesn't exist." }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/invalid").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::content(comp(Index)).fallback(comp(Fallback))
        }
    );

    render! {
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     dioxus_ssr::render(&vdom),
#     "<h1>Error 404 - Not Found</h1><p>The page you asked for doesn't exist.</p>"
# );
```

## More specific fallback routes

In some cases we might want to show different fallback content depending on what
section of our app the user is in.

For example, our app might have several settings pages under `/settings`, such
as the password settings `/settings/password` or the privacy settings
`/settings/privacy`. When our user is in the settings section, we want to show
them _"settings not found"_ instead of _"page not found"_.

We can easily do that by setting a fallback route on our nested [`Segment`]. It
will then replace the global fallback whenever our [`Segment`] was active.

Note the `.clear_fallback(false)` part. If we didn't add this, the fallback
content would be rendered inside the `Settings` component.

```rust, no_run
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;
# extern crate dioxus_router;
use dioxus_router::{history::MemoryHistory, prelude::*};
# extern crate dioxus_ssr;

// This example doesn't show the index or settings components. It only shows how
// to set up several fallback routes.
# fn Index(cx: Scope) -> Element { unimplemented!() }
# fn Settings(cx: Scope) -> Element { unimplemented!() }
# fn GeneralSettings(cx: Scope) -> Element { unimplemented!() }
# fn PasswordSettings(cx: Scope) -> Element { unimplemented!() }
# fn PrivacySettings(cx: Scope) -> Element { unimplemented!() }

fn GlobalFallback(cx: Scope) -> Element {
    render! {
        h1 { "Error 404 - Page Not Found" }
    }
}

fn SettingsFallback(cx: Scope) -> Element {
    render! {
        h1 { "Error 404 - Settings Not Found" }
    }
}

fn App(cx: Scope) -> Element {
    use_router(
        cx,
        &|| RouterConfiguration {
            # synchronous: true,
            # history: Box::new(MemoryHistory::with_initial_path("/settings/invalid").unwrap()),
            ..Default::default()
        },
        &|| {
            Segment::empty()
                .fixed("settings", Route::content(comp(Settings)).nested(
                    Segment::content(comp(GeneralSettings))
                        .fixed("password", comp(PasswordSettings))
                        .fixed("privacy", comp(PrivacySettings))
                        .fallback(comp(SettingsFallback))
                        .clear_fallback(true)
                ))
                .fallback(comp(GlobalFallback))
        }
    );

    render! {
        Outlet { }
    }
}
#
# let mut vdom = VirtualDom::new(App);
# vdom.rebuild();
# assert_eq!(
#     dioxus_ssr::render(&vdom),
#     "<h1>Error 404 - Settings Not Found</h1>"
# );
```

[`Segment`]: https://docs.rs/dioxus-router-core/latest/dioxus_router_core/routes/struct.Segment.html
