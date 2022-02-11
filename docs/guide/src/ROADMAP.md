# Roadmap & Feature-set

Before we dive into Dioxus, feel free to take a look at our feature set and roadmap to see if what Dioxus can do today works for you.

If a feature that you need doesn't exist or you want to contribute to projects on the roadmap, feel free to get involved by [joining the discord](https://discord.gg/XgGxMSkvUM).

Generally, here's the status of each platform:

- **Web**: Dioxus is a great choice for pure web-apps - especially for CRUD/complex apps. However, it does lack the ecosystem of React, so you might be missing a component library or some useful hook.

- **SSR**: Dioxus is a great choice for pre-rendering, hydration, and rendering HTML on a web endpoint. Be warned - the VirtualDom is not (currently) `Send + Sync`.

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


