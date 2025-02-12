# This folder contains the wasm-split sub-workspace

wasm-split is a tool that allows you to split a wasm binary into multiple chunks that are lazily loaded on demand.

This workspace is comprised of:
- the harness on which we test against
- the wasm-split user-facing crate
- the wasm-split-macro crate which is used to generate the wasm-split loader
- the wasm-split-cli crate which is imported by the dioxus-cli and used when building the wasm modules
- the wasm-used crate which provides a custom walrus `Used` struct that makes it easier to debug why a emit_wasm might be failing
