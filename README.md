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
  <a href="https://github.com/async-email/dioxus/actions">
    <img src="https://github.com/async-email/dioxus/workflows/CI/badge.svg"
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
fn App(cx: Context<()>) -> DomTree {
    let mut count = use_state(cx, || 0);

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
};
```

Dioxus can be used to deliver webapps, desktop apps, static pages, liveview apps, eventually mobile apps (WIP), and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

If you know React, then you already know Dioxus.

### **Things you'll love ❤️:**

- Ergonomic design
- Minimal boilerplate
- Simple build, test, and deploy
- Compile-time correct templating
- Support for fine-grained reactivity
- Support for html! and rsx! templates
- SSR, WASM, desktop, and mobile support
- Support for asynchronous batched rendering
- Powerful and simple integrated state management
- Rust! (enums, static types, modules, efficiency)

### Unique features:
- Incredible inline documentation. Supports hover and guides for all HTML elements, listeners, and events.
- Templates are "constified" at compile time. Nodes that don't change will won't be diffed.
- Custom bump-allocator backing for all components. Nearly 0 allocations for steady-state components.
- Starting a new app takes zero templates or special tools - get a new app running in just seconds.
- Desktop apps running natively (no Electron!) in less than 10 lines of code.
- The most ergonomic and powerful state management of any Rust UI toolkit.
- Multithreaded asynchronous coroutine scheduler for powerful async code.
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

## Explore

- [**Fine-grained reactivity**: Skip the diff overhead with signals ](docs/guides/00-index.md)
- [**HTML Templates**: Drop in existing HTML5 templates with html! macro](docs/guides/00-index.md)
- [**RSX Templates**: Clean component design with rsx! macro](docs/guides/00-index.md)
- [**Running the examples**: Explore the vast collection of samples, tutorials, and demos](docs/guides/00-index.md)
- [**Building applications**: Use the Dioxus CLI to build and bundle apps for various platforms](docs/guides/01-ssr.md)
- [**Liveview**: Build custom liveview components that simplify datafetching on all platforms](docs/guides/01-ssr.md)
- [**State management**: Easily add powerful state management that comes integrated with Dioxus Core](docs/guides/01-ssr.md)
- [**Concurrency**: Drop in async where it fits and suspend components until new data is ready](docs/guides/01-ssr.md)
- [**1st party hooks**: Cross-platform router hook](docs/guides/01-ssr.md)
- [**Community hooks**: 3D renderers](docs/guides/01-ssr.md)

## Blog Posts

- [Why we need a stronger typed web]()
- [Isomorphic webapps in 10 minutes]()
- [Rust is high level too]()
- [Eliminating crashes with Rust webapps]()
- [Tailwind for Dioxus]()
- [The monoglot startup]()

## Why?

TypeScript is a great addition to JavaScript, but comes with a lot of tweaking flags, a slight performance hit, and an uneven ecosystem where some of the most important packages are not properly typed. TypeScript provides a lot of great benefits to JS projects, but comes with its own "tax" that can slow down dev teams. Rust can be seen as a step up from TypeScript, supporting:

- static types for _all_ libraries
- advanced pattern matching
- immutability by default
- clean, composable iterators
- a good module system
- integrated documentation
- inline built-in unit/integration testing
- best-in-class error handling
- simple and fast build system (compared to webpack!)
- powerful standard library (no need for lodash or underscore)
- include_str! for integrating html/css/svg templates directly
- various macros (`html!`, `rsx!`) for fast template iteration

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time. Dioxus also works on the server, on the web, on mobile, on desktop - and it runs completely natively so performance is never an issue.

# Parity with React

Dioxus is heavily inspired by React, but we want your transition to feel like an upgrade. Dioxus is _most_ of the way there, but missing a few key features. This parity table does not necessarily include important ecosystem crates like code blocks, markdown, resizing hooks, etc.

### Phase 1: The Basics

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
| Cooperative Scheduling    | ✅      | ✅     | Prioritize important events over non-important events                |
| Runs natively             | ✅      | ❓     | runs as a portable binary w/o a runtime (Node)                       |
| 1st class global state    | ✅      | ❓     | redux/recoil/mobx on top of context                                  |
| Subtree Memoization       | ✅      | ❓     | skip diffing static element subtrees                                 |
| Compile-time correct      | ✅      | ❓     | Throw errors on invalid template layouts                             |
| Heuristic Engine          | ✅      | ❓     | track component memory usage to minimize future allocations          |
| Fine-grained reactivity   | 🛠      | ❓     | Skip diffing for fine-grain updates                                  |
| Effects                   | 🛠      | ✅     | Run effects after a component has been committed to render           |


### Phase 2: Advanced Toolkits

| Feature               | Dioxus | React | Notes for Dioxus                   |
| --------------------- | ------ | ----- | ---------------------------------- |
| 1st class router      | 👀      | ✅     | Hook built on top of history       |
| Assets                | 👀      | ✅     | include css/svg/img url statically |
| Integrated classnames | 🛠      | ❓     | built-in `classnames`              |
| Transition            | 👀      | 🛠     | High-level control over suspense   |
| Animation             | 👀      | ✅     | Spring-style animations            |
| Native Mobile         | 👀      | ✅     | Render with cacao                  |
| Native Desktop        | 👀      | ✅     | Render with native desktop         |
| 3D Renderer           | 👀      | ✅     | react-three-fiber                  |

### Phase 3: Additional Complexity

| Feature              | Dioxus | React | Notes for Dioxus                     |
| -------------------- | ------ | ----- | ------------------------------------ |
| Portal               | ❓      | ✅     | cast elements through tree           |
| Error/Panic boundary | 👀      | ✅     | catch panics and display custom BSOD |
| Code-splitting       | 👀      | ✅     | Make bundle smaller/lazy             |
| LiveView             | 👀      | ❓     | Example for SSR + WASM apps          |

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on
- ❓ = not sure if will or can implement
