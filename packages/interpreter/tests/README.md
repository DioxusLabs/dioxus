# Interpreter subtree cleanup regression tests

These tests run without an application server or the Dioxus CLI. They exercise real Chromium and WebKit DOMs, observers, event listeners, hydration bindings, and mounted queues.

From the repository root, install the existing browser-test dependencies and browser engines once:

```sh
npm ci --prefix packages/playwright-tests
packages/playwright-tests/node_modules/.bin/playwright install chromium webkit
```

Run the suite using those dependencies:

```sh
NODE_PATH="$PWD/packages/playwright-tests/node_modules" \
  node packages/playwright-tests/node_modules/@playwright/test/cli.js \
  test --config packages/interpreter/tests/playwright.config.cjs
```

Bun must be on `PATH` when TypeScript sources need regeneration. The ordinary interpreter build script regenerates checked-in bundles and their source hash; no custom bundling step is used:

```sh
cargo check -p dioxus-interpreter-js --features webonly,binary-protocol
cargo check -p dioxus-web --target wasm32-unknown-unknown --features hydrate
```

Global setup compiles and runs the host-only `cleanup-fixture` example. It exports the actual generated binary interpreter, native interpreter bundle, and Rust-serialized mutation streams under `target/interpreter-cleanup-fixtures`. Playwright artifacts go under `target/interpreter-cleanup-results`.

The first 15 cases exercise the generated base interpreter's 0.8 stack operations directly. The other nine cases execute the emitted binary interpreter against mutation bytes produced by the Rust interpreter. Together, the 24 cases cover full subtree retirement; HTML, SVG, text, and comment bindings; aliases and reused IDs; moved descendants and keyed-style moves; self-retaining replacements; bound template prototypes and pending clone preservation; duplicate/direct/delegated listeners; observer unregistration and stale notifications; markerless hydration binding; and mounted queue identity after `set_id`.

The suite runs all 24 cases in both Chromium and WebKit.

These focused tests do not replace application-level Wasm tests of wasm-bindgen transport or streamed Suspense. Run existing mounted-event, hydration-order, and suspense-carousel fixtures when validating changes to those framework lifecycle paths.
