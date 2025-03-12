# Subsecond

Subsecond is a hot-reloading library for Rust. It makes it easy to add Rust hot-reloading to your
existing Rust project with minimal integration overhead.

## Usage:

For library authors you can use "hot" functions with the `subsecond::current` function:

```rust
/// A user-facing tick / launch / start function
///
/// Typically this will be a request/response handler, a game loop, a main function, callback, etc
///
/// `current` accepts function pointers and Fn types
pub fn tick(handler: Fn(Request) -> Response) {
    // Create a "hot" function that we can inspect
    let hot_fn = subsecond::current(handler);

    // Check if this function has been patched
    if hot_fn.changed() {
        // do thing
    }

    // Register a handler to be called when the function is patched
    hot_fn.on_changed(|| /* do thing */);

    // Call the hot function
    hot_fn.call((request))
}
```

For application authors, you can use the `hot` macro to mark a function as hot-reloadable:

```rust
#[hot]
fn handle_request(request: Request) -> Response {
    // do thing
}
```

If a hot function is actively being called, then subsecond will rewind the stack to the "cleanest" entrypoint. For example, a hot-reloadable server will have two "hot" points: at the start of the server, and at the start of the request handler. When the server is reloaded, subsecond will rewind the stack to the first hot point, and then call the function again.

```rust
// Changes to `serve` will reload the server
#[hot]
fn serve() {
    let router = Router::new();
    router.get("/", handle_request);
    router.serve("0.0.0.0:8080");
}

// Changes below "handle_request" won't be reload the router
#[hot]
fn handle_request(request: Request) -> Response {
    // do thing
}
```

Framework authors can interleave their own hot-reload entrypoints alongside user code. This lets you add new anchors into long-running stateful code:

```rust
fn main() {
    // serve is "hot" and will rebuild the router if it changes
    webserver::serve("0.0.0.0:8080", || {
        Router::new()
            // get is "hot" and changes to handle_request won't rebuild the router
            .get("/", |req| Response::websocket(handle_socket))
    })
}

#[hot]
fn handle_socket(ws: &WebSocket) {
    // do things with the websocket
}
```


By default, `main` is a rooted hot point: subsecond will rewind all the way to `main` if it has to. This can cause issues if you perform side-effectual code (deleting files, leaking memory, etc). Subsecond does this by using stack unwinding. If your code does not support stack unwinding, subsecond might not be suitable for you. Subsecond will try its best to defer to the runtime to avoid the stack unwind issue but might not always be possible.
