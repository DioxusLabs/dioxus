[package]
name = "dioxus-cli"
version = "0.3.1"
authors = ["Jonathan Kelley"]
edition = "2021"
description = "CLI tool for developing, testing, and publishing Dioxus apps"
license = "MIT/Apache-2.0"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]

# cli core
clap = { version = "3.0.14", features = ["derive"] }
thiserror = "1.0.30"
wasm-bindgen-cli-support = "0.2"
colored = "2.0.0"

# features
log = "0.4.14"
fern = { version = "0.6.0", features = ["colored"] }
serde = { version = "1.0.136", features = ["derive"] }
serde_json = "1.0.79"
toml = "0.5.8"
fs_extra = "1.2.0"
cargo_toml = "0.11.4"
futures = "0.3.21"
notify = { version = "5.0.0-pre.16", features = ["serde"] }
html_parser = "0.6.2"
binary-install = "0.0.2"
convert_case = "0.5.0"
cargo_metadata = "0.15.0"
tokio = { version = "1.16.1", features = ["full"] }
atty = "0.2.14"
regex = "1.5.4"
chrono = "0.4.19"
anyhow = "1.0.53"
hyper = "0.14.17"
hyper-rustls = "0.23.2"
indicatif = "0.17.0-rc.11"
subprocess = "0.2.9"

axum = { version = "0.5.1", features = ["ws", "headers"] }
tower-http = { version = "0.2.2", features = ["full"] }
headers = "0.3.7"

walkdir = "2"

# tools download
dirs = "4.0.0"
reqwest = { version = "0.11", features = [
    "rustls-tls",
    "stream",
    "trust-dns",
    "blocking",
] }
flate2 = "1.0.22"
tar = "0.4.38"
zip = "0.6.2"
tower = "0.4.12"

syn = { version = "1.0", features = ["full", "extra-traits"] }


proc-macro2 = { version = "1.0", features = ["span-locations"] }
lazy_static = "1.4.0"

# plugin packages
mlua = { version = "0.8.1", features = [
    "lua54",
    "vendored",
    "async",
    "send",
    "macros",
] }
ctrlc = "3.2.3"
# dioxus-rsx = "0.0.1"
gitignore = "1.0.7"

dioxus-rsx = { version = "0.0.3" }
dioxus-html = { version = "0.3", features = ["hot-reload-context"] }
dioxus-core = { version = "0.3", features = ["serialize"] }
dioxus-autofmt = "0.3.0"
rsx-rosetta = { version = "0.3" }
open = "4.1.0"

[[bin]]
path = "src/main.rs"

name = "dioxus"
