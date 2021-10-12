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
static App: FC<()> = |cx, props| {
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

## Examples:

| File Navigator (Desktop)                                                         | Bluetooth scanner (Desktop)                                      | TodoMVC (All platforms)                                               | Widget Gallery                                                   |
| -------------------------------------------------------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------------------- |
| ![asd](https://github.com/DioxusLabs/file-explorer-example/raw/master/image.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://github.com/DioxusLabs/todomvc/blob/master/example.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) |



| E-Commerce Site                                                  | Doxie Documentation Generator                                    | Chatroom                                                         | Widget Gallery                                                   |
| ---------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- |
| ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) |



| Printer Demo                                                     | Slide Puzzle                                                     | Todo                                                             | Widget Gallery                                                   |
| ---------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------- |
| ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) | ![asd](https://sixtyfps.io/resources/printerdemo_screenshot.png) |




See the awesome-dioxus page for a curated list of content in the Dioxus Ecosystem.

<!-- 
currently commented out until we have more content on the website
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
-->

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

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on
- ❓ = not sure if will or can implement

## FAQ:

### Aren't VDOMs just pure overhead? Why not something like Solid or Svelte?
Remember: Dioxus is a library - not a compiler like Svelte. Plus, the inner VirtualDOM allows Dioxus to easily port into different runtimes, support SSR, and run remotely in the cloud. VDOMs tend to more ergonomic to work with and feel roughly like natural Rust code. The overhead of Dioxus is **extraordinarily** minimal... sure, there may be some overhead but on an order of magnitude lower than the time required to actually update the page.


### Isn't the overhead for interacting with the DOM from WASM too much?
The overhead layer between WASM and JS APIs is extremely poorly understood. Rust web benchmarks typically suffer from differences in how Rust and JS cache strings. In Dioxus, we solve most of these issues and our JS Framework Benchmark actually beats the WASM Bindgen benchmark in many cases. Compared to a "pure vanilla JS" solution, Dioxus adds less than 5% of overhead and takes advantage of batched DOM manipulation.

### Aren't WASM binaries too huge to deploy in production?
WASM binary sizes are another poorly understood characteristic of Rust web apps. 50kb of WASM and 50kb of JS are _not_ made equally. In JS, the code must be downloaded _first_ and _then_ JIT-ted. Just-in-time compiling 50kb of JavaScript takes a while which is why 50kb of JavaScript sounds like a lot! However, with WASM, the code is downloaded and JIT-ted _simultaneously_ through the magic of streaming compilation. By the time the 50kb of Rust is finished downloading, it is already ready to go. Again, Dioxus beats out many benchmarks with time-to-interactivity.

For reference, Dioxus `hello-world` clocks in at around 70kb gzip or 60kb brotli, and Dioxus supports SSR.

### Why hooks? Why not MVC, classes, traits, messages, etc?
There are plenty Rust Elm-like frameworks in the world - we were not interested in making another! Instead, we borrowed hooks from React. JS and Rust share many structural similarities, so if you're comfortable with React, then you'll be plenty comfortable with Dioxus.

### Why a custom DSL? Why not just pure function calls?
The `RSX` DSL is _barely_ a DSL. Rustaceans will find the DSL very similar to simply assembling nested structs, but without the syntactical overhead of "Default" everywhere or having to jump through hoops with the builder pattern. Between RSX, HTML, the Raw Factory API, and the NodeBuilder syntax, there's plenty of options to choose from.

### What are the build times like? Why on earth would I choose Rust instead of JS/TS/Elm?
Dioxus builds as roughly as fast as a complex WebPack-TypeScript site. Compile times will be slower than an equivalent TypeScript site, but not unbearably slow. The WASM compiler backend for Rust is very fast. Iterating on small components is basically instant and larger apps takes a few seconds. In practice, the compiler guarantees of Rust balance out the rebuild times.

### What about Yew/Seed/Sycamore/Dominator/Dodrio/Percy?
- Yew and Seed use an Elm-like pattern and don't support SSR or any alternate rendering platforms
- Sycamore and Dominator are more like SolidJS/Svelte, requiring no VDOM but has less naturally-Rusty state management
- Percy isn't quite mature yet
- Dodrio is the spiritual predecessor of Dioxus, but is currently an archived research project without the batteries of Dioxus

### How do the mobile and desktop renderers work? Is it Electron?
Currently, Dioxus uses your device's native WebView library to draw the page. None of your app code is actually running in the WebView thread, so you can access system resources instead of having to go through something like NodeJS. This means your app will use Safari on macOS/iOS, Edge (Chromium) on Windows, and whatever is the default Web Browser for Linux and Android. Because your code is compiled and running natively, performance is not a problem. You will have to use the various "Escape Hatches" to use browser-native APIs (like WebGL) and work around visual differences in how Safari and Chrome render the page.

In the future, we are interested in using Webrenderer to provide a fully native renderer without having to go through the system WebView library. In practice, Dioxus mobile and desktop are great for CRUD-style apps, but the ergonomic cross-platform APIs (GPS, Camera, etc) are not there yet.

### Why NOT Dioxus?
You shouldn't use Dioxus if:
- You don't like the React Hooks approach to frontend
- You need a no-std renderer
- You want to support browsers where WASM or asm.js are not supported.


## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/tokio-rs/tokio/blob/master/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Pipette by you, shall be licensed as MIT, without any additional
terms or conditions.
