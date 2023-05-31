# Server-Side Rendering

For lower-level control over the rendering process, you can use the `dioxus-ssr` crate directly. This can be useful when integrating with a web framework that `dioxus-server` does not support, or pre-rendering pages.

## Setup

For this guide, we're going to show how to use Dioxus SSR with [Axum](https://docs.rs/axum/latest/axum/).

Make sure you have Rust and Cargo installed, and then create a new project:

```shell
cargo new --bin demo
cd demo
```

Add Dioxus and the ssr renderer as dependencies:

```shell
cargo add dioxus
cargo add dioxus-ssr
```

Next, add all the Axum dependencies. This will be different if you're using a different Web Framework

```
cargo add tokio --features full
cargo add axum
```

Your dependencies should look roughly like this:

```toml
[dependencies]
axum = "0.4.5"
dioxus = { version = "*" }
dioxus-ssr = { version = "*" }
tokio = { version = "1.15.0", features = ["full"] }
```

Now, set up your Axum app to respond on an endpoint.

```rust
{{#include ../../../examples/hello_world_ssr.rs:main}}
```

And then add our endpoint. We can either render `rsx!` directly:

```rust
{{#include ../../../examples/hello_world_ssr.rs:endpoint}}
```

Or we can render VirtualDoms.

```rust
{{#include ../../../examples/hello_world_ssr.rs:second_endpoint}}
```

And then add our app component:

```rust
{{#include ../../../examples/hello_world_ssr.rs:component}}
```

And that's it!


## Multithreaded Support

The Dioxus VirtualDom, sadly, is not currently `Send`. Internally, we use quite a bit of interior mutability which is not thread-safe.
When working with web frameworks that require `Send`, it is possible to render a VirtualDom immediately to a String – but you cannot hold the VirtualDom across an await point. For retained-state SSR (essentially LiveView), you'll need to spawn a VirtualDom on its own thread and communicate with it via channels or create a pool of VirtualDoms.
You might notice that you cannot hold the VirtualDom across an await point. Because Dioxus is currently not ThreadSafe, it _must_ remain on the thread it started. We are working on loosening this requirement.
