# Build

From the cli-plugin directory

```sh
rustup target add wasm32-wasi
cargo install wasm-tools

cd ./examples/tailwind
cargo build --target wasm32-wasi
cd ..
wasm-tools component new ../../../target/wasm32-wasi/debug/dioxus_cli_plugin_test.wasm --adapt ../wasi_snapshot_preview1.reactor.wasm -o output.wasm
cargo test --package dioxus-cli --test plugins
cd ..
```
