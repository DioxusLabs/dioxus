<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/splash-header-darkmode.svg#gh-dark-mode-only" style="width: 80%; height: auto;">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/splash-header.svg#gh-light-mode-only" style="width: 80%; height: auto;">
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/v/dioxus.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus">
    <img src="https://img.shields.io/crates/d/dioxus.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/dioxuslabs/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>
  <!-- Awesome -->
  <a href="https://dioxuslabs.com/awesome">
    <img src="https://cdn.rawgit.com/sindresorhus/awesome/d7305f38d29fed78fa85652e3a63e154dd8e8829/media/badge.svg" alt="Awesome Page" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://dioxuslabs.com"> Website </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.7/getting_started"> Getting Started </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.7/"> Book (0.7) </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/tree/main/examples"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.7/tutorial"> Tutorial </a>
  </h3>
</div>

---

Build for web, desktop, and mobile, and more with a single codebase. Zero-config setup, integrated hot-reloading, and signals-based state management. Add backend functionality with Server Functions and bundle with our CLI.

```rust
fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    }
}
```

## ⭐️ Unique features

- Cross-platform apps in three lines of code (web, desktop, mobile, server, and more)
- [Ergonomic state management](https://dioxuslabs.com/blog/release-050) combines the best of React, Solid, and Svelte
- Built-in featureful, type-safe, fullstack web framework
- Integrated bundler for deploying to the web, macOS, Linux, and Windows
- Subsecond Rust hot-patching and asset hot-reloading
- And more! [Take a tour of Dioxus](https://dioxuslabs.com/learn/0.7/).

## Quick start

Grab the Dioxus CLI (`dx`) and spin up a new app in seconds:

```shell
# install the dioxus-cli
curl -fsSL https://dioxuslabs.com/install.sh | bash

# create a new app, following the template instructions
dx new my-app && cd my-app

# and then serve on `--web, --desktop, --ios, or --android`
dx serve --desktop
```

## Guides

This overview doesn't cover everything. Make sure to check out:

- [Long-form tutorial](https://dioxuslabs.com/learn/0.7/tutorial)
- [Core concept guide](https://dioxuslabs.com/learn/0.7/essentials/)
- [Platform-specific guides](https://dioxuslabs.com/learn/0.7/guides/)
- [Code examples on GitHub](https://github.com/DioxusLabs/dioxus/tree/main/examples)


## Instant hot-reloading

With one command, `dx serve` and your app is running. Edit your markup, styles, and see changes in milliseconds. Use our experimental `dx serve --hotpatch` to update Rust code in real time.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/screenshots/refs/heads/main/blitz/hotreload-video.webp">
</div>

## Build Beautiful Apps

Dioxus apps are styled with HTML and CSS. Use the built-in TailwindCSS support or load your favorite CSS library. Easily call into native code (objective-c, JNI, Web-Sys) for a perfect native touch.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/ebou2.avif">
</div>

## Truly fullstack applications

Dioxus deeply integrates with [axum](https://github.com/tokio-rs/axum) to provide powerful fullstack capabilities for both clients and servers. Pick from a wide array of built-in batteries like WebSockets, SSE, Streaming, File Upload/Download, Server-Side-Rendering, Forms, Middleware, and Hot-Reload, or go fully custom and integrate your existing axum backend.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/fullstack-websockets.avif" width="700">
</div>

## Experimental Native Renderer

Render using web-sys, webview, server-side-rendering, liveview, or even with our experimental WGPU-based renderer. Embed Dioxus in Bevy, WGPU, or even run on embedded Linux!

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/screenshots/refs/heads/main/blitz/native-blitz-wgpu.webp">
</div>

## First-party primitive components

Get started quickly with a complete set of primitives modeled after shadcn/ui and Radix-Primitives.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/primitive-components.avif" width="700">
</div>

## First-class Android and iOS support

Dioxus is the fastest way to build native mobile apps with Rust. Simply run `dx serve --platform android` and your app is running in an emulator or on device in seconds. Call directly into JNI and Native APIs.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/android_and_ios2.avif" width="500">
</div>

## Bundle for web, desktop, and mobile

Simply run `dx bundle` and your app will be built and bundled with maximization optimizations. On the web, take advantage of [`.avif` generation, `.wasm` compression, minification](https://dioxuslabs.com/learn/0.7/tutorial/assets), and more. Build WebApps weighing [less than 50kb](https://github.com/ealmloff/tiny-dioxus/) and desktop/mobile apps less than 5mb.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/bundle.gif">
</div>

## Fantastic documentation

We've put a ton of effort into building clean, readable, and comprehensive documentation. All html elements and listeners are documented with MDN docs, and our Docs runs continuous integration with Dioxus itself to ensure that the docs are always up to date. Check out the [Dioxus website](https://dioxuslabs.com/learn/0.7/) for guides, references, recipes, and more.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/docs.avif">
</div>

## Community

Dioxus is a community-driven project, with a very active [Discord](https://discord.gg/XgGxMSkvUM) and [GitHub](https://github.com/DioxusLabs/dioxus/issues) community. We're always looking for help, and we're happy to answer questions and help you get started. [Our SDK](https://github.com/DioxusLabs/dioxus-std) is community-run and we even have a [GitHub organization](https://github.com/dioxus-community/) for the best Dioxus crates that receive free upgrades and support.

<div align="center">
  <img src="https://raw.githubusercontent.com/DioxusLabs/dioxus/main/notes/dioxus-community.avif">
</div>

## License

This project is licensed under either the [MIT license](https://github.com/DioxusLabs/dioxus/blob/main/LICENSE-MIT) or the [Apache-2 License](https://github.com/DioxusLabs/dioxus/blob/main/LICENSE-APACHE).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Dioxus by you, shall be licensed as MIT or Apache-2, without any additional terms or conditions.
