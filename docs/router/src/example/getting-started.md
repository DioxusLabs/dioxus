# Getting Started
Before we start utilizing Dioxus Router, we need to initialize a Dioxus web application.

## Required Tools
If you haven't already, make sure you install the
[dioxus-cli](https://dioxuslabs.com/nightly/cli/) build tool and the rust
`wasm32-unknown-unknown` target:
```sh
$ cargo install dioxus-cli
$ rustup target add wasm32-unkown-unknown
```

## Creating the Project
First, create a new cargo binary project:
```sh
$ cargo new --bin dioxus-blog
```

Next, we need to add dioxus with the web and router feature to our `Cargo.toml`
file:
```toml
[package]
name = "dioxus-blog"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.3.0", features = ["web", "router"] }
```

Now we can start coding! Open `src/main.rs` and replace its contents with:
```rust
# // Hidden lines (like this one) make the documentation tests work.
# extern crate dioxus;
use dioxus::prelude::*;

fn main() {
    // Launch Dioxus web app
    # // deliberately impossible
    # #[cfg(all(debug_assertions, not(debug_assertions)))]
    dioxus::web::launch(app);
    # let mut vdom = VirtualDom::new(App);
    # vdom.rebuild();
    # assert_eq!(
    #     dioxus::ssr::render_vdom(&vdom),
    #     "<p>Hello, wasm!</p>"
    # )
}

// Our root component.
#[allow(non_snake_case)]
fn App(cx: Scope) -> Element {
    // Render "Hello, wasm!" to the screen.
    cx.render(rsx! {
        p { "Hello, wasm!"}
    })
}
```

Our project is now setup! To make sure everything is running correctly, in the
root of your project run:
```sh
dioxus serve --platform web
```
Then head to [http://localhost:8080](http://localhost:8080) in your browser, and
you should see ``Hello, wasm!`` on your screen.

## Conclusion
We setup a new project with Dioxus and got everything running correctly. Next
we'll create a small homepage and start our journey with Dioxus Router.
