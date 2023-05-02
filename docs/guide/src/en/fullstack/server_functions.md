# Communicating with the server

`dixous-server` provides server functions that allow you to call an automatically generated API on the server from the client as if it were a local function.

To make a server function, simply add the `#[server(YourUniqueType)]` attribute to a function. The function must:

- Be an async function
- Have arguments and a return type that both implement serialize and deserialize (with [serde](https://serde.rs/)).
- Return a `Result` with an error type of ServerFnError

You must call `register` on the type you passed into the server macro in your main function before starting your server to tell Dioxus about the server function.

Let's continue building on the app we made in the [getting started](./getting_started.md) guide. We will add a server function to our app that allows us to double the count on the server.

First, add serde as a dependency:

```shell
cargo add serde
```

Next, add the server function to your `main.rs`:

```rust
{{#include ../../../examples/server_function.rs}}
```

Now, build your client-side bundle with `dioxus build --features web` and run your server with `cargo run --features ssr`. You should see a new button that multiplies the count by 2.

## Conclusion

That's it! You've created a full-stack Dioxus app. You can find more examples of full-stack apps and information about how to integrate with other frameworks and desktop renderers in the [dioxus-fullstack examples directory](https://github.com/DioxusLabs/dioxus/tree/master/packages/server/examples).
