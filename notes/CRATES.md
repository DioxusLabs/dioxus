# Crates

The workspace crates under `packages/`, grouped into layers. Dependencies point down the stack:
Framework depends on Core libraries, Core libraries on Core, and Core on Base. The CLI sits to the
side: it uses Base crates but no crate depends across the CLI boundary.

```
Framework       dioxus, renderers, fullstack
Core libraries  devtools, history, router
Core            dioxus-core facade, vdom, rsx, signals, hooks, html
Base            subsecond, manganis, const-serialize, generational-box, ...
                                                    CLI: dx and its helper crates
```

Each entry is `directory` (`crate name`) when the two differ.

## Base (16 crates)

General-purpose building blocks with no dependency on Dioxus Core. Usable outside Dioxus.

| Directory | Crate | What it is |
| --- | --- | --- |
| `const-array-vec` | `const-array-vec` | A fixed-capacity `Vec` usable in `const` contexts. |
| `subsecond/subsecond` | `subsecond` | Runtime hot-patching engine: calls go through a jump table that `dx` patches with newly compiled code. |
| `subsecond/subsecond-types` | `subsecond-types` | Shared types for subsecond (`JumpTable`, `AddressHasher`). |
| `dx-env` | `dx-env` | The generic env-var contract between `dx` and the app it launches (`DX_*`, falling back to `DIOXUS_*`). |
| `cli-config` | `dioxus-cli-config` | Dioxus-specific settings `dx` passes to apps (window title, always-on-top), on top of `dx-env`, which it re-exports. |
| `const-serialize` | `const-serialize` | Serialization that works in `const` contexts; used to encode asset metadata into link sections. |
| `const-serialize-macro` | `const-serialize-macro` | `#[derive(SerializeConst)]` for `const-serialize`. |
| `manganis/manganis` | `manganis` | The `asset!()` macro and asset options: assets are recorded as linker symbols that `dx` collects and optimizes. |
| `manganis/manganis-core` | `manganis-core` | Core asset types (`Asset`, `BundledAsset`, asset options) shared by manganis, the CLI and renderers. |
| `manganis/manganis-macro` | `manganis-macro` | Proc macros behind `asset!()` and friends. |
| `wasm-split/wasm-split` | `wasm-splitter` | User-facing WASM code-splitting runtime: `#[wasm_split]`, `lazy_loader!` and the lazy loaders. Published as `wasm-splitter`. |
| `wasm-split/wasm-split-macro` | `wasm-split-macro` | Proc macros that generate the split points and loaders for `wasm-splitter`. |
| `lazy-js-bundle` | `lazy-js-bundle` | Build-script helper that bundles TypeScript into JavaScript without requiring a bundler to be installed. |
| `config-macros` | `dioxus-config-macros` | Internal macros whose expansion depends on Dioxus feature flags (e.g. `maybe_wasm_split!`), for use by codegen. |
| `logger` | `dioxus-logger` | One `tracing` setup that works on web, desktop, mobile and server. |
| `generational-box` | `generational-box` | `Copy` handles to runtime-owned values, checked by generation; the storage under signals. |

## Core (14 crates)

The renderer-agnostic UI model. `dioxus-core` is the facade that re-exports this layer.

| Directory | Crate | What it is |
| --- | --- | --- |
| `core` | `dioxus-core` | Facade over the Core layer. Before 0.8 `dioxus-core` was the VirtualDOM itself (now `dioxus-vdom`). Signals, hooks, stores, HTML and document are default features; `ssr` and `wasm-split` are opt-in. |
| `vdom` | `dioxus-vdom` | The VirtualDOM: components, scopes, diffing, scheduling, events, suspense, and the `WriteMutations` trait renderers implement. |
| `vdom-macro` | `dioxus-vdom-macro` | `rsx!`, `#[component]` and `#[derive(Props)]`. |
| `vdom-template` | `dioxus-vdom-template` | The static `Template` that `rsx!` lowers each view into, plus its `const` builder. |
| `rsx` | `dioxus-rsx` | Parser and code generator for RSX, shared by the macro, hot-reload and autofmt. |
| `rsx-hotreload` | `dioxus-rsx-hotreload` | Diffs RSX between builds to produce template hot-reloads without recompiling. Versioned with Core but not exported from the facade. |
| `signals` | `dioxus-signals` | Reactive state: `Signal`, `Memo`, `ReadSignal`, global signals. |
| `hooks` | `dioxus-hooks` | Built-in hooks: `use_signal`, `use_effect`, `use_memo`, `use_resource`, `use_future`, etc. |
| `stores` | `dioxus-stores` | Stores: reactive nested data that lazily creates a signal per field for fine-grained subscriptions. |
| `stores-macro` | `dioxus-stores-macro` | `#[derive(Store)]` and `#[store]`. |
| `ssr` | `dioxus-ssr` | Render a VirtualDOM to an HTML string, with hydration markers. |
| `html` | `dioxus-html` | HTML and SVG elements, attributes and event types for `rsx!`. |
| `html-internal-macro` | `dioxus-html-internal-macro` | Internal macros that generate `dioxus-html`'s element, event and extension-attribute definitions. |
| `document` | `dioxus-document` | The `Document` trait: `eval`, and head elements (`Title`, `Meta`, `Link`, `Script`, `Style`, `Stylesheet`). |

