# `dioxus-hot-reload`: Hot Reloading Utilites for Dioxus

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Discord chat][discord-badge]][discord-url]

[crates-badge]: https://img.shields.io/crates/v/dioxus-hot-reload.svg
[crates-url]: https://crates.io/crates/dioxus-hot-reload
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/dioxuslabs/dioxus/blob/master/LICENSE
[actions-badge]: https://github.com/dioxuslabs/dioxus/actions/workflows/main.yml/badge.svg
[actions-url]: https://github.com/dioxuslabs/dioxus/actions?query=workflow%3ACI+branch%3Amaster
[discord-badge]: https://img.shields.io/discord/899851952891002890.svg?logo=discord&style=flat-square
[discord-url]: https://discord.gg/XgGxMSkvUM

[Website](https://dioxuslabs.com) |
[Guides](https://dioxuslabs.com/learn/0.4/) |
[API Docs](https://docs.rs/dioxus-hot-reload/latest/dioxus_hot_reload) |
[Chat](https://discord.gg/XgGxMSkvUM)

## Overview

Dioxus supports hot reloading for static parts of rsx macros. This enables changing the styling of your application without recompiling the rust code. This is useful for rapid iteration on the styling of your application.

Hot reloading could update the following change without recompiling:

```rust
rsx! {
    div {
        "Count: {count}",
    }
}
```

=>

```rust
rsx! {
    div {
        color: "red",
        font_size: "2em",
        "Count: {count}",
    }
}
```

But it could not update the following change:

```rust
rsx! {
    div {
        "Count: {count}",
    }
}
```

=>

```rust
rsx! {
    div {
        "Count: {count*2}",
        onclick: |_| println!("clicked"),
    }
}
```

## Usage

> This crate implements hot reloading for native compilation targets not WASM. For hot relaoding with the web renderer, see the [dioxus-cli](https://github.com/DioxusLabs/dioxus/tree/master/packages/cli) project.

Add this to the top of your main function on any renderer that supports hot reloading to start the hot reloading server:

```rust
fn main(){
    hot_reload_init!();
    // launch your application
}
```

By default the dev server watches on the root of the crate the macro is called in and ignores changes in the `/target` directory and any directories set in the `.gitignore` file in the root directory. To watch on custom paths pass call the `with_paths` function on the config builder:

```rust
fn main(){
    hot_reload_init!(Config::new().with_paths(&["src", "examples", "assets"]));
    // launch your application
}
```

By default the hot reloading server will output some logs in the console, to disable these logs call the `with_logging` function on the config builder:

```rust
fn main(){
    hot_reload_init!(Config::new().with_logging(false));
    // launch your application
}
```

To rebuild the application when the logic changes, you can use the `with_rebuild_command` function on the config builder. This command will be called when hot reloading fails to quickly update the rsx:

```rust
fn main(){
    hot_reload_init!(Config::new().with_rebuild_command("cargo run"));
    // launch your application
}
```

If you are using a namespace other than html, you can implement the [HotReloadingContext](https://docs.rs/dioxus-rsx/latest/dioxus_rsx/trait.HotReloadingContext.html) trait to provide a mapping between the rust names of your elements/attributes and the resulting strings.

You can then provide the Context to the builder to make hot reloading work with your custom namespace:

```rust
fn main(){
    // Note: Use default instead of new if you are using a custom namespace
    hot_reload_init!(Config::<MyContext>::default());
    // launch your application
}
```

## Implementing Hot Reloading for a Custom Renderer

To add hot reloading support to your custom renderer you can use the connect function. This will connect to the dev server you just need to provide a way to transfer `Template`s to the `VirtualDom`. Once you implement this your users can use the hot_reload_init function just like any other render.

```rust
async fn launch(app: Component) {
    let mut vdom = VirtualDom::new(app);
    // ...

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    dioxus_hot_reload::connect(move |msg| {
        let _ = tx.send(msg);
    });

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg{
                    HotReloadMsg::Shutdown => {
                        // ... shutdown the application
                    }
                    HotReloadMsg::UpdateTemplate(template) => {
                        // update the template in the virtual dom
                        vdom.replace_template(template);
                    }
                }
            }
            _ = vdom.wait_for_work() => {
                // ...
            }
        }
        let mutations = vdom.render_immediate();
        // apply the mutations to the dom
    }
}
```

## Contributing

- Report issues on our [issue tracker](https://github.com/dioxuslabs/dioxus/issues).
- Join the discord and ask questions!

## License

This project is licensed under the [MIT license].

[mit license]: https://github.com/DioxusLabs/dioxus/blob/master/LICENSE-MIT

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Dioxus by you shall be licensed as MIT without any additional
terms or conditions.
