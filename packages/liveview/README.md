# Dioxus LiveView

Enabling server-rendered and hybrid applications with incredibly low latency (<1ms).

```rust
#[async_std::main]
async fn main() -> tide::Result<()> {
    let liveview_pool = dioxus::liveview::pool::default();
    let mut app = tide::new();

    // serve the liveview client
    app.at("/").get(dioxus::liveview::liveview_frontend);

    // and then connect the client to the backend
    app.at("/app").get(|req| dioxus::liveview::launch(App, Props { req }))

    app.listen("127.0.0.1:8080").await?;

    Ok(())
}
```

Dioxus LiveView runs your Dioxus apps on the server 

