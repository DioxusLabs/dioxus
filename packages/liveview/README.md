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



```rust
use soyuz::prelude::*;

#[tokio::main]
async fn main() {
    let mut app = soyuz::new();
    app.at("/app").get(websocket(handler));
    app.listen("127.0.0.1:8080").await.unwrap();
}

async fn order_shoes(mut req: WebsocketRequest) -> Response {
    let stream = req.upgrade();
    dioxus::liveview::launch(App, stream).await;    
}

fn App(cx: Scope<()>) -> Element {
    let mut count = use_state(&cx, || 0);
    cx.render(rsx!(
        button { onclick: move |_| count += 1, "Incr" }
        button { onclick: move |_| count -= 1, "Decr" }
    ))
}
```
