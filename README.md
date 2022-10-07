<p align="center">
  <img src="./notes/header.svg">
</p>

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
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>

  <!--Awesome -->
  <a href="https://github.com/dioxuslabs/awesome-dioxus">
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
    <a href="https://github.com/DioxusLabs/example-projects"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/guide"> Guide </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/notes/README/ZH_CN.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/master/translations/pt-br/README.md"> PT-BR </a>
  </h3>
</div>

<br/>

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user interfaces in Rust.

```rust
fn app(cx: Scope) -> Element {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
```

Dioxus can be used to deliver webapps, desktop apps, static sites, mobile apps, TUI apps, liveview apps, and more. Dioxus is entirely renderer agnostic and can be used as platform for any renderer.

If you know React, then you already know Dioxus.

## Unique features:
---
- Desktop apps running natively (no Electron!) in less than 10 lines of code.
- Incredibly ergonomic and powerful state management.
- Comprehensive inline documentation - hover and guides for all HTML elements, listeners, and events.
- Blazingly fast 🔥🔥 and extremely memory efficient
- Integrated hot reloading for fast iteration
- First-class async support with coroutines and suspense
- And more! Read the [full release post](https://dioxuslabs.com/blog/introducing-dioxus/).

## Supported Platforms
---
<table style="width:100%">
  <tr>
    <td><h2>Web</h2></td>
    <td>
      <ul>
        <li>Render directly to the DOM using WebAssembly</li>
        <li>Pre-render with SSR and rehydrate on the client</li>
        <li>Simple "hello world" at about 65kb, comparable to React</li>
        <li>Built-in dev server and hot reloading for quick iteration</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><h2>Desktop</h2></td>
    <td>
      <ul>
        <li>Render using Webview or - experimentally - with WGPU or Skia </li>
        <li>Zero-config setup. Simply cargo-run to build your app </li>
        <li>Full support for native system access without electron-esque IPC </li>
        <li>Supports macOS, Linux, and Windows. Portable <3mb binaries </li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><h2>Mobile</h2></td>
    <td>
      <ul>
        <li>Render using Webview or - experimentally - with WGPU or Skia </li>
        <li>Support for iOS and Android </li>
        <li><em>Significantly</em> more performant than React Native </li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><h2>Liveview</h2></td>
    <td>
      <ul>
        <li>Render apps - or just a single component - entirely on the server</li>
        <li>Integrations with popular Rust frameworks like Axum and Warp</li>
        <li>Extremely low-latency and ability to support 10,000+ simultaneous apps</li>
      </ul>
    </td>
  </tr>
  <tr>
    <td><h2>Terminal</h2></td>
    <td>
      <ul>
        <li>Render apps directly into your terminal, similar to <a href="https://github.com/vadimdemedes/ink"> ink.js</a></li>
        <li>Powered by the familiar flexbox and CSS model of the browser</li>
        <li>Built-in widgets like text input, buttons, and focus system</li>
      </ul>
    </td>
  </tr>
</table>

## Why Dioxus?
---
There's tons of options for building apps, so why would you choose Dioxus?

Well, first and foremost, Dioxus prioritizes developer experience. This is reflected in a variety of features unique to Dioxus:

- Autoformatting of our meta language (RSX) and accompanying VSCode extension
- Hotreloading using an interpreter of RSX for both desktop and web
- Emphasis on good docs - our guide is complete and our HTML elements are documented
- Significant research in simplifying

Dioxus is also a very extensible platform.

- Easily build new renderers by implementing a very simple optimized stack-machine
- Build and share components and even custom elements

So... Dioxus is great, but why won't it work for me?
- It's not fully mature yet. APIs are still shifting, things might break (though we try to avoid it)
- You need to run in a no-std environment.
- You don't like the React-hooks model of building UIs


## Contributing
---
- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## License
---
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
