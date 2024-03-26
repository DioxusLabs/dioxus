<p>
  <a href="https://dioxuslabs.com">
    <p align="center" >
      <img src="./notes/header-light.svg" >
      <img src="./notes/dioxus_splash_6.avif">
    </p>
  </a>
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
    <a href="https://github.com/DioxusLabs/example-projects"> Examples </a>
    <span> | </span>
    <a href="https://dioxuslabs.com/learn/0.4/guide"> Guide </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/zh-cn/README.md"> 中文 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/pt-br/README.md"> PT-BR </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/ja-jp/README.md"> 日本語 </a>
    <span> | </span>
    <a href="https://github.com/DioxusLabs/dioxus/blob/main/translations/tr-tr"> Türkçe </a>
  </h3>
</div>
<br>


Build for web, desktop, and mobile, and more with a single codebase. Zero-config setup, integrated hotreloading, and signals-based state management help you ship faster and more reliably. Seamlessly add backend functionality with Server Functions and bundle with our CLI.

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

<br>

## ⭐️ Unique features:
- Cross-platform apps in three lines of code (web, desktop, mobile, server, and more)
- [Ergonomic state management](https://dioxuslabs.com/blog/release-050) combines the best of React, Solid, and Svelte
- Extremely performant, powered by Rust's fastest wasm-framework [sledgehammer](https://dioxuslabs.com/blog/templates-diffing)
- Integrated bundler for deploying to the web, macOS, Linux, and Windows
- And more! Read the [take a tour of Dioxus](https://dioxuslabs.com/learn/0.5/).

## ⚙️ Integrated hot-reloading:
With one command, `dx serve` and your app is running. Edit your markup and see the results in real time.

<div align="center">
  <img src="./notes/hotreload.gif">
</div>


## ⚒️ Supported Platforms
<div align="center">
  <table style="width:100%">
    <tr>
      <td>
      <b>Web</b>
      <br />
      <em>Tier 1 Support</em>
      </td>
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
      <td>
      <b>Desktop</b>
      <br />
      <em>Tier 1 Support</em>
      </td>
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
      <td>
      <b>Liveview</b>
      <br />
      <em>Tier 1 Support</em>
      </td>
      <td>
        <ul>
          <li>Render apps - or just a single component - entirely on the server</li>
          <li>Integrations with popular Rust frameworks like Axum and Warp</li>
          <li>Extremely low-latency and ability to support 10,000+ simultaneous apps</li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>Mobile</b>
      <br />
      <em>Tier 2 Support</em>
      </td>
      <td>
        <ul>
          <li>Render using Webview or - experimentally - with WGPU or Skia </li>
          <li>Support for iOS and Android </li>
          <li><em>Significantly</em> more performant than React Native </li>
        </ul>
      </td>
    </tr>
    <tr>
      <td>
      <b>Terminal</b>
      <br />
      <em>Tier 2 Support</em>
      </td>
      <td>
        <ul>
          <li>Render apps directly into your terminal, similar to <a href="https://github.com/vadimdemedes/ink"> ink.js</a></li>
          <li>Powered by the familiar flexbox and CSS model of the browser</li>
          <li>Built-in widgets like text input, buttons, and focus system</li>
        </ul>
      </td>
    </tr>
  </table>
</div>

## Dioxus vs other frameworks

todo

### Dioxus vs Leptos
todo

### Dioxus vs Iced
todo

## Why Dioxus?
There's tons of options for building apps, so why would you choose Dioxus?

Well, first and foremost, Dioxus prioritizes developer experience. This is reflected in a variety of features unique to Dioxus:

- Autoformatting of our meta language (RSX) and accompanying [VSCode extension](https://marketplace.visualstudio.com/items?itemName=DioxusLabs.dioxus)
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
- Check out the website [section on contributing](https://dioxuslabs.com/learn/0.4/contributing).
- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- [Join](https://discord.gg/XgGxMSkvUM) the discord and ask questions!


<a href="https://github.com/dioxuslabs/dioxus/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=dioxuslabs/dioxus&max=30&columns=10" />
</a>

## License
This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
