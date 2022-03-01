# Roadmap & Feature-set

Before we dive into Dioxus, feel free to take a look at our feature set and roadmap to see if what Dioxus can do today works for you.

If a feature that you need doesn't exist or you want to contribute to projects on the roadmap, feel free to get involved by [joining the discord](https://discord.gg/XgGxMSkvUM).

Generally, here's the status of each platform:

- **Web**: Dioxus is a great choice for pure web-apps - especially for CRUD/complex apps. However, it does lack the ecosystem of React, so you might be missing a component library or some useful hook.

- **SSR**: Dioxus is a great choice for pre-rendering, hydration, and rendering HTML on a web endpoint. Be warned - the VirtualDom is not (currently) `Send + Sync`.

- **Desktop**: You can build very competent single-window desktop apps right now. However, multi-window apps require support from Dioxus core and are not ready.

- **Mobile**: Mobile support is very young. You'll be figuring things out as you go and there are not many support crates for peripherals.

- **LiveView**: LiveView support is very young. You'll be figuring things out as you go. Thankfully, none of it is too hard and any work can be upstreamed into Dioxus.

## Web Support
---
The Web is the best-supported target platform for Dioxus. To run on the Web, your app must be compiled to WebAssembly and depend on the `dioxus` crate with the `web` feature enabled. Because of the limitations of Wasm  not every crate will work with your web-apps, so you'll need to make sure that your crates work without native system calls (timers, IO, etc).

Because the web is a fairly mature platform, we expect there to be very little API churn for web-based features.

[Jump to the getting started guide for the web.](/reference/platforms/web)

