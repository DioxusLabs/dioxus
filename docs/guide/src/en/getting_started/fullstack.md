> This guide assumes you read the [Web](web.md) guide and installed the [Dioxus-cli](https://github.com/DioxusLabs/cli)

# Fullstack development

We can combine the `dioxus-web` renderer with the `dioxus-ssr` renderer to create a full-stack Dioxus application. By combining server-side rendering with client-side hydration we can create an application that is initially rendered on the server and then hydrates the application on the client. Server-side rendering results in a fast first paint and make our page SEO-friendly. Client-side hydration makes our page responsive once the application has fully loaded.

To help make full-stack development easier, Dioxus provides a `dioxus-server` crate that integrates with popular web frameworks with utilities for full-stack development.

## Setup

For this guide, we're going to show how to use Dioxus with [Axum](https://docs.rs/axum/latest/axum/), but `dioxus-server` also integrates with the [Warp](https://docs.rs/warp/latest/warp/) and [Salvo](https://docs.rs/salvo/latest/salvo/) web frameworks.

Make sure you have Rust and Cargo installed, and then create a new project:

```shell
cargo new --bin demo
cd demo
```

Add `dioxus` and `dioxus-server` as dependencies:

```shell
cargo add dioxus
cargo add dioxus-server --features axum, ssr
```

Next, add all the Axum dependencies. This will be different if you're using a different Web Framework

```shell
cargo add tokio --features full
cargo add axum
```

Your dependencies should look roughly like this:

```toml
[dependencies]
axum = "*"
dioxus = { version = "*" }
dioxus-server = { version = "*", features = ["axum", "ssr"] }
tokio = { version = "*", features = ["full"] }
```

Now, set up your Axum app to serve the Dioxus app.

```rust
{{#include ../../../examples/server.rs:basic}}
```

Now, run your app with `cargo run` and open `http://localhost:8080` in your browser. You should see a server-side rendered page with a counter.

## Hydration

Right now, the page is static. We can't interact with the buttons. To fix this, we can hydrate the page with `dioxus-web`.

First, modify your `Cargo.toml` to include two features, one for the server called `ssr`, and one for the client called `web`.

```toml
[dependencies]
# Common dependancies
dioxus = { version = "*" }
dioxus-server = { version = "*" }

# Web dependancies
dioxus-web = { version = "*", features=["hydrate"], optional = true }

# Server dependancies
axum = { version = "0.6.12", optional = true }
tokio = { version = "1.27.0", features = ["full"], optional = true }

[features]
default = []
ssr = ["axum", "tokio", "dioxus-server/axum"]
web = ["dioxus-web"]
```

Next, we need to modify our `main.rs` to use either hydrate on the client or render on the server depending on the active features.

```rust
{{#include ../../../examples/server.rs:hydration}}
```

Now, build your client-side bundle with `dioxus build --features web` and run your server with `cargo run --features ssr`. You should see the same page as before, but now you can interact with the buttons!

## Communicating with the server

`dixous-server` provides server functions that allow you to call an automatically generated API on the server from the client as if it were a local function.

To make a server function, simply add the `#[server(YourUniqueType)]` attribute to a function. The function must:

- Be an async function
- Have arguments and a return type that both implement serialize and deserialize (with [serde](https://serde.rs/)).
- Return a `Result` with an error type of ServerFnError

You must call `register` on the type you passed into the server macro in your main function before starting your server to tell Dioxus about the server function.

Let's add a server function to our app that allows us to multiply the count by a number on the server.

First, add serde as a dependency:

```shell
cargo add serde
```

Next, add the server function to your `main.rs`:

```rust
{{#include ../../../examples/server.rs:server_function}}
```

Now, build your client-side bundle with `dioxus build --features web` and run your server with `cargo run --features ssr`. You should see a new button that multiplies the count by 2.

## Conclusion

That's it! You've created a full-stack Dioxus app. You can find more examples of full-stack apps and information about how to integrate with other frameworks and desktop renderers in the [dioxus-server examples directory](https://github.com/DioxusLabs/dioxus/tree/master/packages/server/examples).
