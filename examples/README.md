# Examples

Most of these examples are run through webview so you don't need the dioxus cli installed to preview the functionality.

These examples are fully-fledged micro apps. They can be ran with the `cargo run --example XYZ`

| Example                                             | What it does                                | Status |
| --------------------------------------------------- | ------------------------------------------- | ------ |
| [The basics](./basics.rs)                           | A few basic examples to preview Dioxus      | 🛠      |
| [fine grained reactivity](./signals.rs)             | Escape `diffing` by writing values directly | 🛠      |
| [Global State Management](./statemanagement.rs)     | Share state between components              | 🛠      |
| [Virtual Refs]()                                    | Cross-platform imperative elements          | 🛠      |
| [Inline Styles](./inline-styles.rs)                 | Define styles for elements inline           | 🛠      |
| [Conditional Rendering](./conditional-rendering.rs) | Hide/Show elements using conditionals       | ✅     |

These examples are not necessarily meant to be run, but rather serve as a reference for the given functionality.

| Example                                             | What it does                                    | Status |
| --------------------------------------------------- | ----------------------------------------------- | ------ |
| [The basics](./basics.rs)                           | A few basic examples to preview Dioxus          | 🛠      |
| [fine grained reactivity](./signals.rs)             | Escape `diffing` by writing values directly     | 🛠      |
| [Global State Management](./statemanagement.rs)     | Share state between components                  | 🛠      |
| [Virtual Refs]()                                    | Cross-platform imperative elements              | 🛠      |
| [Inline Styles](./inline-styles.rs)                 | Define styles for elements inline               | 🛠      |
| [Conditional Rendering](./conditional-rendering.rs) | Hide/Show elements using conditionals           | ✅     |
| [Maps/Iterators](./iterators.rs)                    | Use iterators in the rsx! macro                 | 🛠      |
| [Render To string](./tostring.rs)                   | Render a mounted virtualdom to a string         | 🛠      |
| [Component Children](./children.rs)                 | Pass children into child components             | 🛠      |
| [Function Driven children]()                        | Pass functions to make VNodes                   | 🛠      |
| [Memoization & Borrowed Data](./memo.rs)            | Suppress renders, borrow from parents           | ✅     |
| [Fragments](./fragments.rs)                         | Support root-less element groups                | ✅     |
| [Null/None Components](./empty.rs)                  | Return nothing!                                 | 🛠      |
| [Spread Pattern for props](./spreadpattern.rs)      | Manually specify and override props             | ✅     |
| [Controlled Inputs](./controlled-inputs.rs)         | this does                                       | 🛠      |
| [Custom Elements]()                                 | Define custom elements                          | 🛠      |
| [Web Components]()                                  | Custom elements to interface with WebComponents | 🛠      |
| [Testing And debugging]()                           | this does                                       | 🛠      |
| [Asynchronous Data]()                               | Using suspense to wait for data                 | 🛠      |
| [Fiber/Scheduled Rendering]()                       | this does                                       | 🛠      |
| [CSS Compiled Styles]()                             | this does                                       | 🛠      |
| [Anti-patterns](./antipatterns.rs)                  | A collection of discouraged patterns            | ✅     |
| [Complete rsx reference](./rsx_usage.rs)            | A complete reference for all rsx! usage         | ✅     |
| [Event Listeners](./listener.rs)                    | Attach closures to events on elements           | ✅     |

These web-specific examples must be run with `dioxus-cli` using `dioxus develop --example XYZ`

| Example | What it does |
| ------- | ------------ |
| asd     | this does    |
| asd     | this does    |
