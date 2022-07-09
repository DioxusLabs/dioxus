# Getting Started

This section will help you set up your Dioxus project!

## Prerequisites

### An Editor

Dioxus integrates very well with the [Rust-Analyzer LSP plugin](https://rust-analyzer.github.io) which will provide appropriate syntax highlighting, code navigation, folding, and more.

### Rust

Head over to [https://rust-lang.org](http://rust-lang.org) and install the Rust compiler.

We strongly recommend going through the [official Rust book](https://doc.rust-lang.org/book/ch01-00-getting-started.html) _completely_. However, our hope is that a Dioxus app can serve as a great first Rust project. With Dioxus, you'll learn about:

- Error handling
- Structs, Functions, Enums
- Closures
- Macros

We've put a lot of care into making Dioxus syntax familiar and easy to understand, so you won't need deep knowledge on async, lifetimes, or smart pointers until you really start building complex Dioxus apps.

### Platform-Specific Dependencies

#### Windows

Windows Desktop apps depend on WebView2 – a library which should be installed in all modern Windows distributions. If you have Edge installed, then Dioxus will work fine. If you *don't* have Webview2, [then you can install it through Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). MS provides 3 options:

1. A tiny "evergreen" *bootstrapper* which will fetch an installer from Microsoft's CDN
2. A tiny *installer* which will fetch Webview2 from Microsoft's CDN
3. A statically linked version of Webview2 in your final binary for offline users

For development purposes, use Option 1.

#### Linux

Webview Linux apps require WebkitGtk. When distributing, this can be part of your dependency tree in your `.rpm` or `.deb`. However, it's very likely that your users will already have WebkitGtk.

```bash
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev
```

When using Debian/bullseye `libappindicator3-dev` is no longer available but replaced by `libayatana-appindicator3-dev`.

```bash
# on Debian/bullseye use:
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libayatana-appindicator3-dev
```

If you run into issues, make sure you have all the basics installed, as outlined in the [Tauri docs](https://tauri.studio/v1/guides/getting-started/prerequisites#setting-up-linux).


#### MacOS

Currently – everything for macOS is built right in! However, you might run into an issue if you're using nightly Rust due to some permissions issues in our Tao dependency (which have been resolved but not published).

### Suggested Cargo Extensions

If you want to keep your traditional `npm install XXX` workflow for adding packages, you might want to install `cargo-edit` and a few other fun `cargo` extensions:

- [cargo-expand](https://github.com/dtolnay/cargo-expand) for expanding macro calls
- [cargo tree](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) – an integrated cargo command that lets you inspect your dependency tree


## Setup Guides

Dioxus supports multiple platforms. Depending on what you want, the setup is a bit different.

- [Web](web.md): running in the browser using WASM
- [Server Side Rendering](ssr.md): render Dioxus HTML as text
- [Desktop](desktop.md): a standalone app using webview
- [Mobile](mobile.md)
- [Terminal UI](tui.md): terminal text-based graphic interface