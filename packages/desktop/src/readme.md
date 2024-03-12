Dioxus Desktop Renderer

Render the Dioxus VirtualDom using the platform's native WebView implementation.

# Desktop

One of Dioxus' flagship features is the ability to quickly build a native desktop app that looks and feels the same across platforms. Apps built with Dioxus are typically <5mb in size and use existing system resources, so they won't hog extreme amounts of RAM or memory.

Dioxus Desktop is built off Tauri. Right now there aren't any Dioxus abstractions over the menubar, handling, etc, so you'll want to leverage Tauri - mostly [Wry](http://github.com/tauri-apps/wry/) and [Tao](http://github.com/tauri-apps/tao) directly. An upcoming release of Dioxus-Desktop will include components and hooks for notifications, global shortcuts, menubar, etc.

## Getting Set up

Getting Set up with Dioxus-Desktop is quite easy. Make sure you have Rust and Cargo installed, and then create a new project:

```shell
$ cargo new --bin demo
$ cd app
```

Add Dioxus and the `desktop` renderer feature:

```shell
$ cargo add dioxus
$ cargo add dioxus-desktop
```

Edit your `main.rs`:

```rust, ignore
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    rsx!{
        div {
            "hello world!"
        }
    })
}
```

To configure the webview, menubar, and other important desktop-specific features, checkout out some of the launch configuration in the [API reference](https://docs.rs/dioxus-desktop/).

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/learn/0.4/) if you already haven't!
