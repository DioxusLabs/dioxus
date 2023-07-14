# Project Struture

There are many packages in the Dioxus organization. This document will help you understand the purpose of each package and how they fit together.

## Renderers

- [Desktop](https://github.com/DioxusLabs/dioxus/tree/master/packages/desktop): A renderer that runs Dioxus applications natively, but renders them with the system webview.
- [Mobile](https://github.com/DioxusLabs/dioxus/tree/master/packages/mobile): A renderer that runs Dioxus applications natively, but renders them with the system webview. This is currently a copy of the desktop renderer.
- [Web](https://github.com/DioxusLabs/dioxus/tree/master/packages/Web): Renders Dioxus applications in the browser by compiling to WASM and manipulating the DOM.
- [Liveview](https://github.com/DioxusLabs/dioxus/tree/master/packages/liveview): A renderer that runs on the server, and renders using a websocket proxy in the browser.
- [Rink](https://github.com/DioxusLabs/dioxus/tree/master/packages/rink): A renderer that renders a HTML-like tree into a terminal.
- [TUI](https://github.com/DioxusLabs/dioxus/tree/master/packages/dioxus-tui): A renderer that uses Rink to render a Dioxus application in a terminal.
- [Blitz-Core](https://github.com/DioxusLabs/blitz/tree/master/blitz-core): An experimental native renderer that renders a HTML-like tree using WGPU.
- [Blitz](https://github.com/DioxusLabs/blitz): An experimental native renderer that uses Blitz-Core to render a Dioxus application using WGPU.
- [SSR](https://github.com/DioxusLabs/dioxus/tree/master/packages/ssr): A renderer that runs Dioxus applications on the server, and renders them to HTML.

## State Management/Hooks

- [Hooks](https://github.com/DioxusLabs/dioxus/tree/master/packages/hooks): A collection of common hooks for Dioxus applications
- [Signals](https://github.com/DioxusLabs/dioxus/tree/master/packages/signals): A experimental state management library for Dioxus applications. This currently contains a `Copy` version of UseRef
- [Dioxus STD](https://github.com/DioxusLabs/dioxus-std): A collection of platform agnostic hooks to interact with system interfaces (The clipboard, camera, etc.).
- [Fermi](https://github.com/DioxusLabs/dioxus/tree/master/packages/fermi): A global state management library for Dioxus applications.
  [Router](https://github.com/DioxusLabs/dioxus/tree/master/packages/router): A client-side router for Dioxus applications

## Core utilities

- [core](https://github.com/DioxusLabs/dioxus/tree/master/packages/core): The core virtual dom implementation every Dioxus application uses
  - You can read more about the archetecture of the core [in this blog post](https://dioxuslabs.com/blog/templates-diffing/) and the [custom renderer section of the guide](../custom_renderer/index.md)
- [RSX](https://github.com/DioxusLabs/dioxus/tree/master/packages/RSX): The core parsing for RSX used for hot reloading, autoformatting, and the macro
- [core-macro](https://github.com/DioxusLabs/dioxus/tree/master/packages/core-macro): The rsx! macro used to write Dioxus applications. (This is a wrapper over the RSX crate)
- [HTML macro](https://github.com/DioxusLabs/dioxus-html-macro): A html-like alternative to the RSX macro

## Native Renderer Utilities

- [native-core](https://github.com/DioxusLabs/dioxus/tree/master/packages/native-core): Incrementally computed tree of states (mostly styles)
  - You can read more about how native-core can help you build native renderers in the [custom renderer section of the guide](../custom_renderer/index.html#native-core)
- [native-core-macro](https://github.com/DioxusLabs/dioxus/tree/master/packages/native-core-macro): A helper macro for native core
- [Taffy](https://github.com/DioxusLabs/taffy): Layout engine powering Blitz-Core, Rink, and Bevy UI

## Web renderer tooling

- [HTML](https://github.com/DioxusLabs/dioxus/tree/master/packages/html): defines html specific elements, events, and attributes
- [Interpreter](https://github.com/DioxusLabs/dioxus/tree/master/packages/interpreter): defines browser bindings used by the web and desktop renderers

## Developer tooling

- [hot-reload](https://github.com/DioxusLabs/dioxus/tree/master/packages/hot-reload): Macro that uses the RSX crate to hot reload static parts of any rsx! macro. This macro works with any non-web renderer with an [integration](https://crates.io/crates/dioxus-hot-reload)
- [autofmt](https://github.com/DioxusLabs/dioxus/tree/master/packages/autofmt): Formats RSX code
- [rsx-rosetta](https://github.com/DioxusLabs/dioxus/tree/master/packages/RSX-rosetta): Handles conversion between HTML and RSX
- [CLI](https://github.com/DioxusLabs/cli): A Command Line Interface and VSCode extension to assist with Dioxus usage
