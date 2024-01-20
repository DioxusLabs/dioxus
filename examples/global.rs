//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

static COUNT: GlobalSignal<i32> = Signal::global(|| 0);
static DOUBLED_COUNT: GlobalSelector<i32> = Signal::global_selector(|| COUNT() * 2);

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        h1 { "{COUNT} x 2 = {DOUBLED_COUNT}" }
        button { onclick: move |_| *COUNT.write() += 1, "Up high!" }
        button { onclick: move |_| *COUNT.write() -= 1, "Down low!" }
    }
}
