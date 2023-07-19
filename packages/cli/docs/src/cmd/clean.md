# Clean

`dx clean` will clear the build artifacts (the out_dir and the cargo cache)

```
dioxus-clean
Clean build artifacts

USAGE:
    dx clean [OPTIONS]

OPTIONS:
        --bin   [default: None]
```

# Example

```
dx clean
```

# Specify workspace bin
You can add the `--bin` option to select which crate you want Dioxus to clean artifacts from:

```
dioxus clean --bin app
```
