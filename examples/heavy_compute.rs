//! This example shows that you can place heavy work on the main thread, and then
//!
//! You *should* be using `tokio::spawn_blocking` instead.
//!
//! Your app runs in an async runtime (Tokio), so you should avoid blocking
//! the rendering of the VirtualDom.
//!
//!

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app() -> Element {
    // This is discouraged
    std::thread::sleep(std::time::Duration::from_millis(2_000));

    // This is suggested
    tokio::task::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(2_000));
    });

    rsx! {
        div { "Hello, world!" }
    }
}
