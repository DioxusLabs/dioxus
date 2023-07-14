# Serve

The `dx serve` can start a dev server with hot-reloading

```
dioxus-serve
Build, watch & serve the Rust WASM app and all of its assets

USAGE:
    dx serve [OPTIONS]

OPTIONS:
        --example <EXAMPLE>      [default: ""]
        --platform <PLATFORM>    [default: "default_platform"]
        --release                [default: false]
        --hot-reload             [default: false]ß
```

You can use this command to build project and start a dev server:

```
dx serve
```

## Serve Example

You can use the `example` option to serve a example:

```
# serve the `test` example
dx serve --exmaple test
```

## Open Browser

You can add the `--open` option to open system default browser when server startup:

```
dx serve --open
```

## RSX Hot Reloading

You can add the `--hot-reload` flag to enable [rsx hot reloading](https://dioxuslabs.com/docs/0.3/guide/en/getting_started/hot_reload.html). This will allow you to reload some rsx changes without a full recompile:

```
dx serve --open
```

## Cross Origin Policy

You can add the `cross-origin-policy` option to change cross-origin header to:

```
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
```

```
dx serve --corss-origin-policy
```
