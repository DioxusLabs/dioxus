# dioxus-cli-config

This crate has been renamed to [`dx-env`](https://crates.io/crates/dx-env) and now only re-exports it. It will be removed in a future release.

`app_title` and `always_on_top` are Dioxus window settings rather than part of the `dx` environment contract, so they moved into the renderers that use them (`dioxus-desktop`, `dioxus-native` and `dioxus-liveview`). This crate keeps deprecated copies of them.
