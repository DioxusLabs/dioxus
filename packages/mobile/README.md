# Getting started: mobile


Dioxus is unique in that it actually supports mobile. However, support is very young and you might need to dip down into some of the primitives until better supported is ready.

Currently, only iOS is supported through us, however you *can* add android support by following the same instructions below, but using the `android` guide in `cargo-mobile`.

Also, Dioxus Desktop and Dioxus Mobile share the same codebase, and dioxus-mobile currently just re-exports dioxus-desktop.

## Getting Set up

Getting set up with mobile can but quite challenging. The tooling here isn't great (yet) and might take some hacking around to get things working. macOS M1 is broadly unexplored and might not work for you.

We're going to be using `cargo-mobile` to build for mobile. First, install it:

```shell
$ cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```


And then initialize your app for the right platform. Use the `winit` template for now. Right now, there's no "Dioxus" template in cargo-mobile.

```shell
$ cargo moble init 
```

We're going to completely clear out the `dependencies` it generates for us, swapping out `winit` with `dioxus-mobile`.

```toml

[package]
name = "dioxus-ios-demo"
version = "0.1.0"
authors = ["Jonathan Kelley <jkelleyrtp@gmail.com>"]
edition = "2018"


# leave the `lib` declaration
[lib]
crate-type = ["staticlib", "cdylib", "rlib"]


# leave the binary it generates for us
[[bin]]
name = "dioxus-ios-demo-desktop"
path = "gen/bin/desktop.rs"

# clear all the dependencies
[dependencies]
mobile-entry-point = "0.1.0"
dioxus = { version = "*", features = ["mobile"] }
simple_logger = "*"
```

Edit your `lib.rs`:

```rust
// main.rs
use dioxus::prelude::*;

fn main() {
    dioxus::mobile::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```

To configure the webview, menubar, and other important desktop-specific features, checkout out some of the launch configuration in the [API reference](https://docs.rs/dioxus-mobile/).

## Future Steps

Make sure to read the [Dioxus Guide](https://dioxuslabs.com/guide) if you already haven't!
