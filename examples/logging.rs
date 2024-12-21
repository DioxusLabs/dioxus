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

use dioxus::logger::tracing::{debug, error, info, warn};
use dioxus::prelude::*;

fn main() {
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
                onclick: move |_| debug!("Here's a debug"),
                "Debug!"
            }
            button {
                onclick: move |_| info!("Here's an info!"),
                "Info!"
            }
        }
    }
}
