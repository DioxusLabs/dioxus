# Routing for Dioxus App

DioxusRouter adds React-Router style routing to your Dioxus apps. Works in browser, SSR, and natively.

```rust
fn app() {
    cx.render(rsx! {
        Router {
            Route { to: "/", Component {} },
            Route { to: "/blog", Blog {} },
            Route { to: "/blog/:id", BlogPost {} },
        }
    })
}
```

Then, in your route, you can choose to parse the Route any way you want through `use_route`.
```rust
let id: usize = use_route(&cx).segment("id")?;

let state: CustomState = use_route(&cx).parse()?;
```

Adding links into your app:
```rust
Link { to: "id/{id}" }
```

Currently, the router is only supported in a web environment, but we plan to add 1st-party support via the context API when new renderers are available.
