<div align="center">
  <h1>📦✨ Dioxus CLI </h1>
  <p><strong>Tooling to supercharge Dioxus projects</strong></p>
</div>

`dioxus-cli` (inspired by wasm-pack and webpack) is a tool for getting Dioxus projects up and running. It handles all of the build, bundling, development and publishing to simplify web development.

# Installation

## Stable
```
cargo install dioxus-cli
```
## Latest
```
cargo install --git https://github.com/DioxusLabs/cli
```
## Developing
```
cargo install --path . --debug
```

Now that `dioxus` is in the path, use `dioxus --help` to see the help information.

```
dioxus 0.1.2
Build, Bundle & Ship Dioxus Apps

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

Use `dioxus create project_name` to initialize a new Dioxus project. <br>

It will be cloned from the [dioxus-template](https://github.com/DioxusLabs/dioxus-template) repository.

<br>

Alternatively, you can specify the template path:

```
dioxus create hello --template gh:dioxuslabs/dioxus-template
```
