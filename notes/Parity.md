# Parity with React

Sorted by priority

| Feature                | Dioxus | React | Notes                                            |
| ---------------------- | ------ | ----- | ------------------------------------------------ |
| ----- Phase 1 -----    | -----  | ----- | -----                                            |
| Conditional Rendering  | ✅     | ✅    | if/then to hide/show component                   |
| Map, Iterator          | ✅     | ✅    | map/filter/reduce rsx!                           |
| Keyed Components       | ✅     | ✅    | advanced diffing with keys                       |
| Web                    | ✅     | ✅    | renderer for web browser                         |
| Desktop (webview)      | ✅     | ✅    | renderer for desktop                             |
| Context                | ✅     | ✅    | share state through the tree                     |
| Hook                   | ✅     | ✅    | memory cells in components                       |
| SSR                    | ✅     | ✅    | render directly to string                        |
| Runs natively          | ✅     | 👀    | runs as a sharable binary                        |
| Component Children     | ✅     | ✅    | ctx.children() as a list of nodes                |
| Null components        | ✅     | ✅    | allow returning no components                    |
| No-div components      | ✅     | ✅    | components that render components                |
| Fragments              | ✅     | ✅    | rsx! can return multiple elements without a root |
| NodeRef                | 👀     | ✅    | gain direct access to nodes                      |
| Controlled Inputs      | 👀     | ✅    | stateful wrappers around inputs                  |
| CSS/Inline Styles      | 🛠      | ✅    | syntax for inline/conditional styles             |
| 1st class global state | 🛠      | ✅    | redux/recoil/mobx on top of context              |
| ----- Phase 2 -----    | -----  | ----- | -----                                            |
| 1st class router       | 👀     | ✅    | Hook built on top of history                     |
| Assets                 | 👀     | ✅    | include css/svg/img url statically               |
| Integrated classnames  | 🛠      | 👀    | built-in `classnames`                            |
| Suspense               | 👀     | 👀    | schedule future render from future               |
| Transition             | 👀     | 👀    | High-level control over suspense                 |
| Animation              | 👀     | ✅    | Spring-style animations                          |
| Mobile                 | 👀     | ✅    | Render with cacao                                |
| Desktop (native)       | 👀     | ✅    | Render with native desktop                       |
| 3D Renderer            | 👀     | ✅    | react-three-fiber                                |
| ----- Phase 3 -----    | -----  | ----- | -----                                            |
| Portal                 | 👀     | ✅    | cast elements through tree                       |
| Error/Panic boundary   | 👀     | ✅    | catch panics and display BSOD                    |
| Code-splitting         | 👀     | ✅    | Make bundle smaller/lazy                         |
| LiveView               | 👀     | 👀    | Example for SSR + WASM apps                      |

## Required services:

---

Gloo is covering a lot of these. We want to build hooks around these, and provide examples on how to use them.
https://github.com/rustwasm/gloo

If the gloo service doesn't exist, then we need to contribute to the project

| Service                      | Hook examples | Current Projects |
| ---------------------------- | ------------- | ---------------- |
| Fetch                        | 👀            | Reqwest/surf     |
| Local storage (cache)        | 👀            | Gloo             |
| Persistent storage (IndexDB) | 👀            | 👀               |
| WebSocket                    | 👀            | Gloo             |
| 3D Renderer / WebGL          | 👀            | Gloo             |
| Web Worker                   | 👀            | 👀               |
| Router                       | 👀            | 👀               |
| Notifications                | 👀            | 👀               |
| WebRTC Client                | 👀            | 👀               |
| Service Workers              | 👀            | 👀               |
| Resize Observer              | 👀            | 👀               |
| Canvas                       | 👀            | 👀               |
| Clipboard                    | 👀            | 👀               |
| Fullscreen                   | 👀            | 👀               |
| History API                  | 👀            | 👀               |
