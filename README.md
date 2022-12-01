<div align="center">
  <h1>📦✨ Dioxus CLI </h1>
  <p><strong>Tooling to supercharge Dioxus projects</strong></p>
</div>
**dioxus-cli** (inspired by wasm-pack and webpack) is a tool for getting Dioxus projects up and running. It handles all of the build, bundling, development and publishing to simplify web development.


## Installation

### Install stable version
```
cargo install dioxus-cli
```
### Install from git repository
```
cargo install --git https://github.com/DioxusLabs/cli
```
### Install from local folder
```
cargo install --path . --debug
```


## Get Started 

Use `dioxus create project-name` to initialize a new Dioxus project. <br>

It will be cloned from the [dioxus-template](https://github.com/DioxusLabs/dioxus-template) repository.

<br>

Alternatively, you can specify the template path:

```
dioxus create hello --template gh:dioxuslabs/dioxus-template
```

## Dioxus Config File

Dioxus CLI will use `Dioxus.toml` file to Identify some project info and switch some cli feature.

You can get more configure information from [Dioxus CLI Document](https://dioxuslabs.com/cli/configure.html).