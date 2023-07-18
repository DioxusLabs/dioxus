# Clean

`dioxus clean` will clear the build artifacts (the out_dir and the cargo cache)

```
dioxus-clean 
Clean build artifacts

USAGE:
    dioxus clean [OPTIONS]

OPTIONS:
        --bin   [default: None]
```

# Example

```
dioxus clean
```

# Specify workspace bin
You can add the `--bin` option to select which crate you want Dioxus to clean artifacts from:

```
dioxus clean --bin app
```