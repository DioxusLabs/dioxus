<div align="center">
  <h1>🌗🚀 Dioxus</h1>
  <p>
    <strong>Frontend that scales.</strong>
  </p>
</div>

Dioxus is a portable, performant, and ergonomic framework for building cross-platform user experiences in Rust.

```rust
fn Example(cx: Context<()>) -> VNode {
    let (selection, set_selection) = use_state(&cx, || "..?");

    cx.render(rsx! {
        h1 { "Hello, {selection}" }
        button { "?", onclick: move |_| set_selection("world!")}
        button { "?", onclick: move |_| set_selection("Dioxus 🎉")}
    })
};
```

Dioxus can be used to deliver webapps, desktop apps, static pages, liveview apps, eventually mobile apps (WIP), and more. At its core, Dioxus is entirely renderer agnostic and has great documentation for creating new renderers for any platform.

If you know React, then you already know Dioxus.

### **Things you'll love ❤️:**

- Ergonomic design
- Minimal boilerplate
- Familiar design and semantics
- Simple build, test, and deploy
- Compile-time correct templating
- Support for html! and rsx! templating
- SSR, WASM, desktop, and mobile support
- Powerful and simple integrated state management
- Rust! (enums, static types, modules, efficiency)

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

---

TypeScript is a great addition to JavaScript, but comes with a lot of tweaking flags, a slight performance hit, and an uneven ecosystem where some of the most important packages are not properly typed. TypeScript provides a lot of great benefits to JS projects, but comes with its own "tax" that can slow down dev teams. Rust can be seen as a step up from TypeScript, supporting:

- static types for _all_ libraries
- advanced pattern matching
- immutability by default
- clean, composable iterators
- a good module system
- integrated documentation
- inline built-in unit/integration testing
- best-in-class error handling
- simple and fast build system
- powerful standard library (no need for lodash or underscore)
- include_str! for integrating html/css/svg templates directly
- various macros (`html!`, `rsx!`) for fast template iteration

And much more. Dioxus makes Rust apps just as fast to write as React apps, but affords more robustness, giving your frontend team greater confidence in making big changes in shorter time. Dioxus also works on the server, on the web, on mobile, on desktop - and it runs completely natively so performance is never an issue.

# Parity with React

Dioxus is heavily inspired by React, but we want your transition to feel like an upgrade. Dioxus is _most_ of the way there, but missing a few key features. This parity table does not necessarily include important ecosystem crates like code blocks, markdown, resizing hooks, etc.

### Phase 1: The Basics

| Feature                | Dioxus | React | Notes for Dioxus                                 |
| ---------------------- | ------ | ----- | ------------------------------------------------ |
| Conditional Rendering  | ✅     | ✅    | if/then to hide/show component                   |
| Map, Iterator          | ✅     | ✅    | map/filter/reduce rsx!                           |
| Keyed Components       | ✅     | ✅    | advanced diffing with keys                       |
| Web                    | ✅     | ✅    | renderer for web browser                         |
| Desktop (webview)      | ✅     | ✅    | renderer for desktop                             |
| Context                | ✅     | ✅    | share state through the tree                     |
| Hook                   | ✅     | ✅    | memory cells in components                       |
| SSR                    | ✅     | ✅    | render directly to string                        |
| Runs natively          | ✅     | ❓    | runs as a portable binary w/o a runtime (Node)   |
| Component Children     | ✅     | ✅    | cx.children() as a list of nodes                 |
| Null components        | ✅     | ✅    | allow returning no components                    |
| No-div components      | ✅     | ✅    | components that render components                |
| Fragments              | ✅     | ✅    | rsx! can return multiple elements without a root |
| Manual Props           | ✅     | ✅    | Manually pass in props with spread syntax        |
| Controlled Inputs      | ✅     | ✅    | stateful wrappers around inputs                  |
| 1st class global state | 🛠      | ✅    | redux/recoil/mobx on top of context              |
| NodeRef                | 🛠      | ✅    | gain direct access to nodes [1]                  |
| CSS/Inline Styles      | 🛠      | ✅    | syntax for inline styles[2]                      |

- [1] Currently blocked until we figure out a cross-platform way of exposing an imperative Node API.
- [2] Would like to solve this in a more general way. Something like attribute groups that's not styling-specific.

### Phase 2: Advanced Toolkits

| Feature               | Dioxus | React | Notes for Dioxus                           |
| --------------------- | ------ | ----- | ------------------------------------------ |
| 1st class router      | 👀     | ✅    | Hook built on top of history               |
| Assets                | 👀     | ✅    | include css/svg/img url statically         |
| Integrated classnames | 🛠      | ❓    | built-in `classnames`                      |
| Suspense              | 👀     | 🛠     | schedule future render from future/promise |
| Transition            | 👀     | 🛠     | High-level control over suspense           |
| Animation             | 👀     | ✅    | Spring-style animations                    |
| Mobile                | 👀     | ✅    | Render with cacao                          |
| Desktop (native)      | 👀     | ✅    | Render with native desktop                 |
| 3D Renderer           | 👀     | ✅    | react-three-fiber                          |

### Phase 3: Additional Complexity

| Feature              | Dioxus | React | Notes for Dioxus                     |
| -------------------- | ------ | ----- | ------------------------------------ |
| Portal               | ❓     | ✅    | cast elements through tree           |
| Error/Panic boundary | ❓     | ✅    | catch panics and display custom BSOD |
| Code-splitting       | 👀     | ✅    | Make bundle smaller/lazy             |
| LiveView             | 👀     | ❓    | Example for SSR + WASM apps          |

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on
- ❓ = not sure if will or can implement
