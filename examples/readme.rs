//! Example: README.md showcase
//!
//! The example from the README.md

use dioxus::prelude::*;
fn main() {
    dioxus::web::launch(App)
}

static App: FC<()> = |cx| {
    let mut count = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            h1 { "Hifive counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    })
};
