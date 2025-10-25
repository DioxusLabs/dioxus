# Dioxus Codebase Guide for AI Agents

## Project Overview

Dioxus is a cross-platform reactive UI framework for Rust that supports web, desktop, mobile, server-side rendering, and more. The codebase is organized as a Cargo workspace with ~50 packages under `packages/`, each with specific responsibilities.

## Architecture

### Core Packages
- **`packages/core`**: The VirtualDom implementation - the heart of Dioxus. All rendering platforms build on this.
- **`packages/rsx`**: RSX macro DSL parser and syntax tree. Used by `rsx!`, `rsx_rosetta`, and the autoformatter.
- **`packages/signals`**: Copy-based reactive state with local subscriptions (`use_signal`, `use_memo`).
- **`packages/hooks`**: Standard hooks like `use_state`, `use_effect`, `use_resource`.
- **`packages/html`**: HTML elements, attributes, and events. Auto-generated from MDN docs.
- **`packages/dioxus`**: The main facade crate that re-exports everything for end users.

### Platform Renderers
- **`packages/web`**: WebAssembly renderer using `web-sys` and the interpreter
- **`packages/desktop`**: Webview-based desktop apps using `wry` and `tao`
- **`packages/mobile`**: Mobile platform support (iOS/Android) via webview
- **`packages/liveview`**: Server-side rendering with live updates over WebSockets
- **`packages/ssr`**: Static HTML generation
- **`packages/native` + `packages/native-dom`**: Experimental WGPU-based native renderer (Blitz integration)

### Fullstack System
- **`packages/fullstack`**: RPC framework for server functions (wraps `axum`)
- **`packages/fullstack-core`**: Core types shared between client/server
- **`packages/fullstack-macro`**: `#[server]`, `#[get]`, `#[post]` macros for server functions
- **`packages/router`**: Type-safe routing with `#[derive(Routable)]`

### Developer Tooling
- **`packages/cli`**: The `dx` CLI for building, serving, and bundling apps
- **`packages/cli-config`**: Environment variables and configuration read by apps at dev/build time
- **`packages/autofmt`**: Code formatter for RSX (used by VS Code extension)
- **`packages/check`**: Static analysis for RSX macros
- **`packages/rsx-hotreload`**: Hot-reloading infrastructure for RSX and assets

## Key Conventions

### Component Pattern
Components are functions returning `Element` (alias for `Option<VNode>`):

```rust
use dioxus::prelude::*;

// Simple component
fn MyComponent() -> Element {
    rsx! { div { "Hello!" } }
}

// With props
#[component]
fn Greeting(name: String) -> Element {
    rsx! { "Hello, {name}!" }
}
```

The `#[component]` macro is optional but enables nicer prop ergonomics.

### State Management
- Use `use_signal` for local reactive state (Copy, automatically subscribes components on read)
- Use `use_memo` for derived computations
- Use `use_context_provider`/`use_context` for dependency injection
- Signals only trigger re-renders when read **inside the component body**, not in event handlers or futures

### Server Functions
Server functions use `#[get]` or `#[post]` macros (preferred) or `#[server]`:

```rust
#[post("/api/user/{id}")]
async fn update_user(id: u32, body: UserData) -> Result<User> {
    // Runs on server, callable from client
}
```

- Arguments can be path params, query params, JSON body, or Axum extractors
- Server-only extractors go after the path: `#[post("/api/foo", auth: AuthToken)]`
- All server functions auto-register unless they require custom `State<T>` (use `ServerFnState` layer)

### RSX Syntax
```rust
rsx! {
    div { class: "container",
        h1 { "Title" }
        button { onclick: move |_| count += 1, "Click me" }
        for item in items {
            li { key: "{item.id}", "{item.name}" }
        }
        if show_modal {
            Modal {}
        }
    }
}
```

- Use `key` attribute for lists to optimize diffing
- Event handlers can be closures or function pointers
- Interpolation: `"{variable}"` or `{some_expr()}`

## Development Workflows

### Running Examples
```bash
# With cargo (desktop only)
cargo run --example hello_world

# With CLI (supports hot-reload, web platform)
dx serve --example hello_world
dx serve --example hello_world --platform web -- --no-default-features

# Mobile
dx serve --platform android
dx serve --platform ios
```

### Testing
```bash
# Run workspace tests (excludes desktop on Linux due to display requirements)
cargo test --lib --bins --tests --examples --workspace --exclude dioxus-desktop

# Test with release optimizations disabled (faster, checks production paths)
cargo test --workspace --profile release-unoptimized

# Linux: Install GTK dependencies first
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libasound2-dev
```

### CLI Usage
- **`dx new`**: Create new project from template
- **`dx serve`**: Dev server with hot-reload (RSX + assets + experimental Rust hot-patching with `--hotpatch`)
- **`dx bundle`**: Production build with optimizations (wasm compression, asset optimization, minification)
- **`dx build`**: Build without bundling
- Install: `cargo install dioxus-cli` or `cargo binstall dioxus-cli@0.7.0-rc.3`

### Configuration
Projects use `Dioxus.toml` for CLI config:
```toml
[application]
name = "my-app"
default_platform = "web"  # or "desktop"
public_dir = "public"     # Static assets

[web.app]
title = "My App"
```

Apps read CLI-set env vars via `dioxus-cli-config` (e.g., `fullstack_address_or_localhost()`).

## Critical Implementation Details

### VirtualDom Lifecycle
1. `VirtualDom::new(app)` - Create with root component
2. `rebuild_to_vec()` or `rebuild()` - Initial render produces `Mutations`
3. `wait_for_work()` - Async wait for signals/events
4. `handle_event()` - Process user events
5. `render_immediate()` - Apply mutations to real DOM

### Hotreload Architecture
- `rsx-hotreload` crate detects RSX changes and sends diffs to running app
- Uses file watching + AST diffing to minimize reload scope
- Works across all platforms (web, desktop, mobile)
- Rust code hot-patching is experimental via `--hotpatch` flag

### Workspace Dependencies
- All versions pinned to `=0.7.0-rc.3` in workspace
- Version bumps require updating `[workspace.package]` AND `[workspace.dependencies]`
- Use workspace dependencies, not path/git dependencies in published crates

### Testing Patterns
- Unit tests live in `tests/` folders within packages
- Integration tests in `packages/playwright-tests/` (E2E via Playwright)
- Full-project examples in `examples/01-app-demos/*/` are also workspace members

## Common Pitfalls

1. **Signal reads in handlers don't subscribe**: Only reads in component body trigger re-renders
2. **Missing `key` in lists**: Without keys, list reconciliation is inefficient
3. **Forgetting `#[component]`**: Props structs need `#[derive(Props, Clone, PartialEq)]` without it
4. **Server function errors**: Use `Result<T>` return type with appropriate error handling
5. **Platform features**: Examples default to `desktop` - use `--no-default-features` for web

## Release Process

See `notes/RELEASING.md` for the full 50+ step checklist. Key points:
- Manual version bumps across all `workspace.dependencies`
- Use `cargo workspaces publish` for coordinated release
- Verify docs.rs builds before GitHub release
- CLI published via GitHub Actions with binstall support

## Documentation Standards

- All public APIs documented with MDN-style docs (see `packages/html`)
- Examples required for complex features
- Docsite at https://dioxuslabs.com runs on Dioxus itself (dogfooding)
- Use `#[doc(cfg(...))]` for platform-specific APIs

## Contributing

- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace`
- Docs: `cargo doc --workspace --no-deps --all-features`
- CI uses nightly Rust for docs generation
- MSRV: 1.85.0 (checked in CI with `cargo-msrv`)
