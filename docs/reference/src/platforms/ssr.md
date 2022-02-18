# Getting Started: Server-Side-Rendering

The Dioxus VirtualDom can be rendered to a string by traversing the Element Tree. This is implemented in the `ssr` crate where your Dioxus app can be directly rendered to HTML to be served by a web server.



## Setup


If you just want to render `rsx!` or a VirtualDom to HTML, check out the API docs. It's pretty simple:

```rust
// We can render VirtualDoms
let mut vdom = VirtualDom::new(app);
let _ = vdom.rebuild();
println!("{}", dioxus::ssr::render_vdom(&vdom));

// Or we can render rsx! calls directly
println!( "{}", dioxus::ssr::render_lazy(rsx! { h1 { "Hello, world!" } } );
```


However, for this guide, we're going to show how to use Dioxus SSR with `Axum`. 

Make sure you have Rust and Cargo installed, and then create a new project:

```shell
$ cargo new --bin demo
$ cd app
```

Add Dioxus with the `desktop` feature:

```shell
$ cargo add dioxus --features ssr
```

Next, add all the Axum dependencies. This will be different if you're using a different Web Framework
```
$ cargo add dioxus tokio --features full
$ cargo add axum
```

Your dependencies should look roughly like this:

```toml
[dependencies]
axum = "0.4.3"
dioxus = { version = "*", features = ["ssr"] }
tokio = { version = "1.15.0", features = ["full"] }
```


Now, setup your Axum app to respond on an endpoint.

```rust
use axum::{response::Html, routing::get, Router};
use dioxus::prelude::*;

#[tokio::main]
async fn main() {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{}", addr);

    axum::Server::bind(&addr)
        .serve(Router::new().route("/", get(app_endpoint)))
        .await
        .unwrap();
}
```

And then add our endpoint. We can either render `rsx!` directly:

```rust
async fn app_endpoint() -> Html<String> {
    Html(dioxus::ssr::render_lazy(rsx! {
            h1 { "hello world!" }
    }))
}
```

Or we can render VirtualDoms.

```rust
async fn app_endpoint() -> Html<String> {
    fn app(cx: Scope) -> Element {
        cx.render(rsx!(h1 { "hello world" }))
    }
    let mut app = VirtualDom::new(app);
    let _ = app.rebuild();
    
    Html(dioxus::ssr::render_vdom(&app))
}
```

And that's it!

> You might notice that you cannot hold the VirtualDom across an await point. Dioxus is currently not ThreadSafe, so it *must* remain on the thread it started. We are working on loosening this requirement.

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!