Examples:
- [TodoMVC](https://github.com/DioxusLabs/example-projects/tree/master/todomvc)
- [ECommerce](https://github.com/DioxusLabs/example-projects/tree/master/ecommerce-site)

[![TodoMVC example](https://github.com/DioxusLabs/example-projects/raw/master/todomvc/example.png)](https://github.com/DioxusLabs/example-projects/blob/master/todomvc)

## SSR Support
---
Dioxus supports server-side rendering!

For rendering statically to an `.html` file or from a WebServer, then you'll want to make sure the `ssr` feature is enabled in the `dioxus` crate and use the `dioxus::ssr` API. We don't expect the SSR API to change drastically in the future.

```rust
let contents = dioxus::ssr::render_vdom(&dom);
```

[Jump to the getting started guide for SSR.](/reference/platforms/ssr)

Examples:
- [Example DocSite](https://github.com/dioxusLabs/docsite)
- [Tide WebServer]()
- [Markdown to fancy HTML generator]()

## Desktop Support
---
The desktop is a powerful target for Dioxus, but is currently limited in capability when compared to the Web platform. Currently, desktop apps are rendered with the platform's WebView library, but your Rust code is running natively on a native thread. This means that browser APIs are *not* available, so rendering WebGL, Canvas, etc is not as easy as the Web. However, native system APIs *are* accessible, so streaming, WebSockets, filesystem, etc are all viable APIs. In the future, we plan to move to a custom webrenderer-based DOM renderer with WGPU integrations.

Desktop APIs will likely be in flux as we figure out better patterns than our ElectronJS counterpart.

[Jump to the getting started guide for Desktop.](/reference/platforms/desktop)

Examples:
- [File explorer](https://github.com/dioxusLabs/file-explorer/)
- [WiFi scanner](https://github.com/DioxusLabs/example-projects/blob/master/wifi-scanner)

[![File ExplorerExample](https://raw.githubusercontent.com/DioxusLabs/example-projects/master/file-explorer/image.png)](https://github.com/DioxusLabs/example-projects/tree/master/file-explorer)

## Mobile Support
---
Mobile is currently the least-supported renderer target for Dioxus. Mobile apps are rendered with the platform's WebView, meaning that animations, transparency, and native widgets are not currently achievable. In addition, iOS is the only supported Mobile Platform. It is possible to get Dioxus running on Android and rendered with WebView, but the Rust windowing library that Dioxus uses - tao - does not currently supported Android.

Mobile support is currently best suited for CRUD-style apps, ideally for internal teams who need to develop quickly but don't care much about animations or native widgets.

[Jump to the getting started guide for Mobile.](/reference/platforms/mobile)

Examples:
- [Todo App](https://github.com/DioxusLabs/example-projects/blob/master/ios_demo)

## LiveView / Server Component Support
---

The internal architecture of Dioxus was designed from day one to support the `LiveView` use-case, where a web server hosts a running app for each connected user. As of today, there is no first-class LiveView support - you'll need to wire this up yourself.

While not currently fully implemented, the expectation is that LiveView apps can be a hybrid between Wasm and server-rendered where only portions of a page are "live" and the rest of the page is either server-rendered, statically generated, or handled by the host SPA.

### Multithreaded Support
---
The Dioxus VirtualDom, sadly, is not currently `Send`. Internally, we use quite a bit of interior mutability which is not thread-safe. This means you can't easily use Dioxus with most web frameworks like Tide, Rocket, Axum, etc.

To solve this, you'll want to spawn a VirtualDom on its own thread and communicate with it via channels.

When working with web frameworks that require `Send`, it is possible to render a VirtualDom immediately to a String - but you cannot hold the VirtualDom across an await point. For retained-state SSR (essentially LiveView), you'll need to create a pool of VirtualDoms.

Ultimately, you can always wrap the VirtualDom with a `Send` type and manually uphold the `Send` guarantees yourself.



## Features
---

| Feature                   | Status | Description                                                          |
| ------------------------- | ------ | -------------------------------------------------------------------- |
| Conditional Rendering     | ✅      | if/then to hide/show component                                       |
| Map, Iterator             | ✅      | map/filter/reduce to produce rsx!                                    |
| Keyed Components          | ✅      | advanced diffing with keys                                           |
| Web                       | ✅      | renderer for web browser                                             |
| Desktop (webview)         | ✅      | renderer for desktop                                                 |
| Shared State (Context)    | ✅      | share state through the tree                                         |
| Hooks                     | ✅      | memory cells in components                                           |
| SSR                       | ✅      | render directly to string                                            |
| Component Children        | ✅      | cx.children() as a list of nodes                                     |
| Headless components       | ✅      | components that don't return real elements                           |
| Fragments                 | ✅      | multiple elements without a real root                                |
| Manual Props              | ✅      | Manually pass in props with spread syntax                            |
| Controlled Inputs         | ✅      | stateful wrappers around inputs                                      |
| CSS/Inline Styles         | ✅      | syntax for inline styles/attribute groups                            |
| Custom elements           | ✅      | Define new element primitives                                        |
| Suspense                  | ✅      | schedule future render from future/promise                           |
| Integrated error handling | ✅      | Gracefully handle errors with ? syntax                               |
| NodeRef                   | ✅      | gain direct access to nodes                                          |
| Re-hydration              | ✅      | Pre-render to HTML to speed up first contentful paint                |
| Jank-Free Rendering       | ✅      | Large diffs are segmented across frames for silky-smooth transitions |
| Effects                   | ✅      | Run effects after a component has been committed to render           |
| Portals                   | 🛠      | Render nodes outside of the traditional tree structure               |
| Cooperative Scheduling    | 🛠      | Prioritize important events over non-important events                |
| Server Components         | 🛠      | Hybrid components for SPA and Server                                 |
| Bundle Splitting          | 👀      | Efficiently and asynchronously load the app                          |
| Lazy Components           | 👀      | Dynamically load the new components as the page is loaded            |
| 1st class global state    | ✅      | redux/recoil/mobx on top of context                                  |
| Runs natively             | ✅      | runs as a portable binary w/o a runtime (Node)                       |
| Subtree Memoization       | ✅      | skip diffing static element subtrees                                 |
| High-efficiency templates | 🛠      | rsx! calls are translated to templates on the DOM's side             |
| Compile-time correct      | ✅      | Throw errors on invalid template layouts                             |
| Heuristic Engine          | ✅      | track component memory usage to minimize future allocations          |
| Fine-grained reactivity   | 👀      | Skip diffing for fine-grain updates                                  |

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on
- ❓ = not sure if will or can implement




## Roadmap
---



<!--

Core:
- [x] Release of Dioxus Core
- [x] Upgrade documentation to include more theory and be more comprehensive
- [ ] Support for HTML-side templates for lightning-fast dom manipulation
- [ ] Support for multiple renderers for same virtualdom (subtrees)
- [ ] Support for ThreadSafe (Send + Sync)
- [ ] Support for Portals

SSR
- [x] SSR Support + Hydration
- [ ] Integrated suspense support for SSR

Desktop
- [ ] Declarative window management
- [ ] Templates for building/bundling
- [ ] Fully native renderer
- [ ] Access to Canvas/WebGL context natively

Mobile
- [ ] Mobile standard library
  - [ ] GPS
  - [ ] Camera
  - [ ] filesystem
  - [ ] Biometrics
  - [ ] WiFi
  - [ ] Bluetooth
  - [ ] Notifications
  - [ ] Clipboard
  - [ ]

Bundling (CLI)
- [x] translation from HTML into RSX
- [ ] dev server
- [ ] live reload
- [ ] translation from JSX into RSX
- [ ] hot module replacement
- [ ] code splitting
- [ ] asset macros
- [ ] css pipeline
- [ ] image pipeline

Essential hooks
- [ ] Router
- [ ] Global state management
- [ ] Resize observer

 -->
