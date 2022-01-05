# Overview

In this chapter, we're going to get "set up" with a small desktop application.

We'll learn about:
- Installing the Rust programming language
- Installing the Dioxus CLI for bundling and developing
- Suggested cargo extensions


For platform-specific guides, check out the [Platform Specific Guides](../platforms/00-index.md).

# Setting up Dioxus

Dioxus requires a few main things to get up and running:

- The [Rust compiler](https://www.rust-lang.org) and associated build tooling

- An editor of your choice, ideally configured with the [Rust-Analyzer LSP plugin](https://rust-analyzer.github.io)

Dioxus integrates very well with the Rust-Analyzer IDE plugin which will provide appropriate syntax highlighting, code navigation, folding, and more.

### Installing Rust

Head over to [https://rust-lang.org](http://rust-lang.org) and install the Rust compiler. 

Once installed, make sure to  install wasm32-unknown-unknown as a target if you're planning on deploying your app to the web.

```
rustup target add wasm32-unknown-unknown
```

### Dioxus-CLI for dev server, bundling, etc.

We also recommend installing the Dioxus CLI. The Dioxus CLI automates building and packaging for various targets and integrates with simulators, development servers, and app deployment. To install the CLI, you'll need cargo (should be automatically installed with Rust):

```
$ cargo install dioxus-cli
```

You can update the dioxus-cli at any time with:

```
$ cargo install --force dioxus-cli
```

We provide this 1st-party tool to save you from having to run potentially untrusted code every time you add a crate to your project - as is standard in the NPM ecosystem.

### Suggested extensions

If you want to keep your traditional `npm install XXX` workflow for adding packages, you might want to install `cargo-edit` and a few other fun `cargo` extensions:

- [cargo edit](https://github.com/killercup/cargo-edit) for adding dependencies from the CLI
- [cargo-expand](https://github.com/dtolnay/cargo-expand) for expanding macro calls
- [cargo tree](https://doc.rust-lang.org/cargo/commands/cargo-tree.html) - an integrated cargo command that lets you inspect your dependency tree

That's it! We won't need to touch NPM/WebPack/Babel/Parcel, etc. However, you _can_ configure your app to use WebPack with [traditional WASM-pack tooling](https://rustwasm.github.io/wasm-pack/book/tutorials/hybrid-applications-with-webpack/using-your-library.html).

## Rust Knowledge

With Rust, things like benchmarking, testing, and documentation are included in the language. We strongly recommend going through the official Rust book _completely_. However, our hope is that a Dioxus app can serve as a great first Rust project. With Dioxus you'll learn about:

- Error handling
- Structs, Functions, Enums
- Closures
- Macros

We've put a lot of care into making Dioxus syntax familiar and easy to understand, so you won't need deep knowledge on async, lifetimes, or smart pointers until you really start building complex Dioxus apps.

We strongly encourage exploring the guides for more information on how to work with the integrated tooling:

- [Testing](Testing.md)
- [Documentation](Documentation.md)
- [Benchmarking](Benchmarking.md)
- [Building](Building.md)
- [Modules](Modules.md)
- [Crates](Crates.md)