## Core libraries (6 crates)

Libraries built on Core that are not tied to a specific renderer.

| Directory | Crate | What it is |
| --- | --- | --- |
| `devtools` | `dioxus-devtools` | App side of the `dx serve` connection: receives devserver messages and applies RSX hot-reloads and subsecond patches. |
| `devtools-types` | `dioxus-devtools-types` | Message types for that connection (`DevserverMsg`, `ClientMsg`, `HotReloadMsg`), shared with the CLI. |
| `history` | `dioxus-history` | The `History` trait the router navigates through, with an in-memory implementation; renderers provide their own. |
| `router` | `dioxus-router` | Type-safe router: `Router`, `Link`, `Outlet`, navigation, nested routes and layouts. |
| `router-macro` | `dioxus-router-macro` | `#[derive(Routable)]`. |
| `config-macro` | `dioxus-config-macro` | `server_only!`, `client!`, `web!`, `desktop!`, `native!`, `mobile!`, `fullstack!`, `ssr!`, `liveview!`: code that is kept or dropped by enabled platform features. |

## Framework (12 crates)

Renderers, fullstack, and the `dioxus` umbrella that ties them together with `launch`.

| Directory | Crate | What it is |
| --- | --- | --- |
| `dioxus` | `dioxus` | The umbrella crate apps depend on: re-exports Core, router, renderers and fullstack behind features, and provides `launch`/`LaunchBuilder`. |
| `fullstack-core` | `dioxus-fullstack-core` | Fullstack types, contexts and hooks (serialized futures/values, `use_server_future` support) for libraries that integrate with fullstack without pulling in the server. |
| `asset-resolver` | `dioxus-asset-resolver` | Resolves manganis assets at runtime per platform: HTTP on web, bundle paths or Android's AssetManager on native. |
| `web` | `dioxus-web` | Browser renderer for WASM, with hydration. |
| `desktop` | `dioxus-desktop` | Desktop and mobile renderer using a system webview (wry/tao). |
| `liveview` | `dioxus-liveview` | Server-side renderer that streams mutations to the browser over a WebSocket. |
| `native` | `dioxus-native` | GPU renderer using Blitz and Vello, with windowing. |
| `native-dom` | `dioxus-native-dom` | Headless core of the native renderer: applies VirtualDOM mutations to a Blitz DOM. |
| `interpreter` | `dioxus-interpreter-js` | JavaScript/TypeScript (Sledgehammer) that applies mutations to the DOM, used by web, desktop and liveview. |
| `fullstack` | `dioxus-fullstack` | Server functions and client/server coordination: the `#[server]`/`#[get]`/`#[post]` client side, encodings, websockets and streaming. |
| `fullstack-macro` | `dioxus-fullstack-macro` | The server-function macros. |
| `fullstack-server` | `dioxus-server` | Server side of fullstack on Axum: SSR with streaming and hydration, server-function routing, incremental static generation. |

## CLI (10 crates)

`dx` and the crates only it (or its tooling) uses. Nothing in the stack above depends on these.

| Directory | Crate | What it is |
| --- | --- | --- |
| `dx` | `dioxus-cli` | The `dx` binary: create, build, bundle, `serve` (devserver, hot-reload, subsecond), translate, components. |
| `cli-telemetry` | `dioxus-cli-telemetry` | Anonymous CLI telemetry event types and their wire format. |
| `dx-wire-format` | `dx-wire-format` | `dx`'s structured (JSON) output: build stages, artifacts and messages for tools driving `dx`. |
| `depinfo` | `depinfo` | Parser for rustc's `.d` dep-info files, used for hot-reload dependency tracking. |
| `component-manifest` | `dioxus-component-manifest` | Schema for component-library manifests read by `dx components`. |
| `wasm-split/wasm-split-cli` | `wasm-split-cli` | Splits a WASM binary into lazily loaded chunks at the `#[wasm_split]` points; used by `dx` when building for web. |
| `wasm-split/wasm-used` | `wasm-used` | A walrus `Used` analysis that records why items are kept, for debugging wasm-split output. |
| `autofmt` | `dioxus-autofmt` | Formatter for `rsx!` (`dx fmt` and the editor extension). |
| `rsx-rosetta` | `dioxus-rsx-rosetta` | Converts HTML to RSX (`dx translate`). |
| `extension` | `dioxus-ext` | The VS Code extension (TypeScript) plus its WASM helper crate for formatting and HTML-to-RSX. Not published. |

