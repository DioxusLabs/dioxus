<div align="center">
  <h1>Dioxus</h1>
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

### Unique features:
- Desktop apps running natively (no Electron!) in less than 10 lines of code.
- Incredibly ergonomic and powerful state management.
- Comprehensive inline documentation - hover and guides for all HTML elements, listeners, and events.
- Extremely memory efficient - 0 global allocations for steady-state components.
- Multi-channel asynchronous scheduler for first-class async support.
- And more! Read the [full release post](https://dioxuslabs.com/blog/introducing-dioxus/).

## Get Started with...

<table style="width:100%" align="center">
    <tr >
        <th><a href="https://dioxuslabs.com/guide/">Tutorial</a></th>
        <th><a href="https://dioxuslabs.com/reference/web">Web</a></th>
        <th><a href="https://dioxuslabs.com/reference/desktop/">Desktop</a></th>
        <th><a href="https://dioxuslabs.com/reference/ssr/">SSR</a></th>
        <th><a href="https://dioxuslabs.com/reference/mobile/">Mobile</a></th>
        <th><a href="https://dioxuslabs.com/guide/concepts/managing_state.html">State</a></th>
    <tr>
</table>


## Example Projects:

| File Navigator (Desktop)                                                                                                                                                        | WiFi scanner (Desktop)                                                                                                                                                                 | TodoMVC (All platforms)                                                                                                                                                 | E-commerce w/ Tailwind (SSR/LiveView)                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [![File Explorer](https://github.com/DioxusLabs/example-projects/raw/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/blob/master/file-explorer) | [![Wifi Scanner Demo](https://github.com/DioxusLabs/example-projects/raw/master/wifi-scanner/demo_small.png)](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner) | [![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc) | [![E-commerce Example](https://github.com/DioxusLabs/example-projects/raw/master/ecommerce-site/demo.png)](https://github.com/DioxusLabs/example-projects/blob/master/ecommerce-site) |


See the [awesome-dioxus](https://github.com/DioxusLabs/awesome-dioxus) page for a curated list of content in the Dioxus Ecosystem.

## Why Dioxus and why Rust?

TypeScript is a fantastic addition to JavaScript, but it's still fundamentally JavaScript. TS code runs slightly slower, has tons of configuration options, and not every package is properly typed.

By using Rust, we gain:

- Static types for *every* library
- Immutability by default
- A simple and intuitive module system
- Integrated documentation (`go to source` _actually goes to source_)
- Advanced pattern matching
- Clean, efficient, composable iterators
- Inline built-in unit/integration testing
- Best-in-class error handling
- Powerful and sane standard library
- Flexible macro system
- Access to `crates.io`

Specifically, Dioxus provides us many other assurances:

- Proper use of immutable data structures
- Guaranteed error handling (so you can sleep easy at night not worrying about `cannot read property of undefined`)
- Native performance on mobile
- Direct access to system IO

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time.

## Why NOT Dioxus?
You shouldn't use Dioxus if:

- You don't like the React Hooks approach to frontend
- You need a no-std renderer
- You want to support browsers where Wasm or asm.js are not supported.
- You need a Send+Sync UI solution (Dioxus is not currently thread-safe)

## Comparison with other Rust UI frameworks
Dioxus primarily emphasizes **developer experience** and **familiarity with React principles**.

- [Yew](https://github.com/yewstack/yew): prefers the elm pattern instead, no borrowed props, supports SSR (no hydration), no direct desktop/mobile support.
- [Percy](https://github.com/chinedufn/percy): Supports SSR but with less emphasis on state management and event handling.
- [Sycamore](https://github.com/sycamore-rs/sycamore): VDOM-less using fine-grained reactivity, but no direct support for desktop/mobile.
- [Dominator](https://github.com/Pauan/rust-dominator): Signal-based zero-cost alternative, less emphasis on community and docs.
- [Azul](https://azul.rs): Fully native HTML/CSS renderer for desktop applications, no support for web/ssr


## Parity with React & Roadmap

Dioxus is heavily inspired by React, but we want your transition to feel like an upgrade. Dioxus is _most_ of the way there, but missing a few key features. These include:

- Portals
- Suspense integration with SSR
- Server Components / Bundle Splitting / Lazy

Dioxus is unique in the Rust ecosystem in that it supports:

- Components with props that borrow from their parent
- Server-side-rendering with client-side hydration
- Support for desktop applications

For more information on what features are currently available and the roadmap for the future, be sure to check out [the guide](https://dioxuslabs.com/guide/).

## Projects in the ecosystem

Want to jump in and help build the future of Rust frontend? There's plenty of places where your contributions can make a huge difference:

- [TUI renderer](https://github.com/dioxusLabs/rink)
- [CLI Tooling](https://github.com/dioxusLabs/cli)
- [Documentation and Example Projects](https://github.com/dioxusLabs/docsite)
- LiveView and Web Server
- Asset System

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you, shall be licensed as MIT, without any additional
terms or conditions.
