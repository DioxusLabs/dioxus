<div align="center">
  <h1>Rink</h1>
  <p>
    <strong>A beautiful terminal user interfaces library in Rust.</strong>
  </p>
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/rink">
    <img src="https://img.shields.io/crates/v/rink.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/rink">
    <img src="https://img.shields.io/crates/d/rink.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/rink">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/rink/actions">
    <img src="https://github.com/dioxuslabs/rink/actions/workflows/main.yml/badge.svg"
      alt="CI status" />
  </a>
  <!-- Discord -->
  <a href="https://discord.gg/XgGxMSkvUM">
    <img src="https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square" alt="Discord Link" />
  </a>
</div>

<br/>

Leverage CSS, HTML, and Rust to build beautiful, portable, terminal user interfaces. Rink is the cross-framework library that powers [`Dioxus-TUI`](https://github.com/DioxusLabs/dioxus/tree/master/packages/dioxus-tui)

![demo app](examples/example.png)

## Background

You can use Html-like semantics with inline styles, tree hierarchy, components, and more in your [`text-based user interface (TUI)`](https://en.wikipedia.org/wiki/Text-based_user_interface) application.

Rink is essentially a port of [Ink](https://github.com/vadimdemedes/ink) but for [`Rust`](https://www.rust-lang.org/). Rink doesn't depend on Node.js or any other JavaScript runtime, so your binaries are portable and beautiful.

## Limitations

- **Subset of Html**
  Terminals can only render a subset of HTML. We support as much as we can.
- **Particular frontend design**
  Terminals and browsers are and look different. Therefore, the same design might not be the best to cover both renderers.

## Status

**WARNING: Rink is currently under construction!**

Rendering a Dom works fine, but the ecosystem of widgets is not ready yet. Additionally, some bugs in the flexbox implementation might be quirky at times.

## Features

Rink features:

- [x] Flexbox-based layout system
- [ ] CSS selectors
- [x] inline CSS support
- [x] Built-in focusing system

* [ ] Widgets
* [ ] Support for events, hooks, and callbacks<sup>1</sup>
* [ ] Html tags<sup>2</sup>

<sup>1</sup> Basic keyboard, mouse, and focus events are implemented.
<sup>2</sup> Currently, most HTML tags don't translate into any meaning inside of Rink. So an `input` _element_ won't mean anything nor does it have any additional functionality.
