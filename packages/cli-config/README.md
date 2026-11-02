# dioxus-cli-config

Dioxus-specific settings that `dx` passes to Dioxus apps at runtime, such as the window title and whether the window floats on top during development.

The generic environment variable contract between `dx` and the app it launches (devserver address, server address, base path, build id, ...) lives in [`dx-env`](https://crates.io/crates/dx-env), which this crate re-exports.