## Test-only (5 entries)

Not part of any layer and not published.

| Directory | Crate | What it is |
| --- | --- | --- |
| `oracle` | `dioxus-renderer-oracle` | In-memory mock renderer that validates VirtualDOM mutation streams, for tests and fuzzing. |
| `fuzz` | `dioxus-vdom-fuzz`, `dioxus-fuzz` | Structured VirtualDOM fuzzer and its `cargo-fuzz` target. |
| `subsecond/subsecond-tests` | | Subsecond hot-patching test crates. |
| `cli-harnesses` | | Test projects for `dx` builds across feature and platform setups. |
| `playwright-tests` | | End-to-end browser tests. |

# External repositories

Crates maintained by DioxusLabs outside this workspace. They build on the layers above but are
versioned and released separately.

## Blitz (16 crates)

[DioxusLabs/blitz](https://github.com/DioxusLabs/blitz): the modular HTML/CSS engine (Stylo for
style, Taffy for layout, Parley for text, AnyRender for paint) behind `dioxus-native`. The Blitz
workspace also builds its own copies of `dioxus-native` and `dioxus-native-dom` (0.7 line); the
ones listed under Framework are the current ones.

| Directory | Crate | What it is |
| --- | --- | --- |
| `packages/blitz` | `blitz` | High-level APIs for rendering HTML with Blitz. |
| `packages/blitz-dom` | `blitz-dom` | The DOM: tree, styling (Stylo), layout (Taffy), text (Parley), events and incremental updates. |
| `packages/blitz-html` | `blitz-html` | HTML parser (html5ever) that builds a `blitz-dom` document. |
| `packages/blitz-net` | `blitz-net` | Networking (HTTP, filesystem, data URIs), implementing `blitz-traits`' `NetProvider`. |
| `packages/blitz-paint` | `blitz-paint` | Paints a Blitz document through AnyRender. |
| `packages/blitz-shell` | `blitz-shell` | Event loop, windowing and system integration. |
| `packages/blitz-traits` | `blitz-traits` | Shared traits and types so the other Blitz crates avoid depending on each other. |
| `packages/blitz-vibey-cdp-server` | `blitz-vibey-cdp-server` | Chrome DevTools Protocol server so DevTools' Elements panel can inspect Blitz documents. |
| `packages/blitz-vibey-script` | `blitz-vibey-script` | JavaScript execution for Blitz on the Boa engine. Not published. |
| `packages/stylo_taffy` | `stylo_taffy` | Converts Stylo computed styles into Taffy layout styles. |
| `packages/accesskit_xplat` | `accesskit_xplat` | Cross-platform AccessKit adapter. |
| `packages/debug_timer` | `debug_timer` | Simple timing utilities. |
| `packages/parley_code_editor` | `parley_code_editor` | Code-editing control on floem-editor-core, Parley and AnyRender. Not published. |
| `apps/browser` | `browser` | The Blitz browser app. Not published. |
| `apps/browser/persistence` | `browser-persistence` | Saved state for the browser app. Not published. |
| `apps/readme` | `rdme` | Markdown renderer app. |

## Components (3 crates)

