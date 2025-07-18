This method interacts with information from the current request. The request may come from:

1. The initial SSR render if this method called from a [`Component`](dioxus_core_macro::component) or a [`server`](dioxus_server) function that is called during the initial render

```rust
# use dioxus::prelude::*;
#[component]
fn PrintHtmlRequestInfo() -> Element {
    // The server context only exists on the server, so we need to put it behind a server_only! config
    server_only! {
        // Since we are calling this from a component, the server context that is returned will be from
        // the html request for ssr rendering
        let context = server_context();
        let request_parts = context.request_parts();
        println!("headers are {:?}", request_parts.headers);
    }
    rsx! {}
}
```

2. A request to a [`server`](dioxus_server) function called directly from the client (either on desktop/mobile or on the web frontend after the initial render)

```rust
# use dioxus::prelude::*;
#[server]
async fn read_headers() -> ServerFnResult {
    // Since we are calling this from a server function, the server context that is may be from the
    // initial request or a request from the client
    let context = server_context();
    let request_parts = context.request_parts();
    println!("headers are {:?}", request_parts.headers);
    Ok(())
}

#[component]
fn CallServerFunction() -> Element {
    rsx! {
        button {
            // If you click the button, the server function will be called and the server context will be
            // from the client request
            onclick: move |_| async {
                _ = read_headers().await
            },
            "Call server function"
        }
    }
}
```
