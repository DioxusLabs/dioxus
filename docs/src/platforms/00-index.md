# Welcome to Dioxus!

## Running Examples

We use the dedicated `dioxus-cli` to build and test dioxus web-apps. This can run examples, tests, build web workers, launch development servers, bundle, and more. It's general purpose, but currently very tailored to Dioxus for liveview and bundling. If you've not used it before, `cargo install --path pacakages/dioxus-cli` will get it installed. This CLI tool should feel like using `cargo` but with 1st party support for assets, bundling, and other important dioxus-specific features.

Alternatively, `trunk` works but can't run examples.

- tide_ssr: Handle an HTTP request and return an html body using the html! macro. `cargo run --example tide_ssr`
- doc_generator: Use dioxus SSR to generate the website and docs. `cargo run --example doc_generator`
- fc_macro: Use the functional component macro to build terse components. `cargo run --example fc_macro`
- hello_web: Start a simple wasm app. Requires a web packer like dioxus-cli or trunk `cargo run --example hello`
- router: `cargo run --example router`
- tide_ssr: `cargo run --example tide_ssr`
- webview: Use liveview to bridge into a webview context for a simple desktop application. `cargo run --example webview`
- twitter-clone: A full-featured Twitter clone showcasing dioxus-liveview, state management patterns, and hooks. `cargo run --example twitter`
