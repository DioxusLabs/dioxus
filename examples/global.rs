//! Example: README.md showcase
//!
//! The example from the README.md.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

static COUNT: GlobalSignal<i32> = Signal::global(|| 0);
static DOUBLED_COUNT: GlobalMemo<i32> = Signal::global_memo(|| COUNT() * 2);

fn app() -> Element {
    rsx! {
        h1 { "{COUNT} x 2 = {DOUBLED_COUNT}" }
        button { onclick: move |_| *COUNT.write() += 1, "Up high!" }
        button { onclick: move |_| *COUNT.write() -= 1, "Down low!" }
    }
}
