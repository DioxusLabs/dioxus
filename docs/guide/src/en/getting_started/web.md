# Web

Build single-page applications that run in the browser with Dioxus. To run on the Web, your app must be compiled to WebAssembly and depend on the `dioxus` crate with the `web` feature enabled.

A build of Dioxus for the web will be roughly equivalent to the size of a React build (70kb vs 65kb), but will load significantly faster due to [WebAssembly's StreamingCompile](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/) option.

Examples:
- [TodoMVC](https://github.com/DioxusLabs/example-projects/tree/master/todomvc)
- [ECommerce](https://github.com/DioxusLabs/example-projects/tree/master/ecommerce-site)

[![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc)

> Note: Because of the limitations of Wasm, not every crate will work with your web apps, so you'll need to make sure that your crates work without native system calls (timers, IO, etc).

## Support

The Web is the best-supported target platform for Dioxus.

## Tooling

To develop your Dioxus app for the web, you'll need a tool to build and serve your assets. We recommend using [trunk](https://trunkrs.dev) which includes a build system, Wasm optimization, a dev server, and support for SASS/CSS:

```shell
cargo install trunk
```

Make sure the `wasm32-unknown-unknown` target is installed:
```shell
rustup target add wasm32-unknown-unknown
```

## Creating a Project

Create a new crate:

```shell
cargo new --bin demo
cd demo
```

Add Dioxus as a dependency with the `web` feature:

```bash
cargo add dioxus --features web
```

Add an `index.html` for Trunk to use. Make sure your "mount point" element has an ID of "main":

```html
<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
  </head>
  <body>
    <div id="main"> </div>
  </body>
</html>
```

Edit your `main.rs`:
```rust
{{#include ../../../examples/hello_world_web.rs}}
```


And to serve our app:

```bash
trunk serve
```
