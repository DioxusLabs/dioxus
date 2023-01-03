# Web

Build single-page applications that run in the browser with Dioxus. To run on the Web, your app must be compiled to WebAssembly and depend on the `dioxus` and `dioxus-web` crates.

A build of Dioxus for the web will be roughly equivalent to the size of a React build (70kb vs 65kb) but it will load significantly faster because [WebAssembly can be compiled as it is streamed](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/).

Examples:
- [TodoMVC](https://github.com/DioxusLabs/example-projects/tree/master/todomvc)
- [ECommerce](https://github.com/DioxusLabs/example-projects/tree/master/ecommerce-site)

[![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc)

> Note: Because of the limitations of Wasm, [not every crate will work](https://rustwasm.github.io/docs/book/reference/which-crates-work-with-wasm.html) with your web apps, so you'll need to make sure that your crates work without native system calls (timers, IO, etc).

## Support

The Web is the best-supported target platform for Dioxus.

- Because your app will be compiled to WASM you have access to browser APIs through [wasm-bingen](https://rustwasm.github.io/docs/wasm-bindgen/introduction.html).
- Dioxus provides hydration to resume apps that are rendered on the server. See the [hydration example](https://github.com/DioxusLabs/dioxus/blob/master/packages/web/examples/hydrate.rs) for more details.

## Tooling

To develop your Dioxus app for the web, you'll need a tool to build and serve your assets. We recommend using [dioxus-cli](https://github.com/DioxusLabs/cli) which includes a build system, Wasm optimization, a dev server, and support hot reloading:

```shell
cargo install dioxus-cli
```

Make sure the `wasm32-unknown-unknown` target for rust is installed:
```shell
rustup target add wasm32-unknown-unknown
```

## Creating a Project

Create a new crate:

```shell
cargo new --bin demo
cd demo
```

Add Dioxus and the web renderer as dependencies (this will edit your `Cargo.toml`):

```bash
cargo add dioxus
cargo add dioxus-web
```

Edit your `main.rs`:
```rust
{{#include ../../../examples/hello_world_web.rs}}
```


And to serve our app:

```bash
dioxus serve
```
