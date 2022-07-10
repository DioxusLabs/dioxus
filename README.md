<div align="center">
  <h1>📦✨ Dioxus CLI </h1>
  <p><strong>Tooling to supercharge dioxus projects</strong></p>
</div>

dioxus-cli (inspired by wasm-pack and webpack) is a tool to help get dioxus projects off the ground. It handles all the build, development, bundling, and publishing to make web development simple.

## Installation

```shell
# for stable
$ cargo install dioxus-cli
# for latest
$ cargo install --git https://github.com/DioxusLabs/cli
# for developing
$ cargo install --path . --debug
```

Now, `dioxus` is in your path, you can use `dioxus --help` to get the help document.

```
dioxus 0.1.2
Build, bundle, & ship your Dioxus app

USAGE:
    dioxus [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -v               Enable verbose logging
    -V, --version    Prints version information

SUBCOMMANDS:
    build        Build the Rust WASM app and all of its assets
    clean        Clean output artifacts
    config       Dioxus config file controls
    create       Init a new project for Dioxus
    help         Prints this message or the help of the given subcommand(s)
    serve        Build, watch & serve the Rust WASM app and all of its assets
    translate    Translate some source file into Dioxus code
```

## Get Started 

You can use the `dioxus create project_name` to init a dioxus project, its will be clone from the [dioxus-template](https://github.com/DioxusLabs/dioxus-template).

or you can assign the template path:

```
dioxus create hello --template gh:dioxuslabs/dioxus-template
```
