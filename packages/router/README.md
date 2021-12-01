# Router hook for Dioxus apps

Dioxus-router provides a use_router hook that returns a different value depending on the route.
The router is generic over any value, however it makes sense to return a different set of VNodes
and feed them into the App's return VNodes.

Using the router should feel similar to tide's routing framework where an "address" book is assembled at the head of the app.

Here's an example of how to use the router hook:

```rust
#[derive(Clone, PartialEq, Serialize, Deserialize, Routable)]
enum AppRoute {
    Home, 
    Posts,
    NotFound
}

static App: FC<()> = |cx, props| {
    let route = use_router(cx, AppRoute::parse);
    
    match route {
        AppRoute::Home => rsx!(cx, Home {})
        AppRoute::Posts => rsx!(cx, Posts {})
        AppRoute::Notfound => rsx!(cx, Notfound {})
    }
};
```

Adding links into your app:

```rust
static Leaf: FC<()> = |cx, props| {
    rsx!(cx, div { 
        Link { to: AppRoute::Home } 
    })
}
```

Currently, the router is only supported in a web environment, but we plan to add 1st-party support via the context API when new renderers are available.
