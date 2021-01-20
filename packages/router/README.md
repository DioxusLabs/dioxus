# Router hook for Dioxus apps

Dioxus-router provides a use_router hook that returns a different value depending on the route. 
The router is generic over any value, however it makes sense to return a different set of VNodes 
and feed them into the App's return VNodes.

Using the router should feel similar to tide's routing framework where an "address" book is assembled at the head.

Here's an example of how to use the router hook:

```rust
static App: FC<()> = |ctx| {

    // Route returns the associated VNodes 
    // This hook re-fires when the route changes
    let route = use_router(ctx, |cfg| {
        cfg.at("/").serve(|ctx| {
            html!{ <LandingPage /> }
        });
        
        cfg.at("/shoes/:id").serve(|ctx| {
            let id: Uuid = ctx.props.parse().unwrap();
            html!{ <ShoesPage id=id /> }
        });

        cfg.at("/pants/:id").serve(|ctx| {
            let id: Uuid = ctx.props.parse().unwrap();
            html!{ <PantsPage id=id /> }
        });
    });

    html! {
        <PanicBoundary model={|_| html!{<div>"Uh oh!"</div>}}>
            <StateManager>
                <ThemeSystem>
                    <Header />
                    {route}
                </ThemeSystem>
            </StateManager>
        </PanicBoundary >
    }
};
```

Currently, the router is only supported in a web environment, but we plan to add 1st-party support via the context API when new renderers are available.