[DioxusLabs/dioxus-components](https://github.com/DioxusLabs/dioxus-components): accessible,
unstyled primitives plus a registry of 45 styled components (`component.json`) that
`dx components` installs into an app.

| Directory | Crate | What it is |
| --- | --- | --- |
| `primitives` | `dioxus-primitives` | Unstyled, accessible components: accordion, dialog, menus, select, calendar, toast, etc. |
| `dioxus-attributes` | `dioxus-attributes` | Proc macro for building dynamic attribute lists for Dioxus elements. |
| `preview` | `preview` | The styled components (under `preview/src/components`) and the gallery site that shows them. Not published. |

## SDK (8 crates)

[DioxusLabs/sdk](https://github.com/DioxusLabs/sdk): platform-agnostic hooks and utilities for
common app needs.

| Directory | Crate | What it is |
| --- | --- | --- |
| `packages/sdk` | `dioxus-sdk` | Entry point that re-exports the SDK crates behind features. |
| `packages/geolocation` | `dioxus-sdk-geolocation` | Geolocation hooks. |
| `packages/notification` | `dioxus-sdk-notification` | Send notifications from the app. |
| `packages/storage` | `dioxus-sdk-storage` | Local and persistent storage hooks. |
| `packages/sync` | `dioxus-sdk-sync` | Synchronization primitives such as channels. |
| `packages/time` | `dioxus-sdk-time` | Timers, intervals, debounce and timeouts. |
| `packages/util` | `dioxus-sdk-util` | General utilities such as scroll position. |
| `packages/window` | `dioxus-sdk-window` | Window size and theme hooks. |

## Code (3 crates)

[DioxusLabs/dioxus-code](https://github.com/DioxusLabs/dioxus-code): bundle-size-friendly syntax
highlighting.

| Directory | Crate | What it is |
| --- | --- | --- |
| `.` | `dioxus-code` | Syntax-highlighted code block component. |
| `dioxus-code-macro` | `dioxus-code-macro` | Compile-time highlighting macro, so no highlighter ships in the app. |
| `code-editor` | `dioxus-code-editor` | Syntax-highlighted code editor component. |

## Icons (2 crates)

[DioxusLabs/dioxus-icons](https://github.com/DioxusLabs/dioxus-icons): Lucide icons, one component
per icon.

| Directory | Crate | What it is |
| --- | --- | --- |
| `crates/dioxus-icons` | `dioxus-icons` | The icon components (`dioxus_icons::lucide::Trash`, etc.), generated from Lucide. |
| `crates/dioxus-icons-codegen` | `dioxus-icons-codegen` | Hand-run generator that regenerates the components from a pinned Lucide version. Not published. |

## Testing (1 crate)

[DioxusLabs/dioxus-test](https://github.com/DioxusLabs/dioxus-test): experimental testing library.

| Directory | Crate | What it is |
| --- | --- | --- |
| `.` | `dioxus-test` | Renders a Dioxus app headlessly on the native renderer so tests can interact with and query the DOM, with control over render lifecycle and async work. |

## Docsite (21 crates)

[DioxusLabs/docsite](https://github.com/DioxusLabs/docsite): the dioxuslabs.com site, with the
guide for each release, the blog, the playground and search.

| Directory | Crate | What it is |
| --- | --- | --- |
| `packages/docsite` | `dioxus_docs_site` | The website itself. |
| `packages/docs-03` … `packages/docs-07` | `dioxus-docs-03` … `dioxus-docs-07` | The guide for each release (0.3 to 0.7), compiled from `docs-src/<version>` mdbooks. Not published. |
| `packages/docs-blog` | `dioxus-docs-blog` | Blog posts, compiled from `docs-src/blog`. Not published. |
| `packages/docs-router` | `dioxus-docs-examples` | Runnable code examples embedded in the guide. |
| `packages/include_mdbook/packages/use-mdbook` | `use-mdbook` | `use_mdbook` hook and `include_mdbook!` for loading an mdbook's contents at compile time. |
| `packages/include_mdbook/packages/mdbook-macro` | `mdbook-macro` | The `include_mdbook!` proc macro. |
| `packages/include_mdbook/packages/mdbook-gen` | `mdbook-gen` | Build-script codegen that turns an mdbook into Dioxus routes and components. |
| `packages/include_mdbook/packages/mdbook-shared` | `mdbook-shared` | mdbook types shared by the crates above. |
| `packages/notion-to-blog` | `notion-to-blog` | Converts Notion exports into blog posts. Not published. |
| `packages/playground/playground` | `dioxus-playground` | The playground component library: an editor that builds and runs Dioxus code in the browser. |
| `packages/playground/server` | `server` | Axum server that queues and compiles submitted code to WASM. |
| `packages/playground/model` | `model` | Types shared by the playground client and server. |
| `packages/playground/runner` | `runner` | Wraps the playground component to run it. |
| `packages/playground/example-projects` | `example-projects` | The snippets the playground offers, with their index. |
| `packages/search/search` | `dioxus-search` | Prebuilt search index over a site's static router routes. |
| `packages/search/search-macro` | `dioxus-search-macro` | Macro that loads the search index. |
| `packages/search/search-shared` | `dioxus-search-shared` | Types shared by the search crates. |

## Template (4 templates)

[DioxusLabs/dioxus-template](https://github.com/DioxusLabs/dioxus-template): the cargo-generate
templates that `dx new` uses; not meant to be cloned directly. It holds no library crates, just one
template project per platform: `Web`, `Desktop`, `Fullstack` and `Liveview`.
