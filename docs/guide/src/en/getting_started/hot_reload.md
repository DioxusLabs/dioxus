# Setting Up Hot Reload

1. Hot reloading allows much faster iteration times inside of rsx calls by interpreting them and streaming the edits.
2. It is useful when changing the styling/layout of a program, but will not help with changing the logic of a program.
3. Currently the cli only implements hot reloading for the web renderer. For TUI, desktop, and LiveView you can use the hot reload macro instead.

# Web

For the web renderer, you can use the dioxus cli to serve your application with hot reloading enabled.

## Setup

Install [dioxus-cli](https://github.com/DioxusLabs/cli).
Hot reloading is automatically enabled when using the web renderer on debug builds.

## Usage

1. Run:
```bash 
dioxus serve --hot-reload
```
2. Change some code within a rsx or render macro
3. Open your localhost in a browser
4. Save and watch the style change without recompiling

# Desktop/Liveview/TUI

For desktop, LiveView, and tui, you can place the hot reload macro at the top of your main function to enable hot reloading.
Hot reloading is automatically enabled on debug builds.

## Setup

Add the following to your main function:

```rust
fn main() {
    hot_reload_init!();
    // launch your application
}
```

## Usage
1. Run:
```bash
cargo run
```
2. Change some code within a rsx or render macro
3. Save and watch the style change without recompiling

# Limitations
1. The interpreter can only use expressions that existed on the last full recompile. If you introduce a new variable or expression to the rsx call, it will trigger a full recompile to capture the expression.
2. Components and Iterators can contain arbitrary rust code and will trigger a full recompile when changed.
