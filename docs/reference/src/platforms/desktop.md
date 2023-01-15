# Getting Started: Desktop

One of Dioxus' killer features is the ability to quickly build a native desktop app that looks and feels the same across platforms. Apps built with Dioxus are typically <5mb in size and use existing system resources, so they won't hog extreme amounts of RAM or memory.

Dioxus Desktop is built off Tauri. Right now there aren't any Dioxus abstractions over keyboard shortcuts, menubar, handling, etc, so you'll want to leverage Tauri - mostly [Wry](http://github.com/tauri-apps/wry/) and [Tao](http://github.com/tauri-apps/tao)) directly. The next major release of Dioxus-Desktop will include components and hooks for notifications, global shortcuts, menubar, etc.

## Getting Set up

Getting Set up with Dioxus-Desktop is quite easy. Make sure you have Rust and Cargo installed, and then create a new project:

```shell
$ cargo new --bin demo
$ cd demo
```

Add Dioxus with the `desktop` feature:

```shell
$ cargo add dioxus --features desktop
```

Edit your `main.rs`:

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```

To configure the webview, menubar, and other important desktop-specific features, checkout out some of the launch configuration in the [API reference](https://docs.rs/dioxus-desktop/).

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide/en) if you already haven't!
