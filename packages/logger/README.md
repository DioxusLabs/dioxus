<div align="center">
  <h1>📡 Dioxus Logger 🛰️</h1>
  <p><strong>A logging utility to provide a standard interface whether you're targeting web, desktop, fullstack, and more.</strong></p>
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus-logger">
    <img src="https://img.shields.io/crates/v/dioxus-logger.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus-logger">
    <img src="https://img.shields.io/crates/d/dioxus-logger.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus-logger">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

-----

`dioxus-logger` is a basic cross-platform facade for logging in [Dioxus](https://dioxuslabs.com/) that uses the [tracing](https://crates.io/crates/tracing) crate.


```rust
use dioxus::prelude::*;
use dioxus_logger::tracing::{Level, info};

fn main() {
  dioxus_logger::init(Level::INFO).expect("logger failed to init");
  dioxus::launch(App);
}

#[component]
fn App() -> Element {
  info!("App rendered");
  rsx! {
    p { "hi" }
  }
}
```

## Dioxus support

As of v0.6, dioxus_logger is part of dioxus itself. Dioxus will call `init` with a default Level, though you can still override the default with `init`.

```rust
use dioxus::prelude::*;
use dioxus::logger::tracing::{Level, info};

fn main() {
  dioxus::logger::init(Level::INFO).expect("logger failed to init");
  dioxus::launch(App);
}

#[component]
fn App() -> Element {
  info!("App rendered");
  rsx! {
    p { "hi" }
  }
}


```

## Platform Support
Dioxus logger will eventually support every target that Dioxus does. Currently mobile and TUI are not supported.

## Installation
`dioxus_logger` is part of Dioxus v0.6. If you're using Dioxus v0.6, then no installation is required!


If you're on Dioxus v0.5 and below, you can add `dioxus-logger` to your application by adding it to your dependencies.
```toml
[dependencies]
dioxus-logger = "0.5"
```


## License
This project is licensed under the [MIT license].

[mit license]: ./LICENSE

Every contribution intentionally submitted for inclusion in `dioxus-logger` by you, shall be licensed as MIT, without any additional terms or conditions.
