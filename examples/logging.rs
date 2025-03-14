//! Dioxus ships out-of-the-box with tracing hooks that integrate with the Dioxus-CLI.
//!
//! The built-in tracing-subscriber automatically sets up a wasm panic hook and wires up output
//! to be consumed in a machine-readable format when running under `dx`.
//!
//! You can disable the built-in tracing-subscriber or customize the log level yourself.
//!
//! By default:
//! - in `dev` mode, the default log output is `debug`
//! - in `release` mode, the default log output is `info`
//!
//! To use the dioxus logger in your app, simply call any of the tracing functions (info!(), warn!(), error!())

#[allow(unused_imports)]
use dioxus::logger::tracing::{debug, error, info, warn, Level};
use dioxus::prelude::*;

fn main() {
    dioxus::logger::init(Level::INFO).expect("Failed to initialize logger");
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Logger demo" }
            button {
                onclick: move |_| warn!("Here's a warning!"),
                "Warn!"
            }
            button {
                onclick: move |_| error!("Here's an error!"),
                "Error!"
            }
            button {
                onclick: move |_| {
                    debug!("Here's a debug");
                    warn!("The log level is set to info so there should not be a debug message")
                },
                "Debug!"
            }
            button {
                onclick: move |_| info!("Here's an info!"),
                "Info!"
            }
        }
    }
}
