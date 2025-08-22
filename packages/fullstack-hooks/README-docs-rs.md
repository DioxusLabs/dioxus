# Dioxus Fullstack Hooks

Dioxus fullstack hooks provides hooks and contexts for [`dioxus-fullstack`](https://crates.io/crates/dioxus-fullstack). Libraries that need to integrate with dioxus-fullstack should rely on this crate instead of the renderer for quicker build times.

## Usage

To start using this crate, you can run the following command:

```bash
cargo add dioxus-fullstack-hooks
```

Then you can use hooks like `use_server_future` in your components:

```rust
use dioxus::prelude::*;
async fn fetch_article(id: u32) -> String {
    format!("Article {}", id)
}

fn App() -> Element {
    let mut article_id = use_signal(|| 0);
    // `use_server_future` will spawn a task that runs on the server and serializes the result to send to the client.
    // The future will rerun any time the
    // Since we bubble up the suspense with `?`, the server will wait for the future to resolve before rendering
    let article = use_server_future(move || fetch_article(article_id()))?;

    rsx! {
        "{article().unwrap()}"
    }
}
```
