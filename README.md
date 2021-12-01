<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
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
  <!-- docs.rs docs -->
  <a href="https://docs.rs/dioxus">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
  <!-- CI -->
  <a href="https://github.com/jkelleyrtp/dioxus/actions">
    <img src="https://github.com/jkelleyrtp/dioxus/workflows/CI/badge.svg"
      alt="CI status" />
  </a>
</div>

<div align="center">
  <h3>
    <a href="https://docs.rs/dioxus">
      API Docs
    </a>
    <span> | </span>
    <a href="https://docs.rs/dioxus">
      Website
    </a>
    <span> | </span>
    <a href="https://docs.rs/dioxus">
      Examples
    </a>
  </h3>
</div>

<br/>

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.

```rust
fn App(cx: Scope, props: &()) -> Element {
    let mut count = use_state(cx, || 0);

    cx.render(rsx!(
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    ))
};
```

Dioxus can be used to deliver webapps, desktop apps, static sites, liveview apps, mobile apps (WIP), and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

If you know React, then you already know Dioxus.

### Unique features:
- Desktop apps running natively (no Electron!) in less than 10 lines of code.
- Incredibly ergonomic and powerful state management.
- Incredible inline documentation - hover and guides for all HTML elements, listeners, and events.
- Extremely memory efficient - 0 global allocations for steady-state components.
- Multithreaded asynchronous coroutine scheduler for first-class async support.
- And more! Read the full release post here.

## Get Started with...

<table style="width:100%" align="center">
    <tr >
        <th><a href="http://github.com/jkelleyrtp/dioxus">Web</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Desktop</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Mobile</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">State</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Docs</a></th>
        <th><a href="http://github.com/jkelleyrtp/dioxus">Tools</a></th>
    <tr>
</table>

## Examples:

| File Navigator (Desktop)                                                                                                                | Bluetooth scanner (Desktop)                                      | TodoMVC (All platforms)                                                                                         | Widget Gallery                                                   |
| --------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| [![asd](https://github.com/DioxusLabs/file-explorer-example/raw/master/image.png)](https://github.com/DioxusLabs/file-explorer-example) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | [![asd](https://github.com/DioxusLabs/todomvc/blob/master/example.png)](https://github.com/dioxusLabs/todomvc/) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) |


See the awesome-dioxus page for a curated list of content in the Dioxus Ecosystem.

## Why Dioxus and why Rust?

TypeScript is a fantastic addition to JavaScript, but it's still fundamentally JavaScript. TS code runs slightly slower, has tons of configuration options, and not every package is properly typed. 

In contrast, Dioxus is written in Rust - which is almost like "TypeScript on steroids". 

By using Rust, we gain:

- Static types for *every* library
- Immutability by default
- A simple and intuitive module system
- Integrated documentation (`go to source` _actually goes to source_)
- Advanced pattern matching
- Clean, efficient, composable iterators
- Inline built-in unit/integration testing
- Best-in-class error handling
- Powerful and sane, standard library
- Flexible macro system
- Access to `crates.io`

Specifically, Dioxus provides us many other assurances:

- Proper use of immutable datastructures
- Guaranteed error handling (so you can sleep easy at night not worrying about `cannot read property of undefined`) 
- Native performance on mobile
- Direct access to system IO

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time. 

### Why NOT Dioxus?
You shouldn't use Dioxus if:
- You don't like the React Hooks approach to frontend
- You need a no-std renderer
- You want to support browsers where Wasm or asm.js are not supported.

# Parity with React

Dioxus is heavily inspired by React, but we want your transition to feel like an upgrade. Dioxus is _most_ of the way there, but missing a few key features. This parity table does not necessarily include important ecosystem crates like code blocks, markdown, resizing hooks, etc.


| Feature                   | Dioxus | React | Notes for Dioxus                                                     |
| ------------------------- | ------ | ----- | -------------------------------------------------------------------- |
| Conditional Rendering     | ✅      | ✅     | if/then to hide/show component                                       |
| Map, Iterator             | ✅      | ✅     | map/filter/reduce to produce rsx!                                    |
| Keyed Components          | ✅      | ✅     | advanced diffing with keys                                           |
| Web                       | ✅      | ✅     | renderer for web browser                                             |
| Desktop (webview)         | ✅      | ✅     | renderer for desktop                                                 |
| Shared State (Context)    | ✅      | ✅     | share state through the tree                                         |
| Hooks                     | ✅      | ✅     | memory cells in components                                           |
| SSR                       | ✅      | ✅     | render directly to string                                            |
| Component Children        | ✅      | ✅     | cx.children() as a list of nodes                                     |
| Headless components       | ✅      | ✅     | components that don't return real elements                           |
| Fragments                 | ✅      | ✅     | multiple elements without a real root                                |
| Manual Props              | ✅      | ✅     | Manually pass in props with spread syntax                            |
| Controlled Inputs         | ✅      | ✅     | stateful wrappers around inputs                                      |
| CSS/Inline Styles         | ✅      | ✅     | syntax for inline styles/attribute groups                            |
| Custom elements           | ✅      | ✅     | Define new element primitives                                        |
| Suspense                  | ✅      | ✅     | schedule future render from future/promise                           |
| Integrated error handling | ✅      | ✅     | Gracefully handle errors with ? syntax                               |
| NodeRef                   | ✅      | ✅     | gain direct access to nodes                                          |
| Re-hydration              | ✅      | ✅     | Pre-render to HTML to speed up first contentful paint                |
| Jank-Free Rendering       | ✅      | ✅     | Large diffs are segmented across frames for silky-smooth transitions |
| Effects                   | ✅      | ✅     | Run effects after a component has been committed to render           |
| Cooperative Scheduling    | 🛠      | ✅     | Prioritize important events over non-important events                |
| Server Components         | 🛠      | ✅     | Hybrid components for SPA and Server                                 |
| Bundle Splitting          | 👀      | ✅     | Efficiently and asynchronously load the app                          |
| Lazy Components           | 👀      | ✅     | Dynamically load the new components as the page is loaded            |
| 1st class global state    | ✅      | ✅     | redux/recoil/mobx on top of context                                  |
| Runs natively             | ✅      | ❓     | runs as a portable binary w/o a runtime (Node)                       |
| Subtree Memoization       | ✅      | ❓     | skip diffing static element subtrees                                 |
| Compile-time correct      | ✅      | ❓     | Throw errors on invalid template layouts                             |
| Heuristic Engine          | ✅      | ❓     | track component memory usage to minimize future allocations          |
| Fine-grained reactivity   | 👀      | ❓     | Skip diffing for fine-grain updates                                  |

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on
- ❓ = not sure if will or can implement


## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/tokio-rs/tokio/blob/master/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Pipette by you, shall be licensed as MIT, without any additional
terms or conditions.
