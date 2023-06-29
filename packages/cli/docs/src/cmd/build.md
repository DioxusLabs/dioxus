# Build

The `dioxus build` command can help you `pack & build` a dioxus project.

```
dioxus-build 
Build the Rust WASM app and all of its assets

USAGE:
    dioxus build [OPTIONS]

OPTIONS:
        --example <EXAMPLE>      [default: ""]
        --platform <PLATFORM>    [default: "default_platform"]
        --release                [default: false]
```

You can use this command to build a project:

```
dioxus build --release
```

## Target platform

Use the `platform` option to choose your target platform:

```
# for desktop project
dioxus build --platform desktop
```

`platform` currently only supports `desktop` & `web`.

```
# for web project
dioxus build --platform web
```

## Build Example

You can use the `example` option to select a example to build:

```
# build the `test` example
dioxus build --exmaple test
```