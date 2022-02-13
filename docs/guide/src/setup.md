# Overview

In this chapter, we're going to get set up with a small desktop application.

We'll learn about:
- Installing the Rust programming language
- Installing the Dioxus CLI for bundling and developing
- Suggested cargo extensions


For platform-specific guides, check out the [Platform Specific Guides](../platforms/00-index.md).

# Setting up Dioxus

Dioxus requires a few main things to get up and running:

- The [Rust compiler](https://www.rust-lang.org) and associated build tooling
- An editor of your choice, ideally configured with the [Rust-Analyzer LSP plugin](https://rust-analyzer.github.io)

Dioxus integrates very well with the Rust-Analyzer IDE plugin which will provide appropriate syntax highlighting, code navigation, folding, and more.

## Installing Rust

Head over to [https://rust-lang.org](http://rust-lang.org) and install the Rust compiler.

Once installed, make sure to install wasm32-unknown-unknown as a target if you're planning on deploying your app to the web.

```
rustup target add wasm32-unknown-unknown
```

## Platform-Specific Dependencies

If you are running a modern, mainstream operating system, you should need no additional setup to build WebView-based Desktop apps. However, if you are running an older version of Windows or a flavor of Linux with no default web rendering engine, you might need to install some additional dependencies.


### Windows

Windows Desktop apps depend on WebView2 - a library which should be installed in all modern Windows distributions. If you have Edge installed, then Dioxus will work fine. If you *don't* have Webview2, [then you can install it through Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). MS provides 3 options:

1. A tiny "evergreen" *bootstrapper* which will fetch an installer from Microsoft's CDN
2. A tiny *installer* which will fetch Webview2 from Microsoft's CDN
3. A statically linked version of Webview2 in your final binary for offline users

For development purposes, use Option 1. 

### Linux

Webview Linux apps require WebkitGtk. When distributing, this can be part of your dependency tree in your `.rpm` or `.deb`. However, it's very likely that your users will already have WebkitGtk.

```
sudo apt install libwebkit2gtk-4.0-dev libgtk-3-dev libappindicator3-dev
```


If you run into issues, make sure you have all the basics installed, as outlined in the [Tauri docs](https://tauri.studio/en/docs/get-started/setup-linux).


### macOS

Currently - everything for macOS is built right in! However, you might run into an issue if you're using nightly Rust due to some permissions issues in our Tao dependency (which have been resolved but not published).


## Dioxus-CLI for dev server, bundling, etc.

We also recommend installing the Dioxus CLI. The Dioxus CLI automates building and packaging for various targets and integrates with simulators, development servers, and app deployment. To install the CLI, you'll need cargo (which should be automatically installed with Rust):

```
$ cargo install dioxus-cli
```

You can update dioxus-cli at any time with:

```
$ cargo install --force dioxus-cli
```

We provide this 1st-party tool to save you from having to run potentially untrusted code every time you add a crate to your project - as is standard in the NPM ecosystem.

## Suggested extensions

If you want to keep your traditional `npm install XXX` workflow for adding packages, you might want to install `cargo-edit` and a few other fun `cargo` extensions:

- [cargo edit](https://github.com/killercup/cargo-edit) for adding dependencies from the CLI
- [cargo-expand](https://github.com/dtolnay/cargo-expand) for expanding macro calls
- [cargo tree](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) - an integrated cargo command that lets you inspect your dependency tree

That's it! We won't need to touch NPM/WebPack/Babel/Parcel, etc. However, you _can_ configure your app to use WebPack with [traditional WASM-pack tooling](https://rustwasm.github.io/wasm-pack/book/tutorials/hybrid-applications-with-webpack/using-your-library.html).

## Rust Knowledge

With Rust, things like benchmarking, testing, and documentation are included in the language. We strongly recommend going through the official Rust book _completely_. However, our hope is that a Dioxus app can serve as a great first Rust project. With Dioxus you'll learn about:

- Error handling
- Structs, Functions, Enums
- Closures
- Macros

We've put a lot of care into making Dioxus syntax familiar and easy to understand, so you won't need deep knowledge on async, lifetimes, or smart pointers until you really start building complex Dioxus apps.

We strongly encourage exploring the guides for more information on how to work with the integrated tooling:

- [Testing](Testing.md)
- [Documentation](Documentation.md)
- [Benchmarking](Benchmarking.md)
- [Building](Building.md)
- [Modules](Modules.md)
- [Crates](Crates.md)
