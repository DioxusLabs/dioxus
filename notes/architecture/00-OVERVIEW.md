# Dioxus Architecture Overview

This directory contains comprehensive architecture documentation for the Dioxus framework, designed to help future Claude agents (and human contributors) understand the codebase deeply.

## Document Index

| File | Description |
|------|-------------|
| `01-CORE.md` | VirtualDOM, rendering pipeline, component system, diffing algorithm |
| `02-CLI.md` | Build system, dev server, bundling, terminal UI, platform support |
| `03-RSX.md` | RSX macro parsing, AST types, code generation, autofmt |
| `04-SIGNALS.md` | Signals, generational-box, hooks, stores, state management |
| `05-FULLSTACK.md` | Server functions, SSR, hydration, client-server communication |
| `06-RENDERERS.md` | Web, desktop, native, liveview renderer implementations |
| `07-HOTRELOAD.md` | Subsecond hot-patching, RSX hot-reload, devtools protocol |
| `08-ASSETS.md` | Manganis asset system, const serialization, build-time processing |
| `09-ROUTER.md` | Routing system, Routable trait, navigation, history |
| `10-WASM-SPLIT.md` | WASM code splitting, lazy loading, chunk management |

## Crate Dependency Overview

```
dioxus (umbrella crate)
├── dioxus-core (VirtualDOM, runtime, scheduler)
│   └── generational-box (Copy semantics for references)
├── dioxus-rsx (macro parsing)
│   └── dioxus-autofmt (code formatting)
├── dioxus-signals (reactive state)
│   ├── generational-box
│   └── dioxus-stores (nested reactivity)
├── dioxus-hooks (use_signal, use_effect, etc.)
├── dioxus-html (HTML elements, events)
├── dioxus-router (client-side routing)
│
├── Renderers:
│   ├── dioxus-web (WASM/browser)
│   ├── dioxus-desktop (wry/tao webview)
│   ├── dioxus-native (blitz/vello GPU)
│   ├── dioxus-liveview (WebSocket streaming)
│   └── dioxus-ssr (server-side rendering)
│
├── Fullstack:
│   ├── dioxus-fullstack (client/server coordination)
│   ├── dioxus-fullstack-core (transport, types)
│   ├── dioxus-fullstack-macro (#[server] macro)
│   └── dioxus-server (Axum integration)
│
├── CLI (dx command):
│   ├── dioxus-cli (main binary)
│   ├── dioxus-cli-config (Dioxus.toml parsing)
│   └── dioxus-cli-opt (optimization passes)
│
├── Hot Reload:
│   ├── subsecond (hot-patching runtime)
│   ├── dioxus-rsx-hotreload (template diffing)
│   └── dioxus-devtools (WebSocket communication)
│
├── Assets:
│   ├── manganis (asset!() macro)
│   ├── manganis-core (asset types)
│   ├── manganis-macro (proc-macro)
│   └── const-serialize (compile-time CBOR)
│
└── Code Splitting:
    ├── wasm-splitter (CLI tool)
    ├── wasm-split-macro (#[wasm_split])
    └── wasm-split-cli (binary processing)
```

## Key Architectural Patterns

### 1. Copy Semantics for Reactive State
Dioxus uses `generational-box` to provide `Copy` semantics for references. This enables signals and other reactive primitives to be freely passed around without explicit cloning.

### 2. WriteMutations Trait
All renderers implement `WriteMutations` which defines how VirtualDOM changes translate to real DOM operations. This abstraction enables the same component code to run on web, desktop, mobile, and server.

### 3. Template-Based Rendering
RSX macros generate static `Template` definitions at compile time. Only dynamic parts are diffed at runtime, making updates extremely efficient.

### 4. Hot-Reload Without Full Rebuild
Two complementary systems:
- **RSX Hot-Reload**: Changes to template literals are diffed and sent via WebSocket
- **Subsecond Hot-Patching**: Full Rust function replacement via jump table indirection

### 5. Compile-Time Asset Processing
The `asset!()` macro embeds asset metadata in binary link sections. The CLI extracts this at build time, processes assets, and patches the binary with final URLs.

### 6. Server Functions as RPC
The `#[server]` macro generates dual code paths - client serialization and server handler - enabling seamless RPC-style communication.

## Important Design Decisions

1. **Single VirtualDOM**: Even multi-window desktop apps share one VirtualDOM for simpler state management
2. **Scope-Based Cleanup**: All state (signals, tasks, contexts) is tied to component scope lifetime
3. **Platform Agnostic Events**: Events are converted through `HtmlEventConverter` trait for cross-platform support
4. **Binary Protocols**: Desktop and liveview use Sledgehammer binary format for compact mutation encoding
5. **Generational GC**: References are invalidated by generation counters, preventing use-after-free without runtime overhead per access

## Future Development Areas

These docs are designed to help with planned features:
- **Custom Bundlers**: Replacing tauri-bundler with native implementation
- **CLI Tunnels**: Remote device hot-reloading via SSH/SCP
- **Workspace Hot-Patching**: Patching library crates, not just the tip crate
- **Universal Android Builds**: Single APK for all architectures
- **Fullstack Extensions**: First-party database, auth, IAAC, secrets
