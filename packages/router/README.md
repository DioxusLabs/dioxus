# Router hook for Dioxus apps

Dioxus-router provides a use_router hook that returns a different value depending on the route.
The router is generic over any value, however it makes sense to return a different set of VNodes
and feed them into the App's return VNodes.

Using the router should feel similar to tide's routing framework where an "address" book is assembled at the head of the app.

Here's an example of how to use the router hook:

```rust
static App: FC<()> = |cx| {

    // Route returns the associated VNodes
    // This hook re-fires when the route changes
    let route = use_router(cx, |router| {
        router.at("/").get(|path| {
            rsx!{ <LandingPage /> }
        });
        router.at("/shoes/:id").get(|path| {
            let id: Uuid = path.parse().unwrap();
            rsx!{ <ShoesPage id=id /> }
        });
        router.at("/pants/:id").get(|path| {
            let id: Uuid = path.parse().unwrap();
            rsx!{ <PantsPage id=id /> }
        });
    });

    cx.render(rsx!{
        div {
            Navbar {}
            {route}
            Footer {}
        }
    })
};
```

Currently, the router is only supported in a web environment, but we plan to add 1st-party support via the context API when new renderers are available.
