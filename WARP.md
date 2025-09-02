# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Common Development Commands

### Setup and Installation
```bash
# Install Rust toolchain and targets
rustup toolchain install stable
rustup target add wasm32-unknown-unknown

# Install Dioxus CLI (latest development version)
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli --locked

# Install Node.js dependencies for Playwright tests
cd packages/playwright-tests
npm ci
npx playwright install --with-deps
```

### Building and Running
```bash
# Run examples (50+ available)
cargo run --example hello_world

# Run with hot-reloading using dx CLI
dx serve

# Run specific example with web platform
dx serve --example calculator --platform web -- --no-default-features

# Build for release
dx bundle --release

# Build workspace
cargo build --workspace --all-features
```

### Code Quality
```bash
# Format code
cargo fmt --all

# Format RSX syntax
dx fmt

# Check formatting (CI)
cargo fmt --all -- --check

# Lint code
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Type check
cargo check --workspace --all-features
```

### Testing
```bash
# Run all tests (excludes desktop and mobile)
cargo test --workspace --exclude dioxus-desktop --exclude dioxus-mobile

# Run tests for specific package
cargo test -p dioxus-core

# Run Playwright integration tests
cd packages/playwright-tests && npm test

# Run tests with release optimizations
cargo test --workspace --exclude dioxus-desktop --exclude dioxus-mobile --profile release-unoptimized
```

### Documentation
```bash
# Generate documentation
cargo doc --workspace --no-deps --all-features --document-private-items
```

## High-level Architecture

Dioxus is a React-like UI framework for Rust that enables building cross-platform applications with a single codebase. The architecture is built around these core concepts:

### Core Components
- **Virtual DOM (`packages/core/`)**: Renderer-agnostic virtual DOM implementation that powers all platforms
- **RSX Macros (`packages/rsx/`, `packages/core-macro/`)**: Custom syntax for declaring UI similar to JSX
- **State Management**: 
  - **Hooks (`packages/hooks/`)**: React-like state management (use_state, use_memo, use_effect, etc.)
  - **Signals (`packages/signals/`)**: Fine-grained reactive state management
- **Component System**: UI components are Rust functions returning `Element`

### Platform Renderers
- **Web (`packages/web/`)**: WebAssembly renderer for browsers
- **Desktop (`packages/desktop/`)**: Native desktop apps using WebView
- **Mobile (`packages/mobile/`)**: iOS and Android support
- **SSR (`packages/ssr/`)**: Server-side rendering
- **LiveView (`packages/liveview/`)**: Server-driven UI updates

### Fullstack Web Framework
- **Fullstack (`packages/fullstack/`)**: Complete web framework with SSR and hydration
- **Router (`packages/router/`)**: Type-safe client and server routing
- **Server Functions**: Type-safe RPC between client and server using `#[server]` macro

### Development Tools
- **CLI (`packages/cli/`)**: `dx` command for building, serving, bundling, and hot-reloading
- **AutoFormat (`packages/autofmt/`)**: RSX code formatter
- **Hot Reloading (`packages/rsx-hotreload/`)**: Live code updates during development

### Architecture Flow
```
RSX Components → Virtual DOM → Platform Renderers
     ↓              ↓              ↓
State (Hooks/     Diffing &      Web/Desktop/
Signals)         Mutations       Mobile/SSR
```

## Platform-Specific Development

### Web (WASM) Development
```bash
# Prerequisites
rustup target add wasm32-unknown-unknown

# Local development with hot-reloading
dx serve --platform web

# Production build
dx build --platform web --release
dx bundle --platform web

# Testing
cargo test -p dioxus-web
cd packages/playwright-tests && npx playwright test
```

### Desktop Development
```bash
# Local development
dx serve --platform desktop
# or run examples directly
cargo run --example calculator

# Production packaging
dx bundle --platform desktop --release

# Testing
cargo test -p dioxus-desktop
```

### SSR/Fullstack Development
```bash
# Run fullstack examples
cargo run -p fullstack-hello-world

# Testing server functions and routing
cargo test -p dioxus-fullstack
cargo test -p dioxus-router
```

### Mobile Development
```bash
# Prerequisites: Android SDK or Xcode installed
# Build for mobile platforms
dx build --platform mobile

# Run on device/emulator
dx serve --platform android
dx serve --platform ios
```

## Key Files and Information

### Configuration Files
- `Cargo.toml`: Workspace configuration with 70+ packages
- `Dioxus.toml`: Project-specific configuration for dx CLI
- `.github/workflows/main.yml`: CI pipeline defining canonical commands

### Important Packages
- `packages/dioxus/`: Main umbrella crate with feature flags
- `packages/core/`: Virtual DOM implementation
- `packages/rsx/`: RSX macro parser and implementation
- `packages/cli/`: Dioxus CLI tool source
- `packages/web/`, `packages/desktop/`, `packages/mobile/`: Platform renderers
- `packages/fullstack/`: Web framework with SSR
- `packages/router/`: Type-safe routing

### Development Version
- Current development version: `0.7.0-alpha.2`
- Minimum Supported Rust Version (MSRV): `1.80.0`

## Examples and Learning

The repository contains extensive examples:
- `examples/`: 50+ examples covering all features
- `example-projects/`: Full applications (e.g., HackerNews clone, file explorer)
- Run any example: `cargo run --example <name>`
- Web examples: `dx serve --example <name> --platform web -- --no-default-features`

## Troubleshooting

### Common Issues
- **Build failures**: Ensure all targets installed with `rustup target add wasm32-unknown-unknown`
- **Playwright test failures**: Install browsers with `npx playwright install --with-deps`
- **Linux dependencies**: Install webkit dependencies:
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libasound2-dev libudev-dev libayatana-appindicator3-dev libxdo-dev libglib2.0-dev
  ```
- **Cache issues**: Clear with `cargo clean` and `rm -rf target/dx`

### Platform-Specific Notes
- **macOS**: Requires Xcode for native dependencies
- **Windows**: May need Visual Studio Build Tools
- **Linux**: Additional system dependencies required (see CI configuration)

## Testing Strategy

- **Unit tests**: `cargo test --workspace` (most packages)
- **Integration tests**: Playwright tests for web functionality
- **Cross-platform**: CI tests on Ubuntu, macOS, and Windows
- **Multiple Rust versions**: Stable, beta, and MSRV testing
- **Platform matrix**: Tests desktop, web, mobile, and server targets

The project uses a comprehensive CI pipeline that tests across multiple platforms, Rust versions, and feature combinations to ensure compatibility and stability.
