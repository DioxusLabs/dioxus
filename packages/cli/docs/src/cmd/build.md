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

You can use this command to build project to `out_dir` :

```
dioxus build --release
```

## Target platform

Use option `platform` choose build target platform:

```
# for desktop project
dioxus build --platform desktop
```

`platform` only supports `desktop` & `web`.

```
# for web project
dioxus build --platform web
```

## Build Example

You can use `--example {name}` to build a example code.

```
# build `example/test` code
dioxus build --exmaple test
```