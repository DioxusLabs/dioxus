//! Run code on the main thread.
//!
//! Dioxus desktop runs your components on a dedicated thread, separate from the main thread that
//! owns the OS event loop and every native window. Some platform and FFI APIs *must* be called
//! from the main thread. [`window().run_on_main_thread`] hops over to the main thread, runs your
//! closure there, and blocks until it hands the result back.

use dioxus::desktop::window;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let dom_thread = use_signal(|| format!("{:?}", std::thread::current().id()));
    let mut main_thread = use_signal(String::new);

    rsx! {
        h1 { "run_on_main_thread" }
        p { "Components run on the DOM thread: {dom_thread}" }
        button {
            onclick: move |_| {
                // This closure runs on the main thread. Put any main-thread-only FFI / platform
                // calls here — whatever it returns is sent back to the DOM thread.
                let id = window().run_on_main_thread(|| format!("{:?}", std::thread::current().id()));
                main_thread.set(id);
            },
            "Run a closure on the main thread"
        }
        if !main_thread.read().is_empty() {
            p { "The closure ran on the main thread: {main_thread}" }
        }
    }
}
