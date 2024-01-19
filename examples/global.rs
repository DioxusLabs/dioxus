//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

static COUNT: GlobalSignal<i32> = Signal::global(|| 0);

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    rsx! {
        h1 { "High-Five counter: {COUNT}" }
        button { onclick: move |_| *COUNT.write() += 1, "Up high!" }
        button { onclick: move |_| *COUNT.write() -= 1, "Down low!" }
    }
}
