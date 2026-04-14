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

use dioxus::logger::tracing::{Level, debug, error, info, warn};
use dioxus::prelude::*;

fn main() {
    // `dioxus::logger::init` is optional and called automatically by `dioxus::launch`.
    // In development mode, the `Debug` tracing level is set, and in release only the `Info` level is set.
    // You can call it yourself manually in the cases you:
    //   - want to customize behavior
    //   - aren't using `dioxus::launch` (i.e. custom fullstack setups) but want the integration.
    // The Tracing crate is the logging interface that the dioxus-logger uses.
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
