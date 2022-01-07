# Getting Started: Dioxus for Web

[*"Pack your things, we're going on an adventure!"*](https://trunkrs.dev)

Dioxus is well supported for the web through WebAssembly. A build of Dioxus for the web will be roughly equivalent to the size of a React build (70kb vs 65kb), but will load significantly faster due to [WebAssembly's StreamingCompile](https://hacks.mozilla.org/2018/01/making-webassembly-even-faster-firefoxs-new-streaming-and-tiering-compiler/) option.

Building Dioxus apps for the web requires much less configuration than our JavaScript counterparts.

## Tooling

To develop your Dioxus app for the web, you'll need a tool to build and serve your assets. We recommend using [trunk](https://trunkrs.dev) which includes a build system, Wasm optimization, a dev server, and support for SASS/CSS.

Currently, trunk projects can only build the root binary (ie the `main.rs`). To build a new Dioxus compatible project, this should get you up and running.

First, [install trunk](https://trunkrs.dev/#install):
```shell
$ cargo install trunk
```

Make sure the `wasm32-unknown-unknown` target is installed:
```shell
$ rustup target add wasm32-unknown-unknown
```

Create a new project:

```shell
$ cargo new --bin demo
$ cd demo
```

Add Dioxus with the `web` features:

```
$ cargo add dioxus --features web
```

Add an `index.html` for Trunk to use. Make sure your "mount point" element has an ID of "main" (this can be configured later):

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
// main.rs

use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div { "hello, wasm!" }
    })
}
```


And to serve our app:

```shell
trunk serve
```

To build our app and publish it to Github:

- Make sure Github Pages is set up for your repo
- Build your app with `trunk build --release`
- Move your generated HTML/CSS/JS/Wasm from `dist` into the folder configured for Github Pages
- Add and commit with git
- Push to Github

That's it!

## Future Build Tool

We are currently working on our own build tool called [Dioxus Studio](http://github.com/dioxusLabs/studio) which will support:
- an interactive TUI
- on-the-fly reconfiguration
- hot CSS reloading
- two-way data binding between browser and source code
- an interpreter for `rsx!` 
- ability to publish to github/netlify/vercel
- bundling for iOS/Desktop/etc

## Features

Currently, the web supports:

- Pre-rendering/Hydration

## Events

The regular events in the `html` crate work just fine in the web.


## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!
