# dx-env

A crate that provides the environment variable names and accessors that `dx` uses to configure the applications it launches at runtime.

This crate was previously published as `dioxus-cli-config`.

This crate exists for us to very cleanly define the exact fields we want to pass down to Dioxus applications at runtime but without exposing the entire config object.

This leads to faster compile times, smaller binaries, and a clearer distinction between the config and the application.
