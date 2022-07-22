# Mobile App

Build a mobile app with Dioxus!

Example: [Todo App](https://github.com/DioxusLabs/example-projects/blob/master/ios_demo)

## Support
Mobile is currently the least-supported renderer target for Dioxus. Mobile apps are rendered with the platform's WebView, meaning that animations, transparency, and native widgets are not currently achievable.

In addition, iOS is the only supported Mobile Platform. It is possible to get Dioxus running on Android and rendered with WebView, but the Rust windowing library that Dioxus uses – tao – does not currently support Android.

Mobile support is currently best suited for CRUD-style apps, ideally for internal teams who need to develop quickly but don't care much about animations or native widgets.

## Getting Set up

Getting set up with mobile can be quite challenging. The tooling here isn't great (yet) and might take some hacking around to get things working. macOS M1 is broadly unexplored and might not work for you.

We're going to be using `cargo-mobile` to build for mobile. First, install it:

```shell
cargo install --git https://github.com/BrainiumLLC/cargo-mobile
```

And then initialize your app for the right platform. Use the `winit` template for now. Right now, there's no "Dioxus" template in cargo-mobile.

```shell
cargo mobile init
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
use dioxus::prelude::*;

fn main() {
    dioxus_mobile::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!{
        div {
            "hello world!"
        }
    })
}
```