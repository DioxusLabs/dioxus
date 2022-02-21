# Serve

`dioxsu serve` can start a dev server (include hot-reload tool) to run the project.

```
dioxus-serve 
Build, watch & serve the Rust WASM app and all of its assets

USAGE:
    dioxus serve [OPTIONS]

OPTIONS:
        --example <EXAMPLE>      [default: ""]
        --platform <PLATFORM>    [default: "default_platform"]
        --release                [default: false]
```

you can use this command to build project and start a `dev server` :

```
dioxus serve
```

## Target platform

use option `platform` choose build target platform:

```
# for desktop project
dioxus serve --platform desktop
```

`platform` only supports `desktop` & `web`.

`dev-server` only for `web` project.

```
# for web project
dioxus serve --platform web
```

## Serve Example

you can use `--example {name}` to start a example code.

```
# build `example/test` code
dioxus serve --exmaple test
```