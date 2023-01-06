# Roadmap & Feature-set

This feature set and roadmap can help you decide if what Dioxus can do today works for you.

If a feature that you need doesn't exist or you want to contribute to projects on the roadmap, feel free to get involved by [joining the discord](https://discord.gg/XgGxMSkvUM).

Generally, here's the status of each platform:

- **Web**: Dioxus is a great choice for pure web-apps – especially for CRUD/complex apps. However, it does lack the ecosystem of React, so you might be missing a component library or some useful hook.

- **SSR**: Dioxus is a great choice for pre-rendering, hydration, and rendering HTML on a web endpoint. Be warned – the VirtualDom is not (currently) `Send + Sync`.

- **Desktop**: You can build very competent single-window desktop apps right now. However, multi-window apps require support from Dioxus core and are not ready.

- **Mobile**: Mobile support is very young. You'll be figuring things out as you go and there are not many support crates for peripherals.

- **LiveView**: LiveView support is very young. You'll be figuring things out as you go. Thankfully, none of it is too hard and any work can be upstreamed into Dioxus.

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
| High-efficiency templates | ✅      | rsx! calls are translated to templates on the DOM's side             |
| Compile-time correct      | ✅      | Throw errors on invalid template layouts                             |
| Heuristic Engine          | ✅      | track component memory usage to minimize future allocations          |
| Fine-grained reactivity   | 👀      | Skip diffing for fine-grain updates                                  |

- ✅ = implemented and working
- 🛠 = actively being worked on
- 👀 = not yet implemented or being worked on


## Roadmap
These Features are planned for the future of Dioxus:

### Core
- [x] Release of Dioxus Core
- [x] Upgrade documentation to include more theory and be more comprehensive
- [x] Support for HTML-side templates for lightning-fast dom manipulation
- [ ] Support for multiple renderers for same virtualdom (subtrees)
- [ ] Support for ThreadSafe (Send + Sync)
- [ ] Support for Portals

### SSR
- [x] SSR Support + Hydration
- [ ] Integrated suspense support for SSR

### Desktop
- [ ] Declarative window management
- [ ] Templates for building/bundling
- [ ] Fully native renderer
- [ ] Access to Canvas/WebGL context natively

### Mobile
- [ ] Mobile standard library
  - [ ] GPS
  - [ ] Camera
  - [ ] filesystem
  - [ ] Biometrics
  - [ ] WiFi
  - [ ] Bluetooth
  - [ ] Notifications
  - [ ] Clipboard
- [ ] Animations
- [ ] Native Renderer

### Bundling (CLI)
- [x] Translation from HTML into RSX
- [x] Dev server
- [x] Live reload
- [x] Translation from JSX into RSX
- [ ] Hot module replacement
- [ ] Code splitting
- [ ] Asset macros
- [ ] Css pipeline
- [ ] Image pipeline

### Essential hooks
- [x] Router
- [x] Global state management
- [ ] Resize observer


## Work in Progress

### Build Tool

We are currently working on our own build tool called [Dioxus CLI](https://github.com/DioxusLabs/cli) which will support:

- an interactive TUI
- on-the-fly reconfiguration
- hot CSS reloading
- two-way data binding between browser and source code
- an interpreter for `rsx!`
- ability to publish to github/netlify/vercel
- bundling for iOS/Desktop/etc

### Server Component Support

While not currently fully implemented, the expectation is that LiveView apps can be a hybrid between Wasm and server-rendered where only portions of a page are "live" and the rest of the page is either server-rendered, statically generated, or handled by the host SPA.

### Native rendering

We are currently working on a native renderer for Dioxus using WGPU called [Blitz](https://github.com/DioxusLabs/blitz/). This will allow you to build apps that are rendered natively for iOS, Android, and Desktop.
