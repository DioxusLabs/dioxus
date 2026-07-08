Dioxus Desktop Renderer

Render the Dioxus VirtualDom using the platform's native WebView implementation.

# Desktop

One of Dioxus' flagship features is the ability to quickly build a native desktop app that looks and feels the same across platforms. Apps built with Dioxus are typically <5mb in size and use existing system resources, so they won't hog extreme amounts of RAM or memory.

Dioxus Desktop is built directly on [Wry](http://github.com/tauri-apps/wry/) and [Tao](http://github.com/tauri-apps/tao). Dioxus APIs cover common desktop features like window access, custom asset handlers, global shortcuts, menus, tray icons, and main-thread callbacks. The underlying Wry/Tao types are re-exported for lower-level control.

## Getting Set up

Getting Set up with Dioxus-Desktop is quite easy. Make sure you have Rust and Cargo installed, and then create a new project:

```shell
$ cargo new --bin demo
$ cd app
```

Add Dioxus with the `desktop` renderer feature:

```shell
$ cargo add dioxus --features desktop
```

Edit your `main.rs`:

```rust, ignore
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            "hello world!"
        }
    }
}
```

To configure the webview, menubar, tray icon, protocols, and other desktop-specific features, check out the launch configuration in the [API reference](https://docs.rs/dioxus-desktop/).

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/learn/0.7/) if you already haven't!
