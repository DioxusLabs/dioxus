# New app

To get started, let's create a new Rust project for our Dog Search Engine.

```shell
$ cargo new --bin doggo
$ cd doggo
```

Make sure our project builds by default

```shell
$ cargo run

   Compiling doggo v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 0.41s
     Running `target/debug/doggo`
Hello, world!
```

## Adding Dioxus Desktop as a dependency

We can either edit our Cargo.toml directly:

```toml
[dependencies]
dioxus = { version = "*", features = ["desktop"]}
```

or use `cargo-edit` to add it via the CLI:

```shell
$ cargo add dioxus --features desktop
```

## Setting up a hello world

Let's edit the project's `main.rs` and add the skeleton of 

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div { "hello world!" }
    ))
}
```


## Making sure things run





