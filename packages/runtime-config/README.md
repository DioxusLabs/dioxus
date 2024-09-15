# dioxus-runtime-config

A crate that provides key/value names and types for configuring Dioxus applications at runtime.

This crate exists for us to very cleanly define the exact fields we want to pass down to Dioxus applications at runtime but without exposing the entire config object.

This leads to faster compile times, smaller binaries, and a clearer distinction between the config and the application.
