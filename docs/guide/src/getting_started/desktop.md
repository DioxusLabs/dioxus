# Desktop Application

Build a standalone native desktop app that looks and feels the same across operating systems.

Apps built with Dioxus are typically <5mb in size and use existing system resources, so they won't hog extreme amounts of RAM or memory.

Examples:
- [File explorer](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer)
- [WiFi scanner](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner)

[![File ExplorerExample](https://raw.githubusercontent.com/DioxusLabs/example-projects/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/tree/master/file-explorer)

## Support

The desktop is a powerful target for Dioxus, but is currently limited in capability when compared to the Web platform. Currently, desktop apps are rendered with the platform's WebView library, but your Rust code is running natively on a native thread. This means that browser APIs are *not* available, so rendering WebGL, Canvas, etc is not as easy as the Web. However, native system APIs *are* accessible, so streaming, WebSockets, filesystem, etc are all viable APIs. In the future, we plan to move to a custom webrenderer-based DOM renderer with WGPU integrations.

Dioxus Desktop is built off [Tauri](https://tauri.app/). Right now there aren't any Dioxus abstractions over keyboard shortcuts, menubar, handling, etc, so you'll want to leverage Tauri – mostly [Wry](http://github.com/tauri-apps/wry/) and [Tao](http://github.com/tauri-apps/tao)) directly.

## Creating a Project

Create a new crate:

```shell
cargo new --bin demo
cd demo
```

Add Dioxus with the `desktop` feature (this will edit `Cargo.toml`):

```shell
cargo add dioxus --features desktop
```

> If your system does not provide the `libappindicator3` library, like Debian/bullseye, you can enable the replacement `ayatana` with an additional flag:
>
>```shell
># On Debian/bullseye use:
>cargo add dioxus --features desktop --features ayatana
>```

Edit your `main.rs`:

```rust
{{#include ../../examples/hello_world_desktop.rs:all}}
```
