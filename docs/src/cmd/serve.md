# Serve

The `dioxus serve` can start a dev server (include hot-reload tool) to run the project.

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

You can use this command to build project and start a `dev server` :

```
dioxus serve
```

## Target platform

Use option `platform` choose build target platform:

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

You can use `--example {name}` to start a example code.

```
# build `example/test` code
dioxus serve --exmaple test
```

## Open Browser

You can add `--open` flag to open system default browser when server startup.

```
dioxus serve --open
```


## Cross Origin Policy

use `cross-origin-policy` can change corss-origin header in serverside.

```
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
```

```
dioxus serve --corss-origin-policy
```