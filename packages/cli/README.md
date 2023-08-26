<div style="text-align: center">
  <h1>📦✨ Dioxus CLI</h1>
  <p><strong>Tooling to supercharge Dioxus projects</strong></p>
</div>

**dioxus-cli** (inspired by wasm-pack and webpack) is a tool for getting Dioxus projects up and running.
It handles all build, bundling, development and publishing to simplify web development.

## Installation

### Install stable version
```
cargo install dioxus-cli
```
### Install from git repository
```
cargo install --git https://github.com/DioxusLabs/dioxus dioxus-cli
```
### Install from local folder
```
cargo install --path . --debug
```

## Get started

Use `dx create project-name` to initialize a new Dioxus project.
It will be cloned from the [dioxus-template](https://github.com/DioxusLabs/dioxus-template) repository.

Alternatively, you can specify the template path:

```
dx create hello --template gh:dioxuslabs/dioxus-template
```

## Dioxus config file

You can use the `Dioxus.toml` file for further configuration.
Some fields are mandatory, but the CLI tool will tell you which ones are missing.
You can create a `Dioxus.toml` with all fields already set using `dx config init project-name`,
or you can use this bare-bones template (only mandatory fields) to get started:
```toml
[application]
name = "project-name"
# Currently supported platforms: web, desktop
default_platform = "web"

[web.app]
title = "Hello"

[web.watcher]

[web.resource.dev]
```

The full anatomy of `Dioxus.toml` is shown on the [Dioxus website](https://dioxuslabs.com/learn/0.4/CLI/configure).
